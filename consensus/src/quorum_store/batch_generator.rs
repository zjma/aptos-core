// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    monitor,
    network::{NetworkSender, QuorumStoreSender},
    quorum_store::{
        counters,
        quorum_store_db::QuorumStoreStorage,
        types::Batch,
        utils::{MempoolProxy, TimeExpirations},
    },
};
use aptos_config::config::QuorumStoreConfig;
use aptos_consensus_types::{common::TransactionSummary, proof_of_store::BatchId};
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_types::PeerId;
use futures_channel::mpsc::Sender;
use rand::{thread_rng, RngCore};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::Interval;

#[derive(Debug)]
pub enum BatchGeneratorCommand {
    CommitNotification(u64),
    ProofExpiration(Vec<BatchId>),
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct BackPressure {
    pub txn_count: bool,
    pub proof_count: bool,
}

pub struct BatchGenerator {
    epoch: u64,
    my_peer_id: PeerId,
    batch_id: BatchId,
    db: Arc<dyn QuorumStoreStorage>,
    config: QuorumStoreConfig,
    mempool_proxy: MempoolProxy,
    batches_in_progress: HashMap<BatchId, Vec<TransactionSummary>>,
    batch_expirations: TimeExpirations<BatchId>,
    latest_block_timestamp: u64,
    last_end_batch_time: Instant,
    // quorum store back pressure, get updated from proof manager
    back_pressure: BackPressure,
}

impl BatchGenerator {
    pub(crate) fn new(
        epoch: u64,
        my_peer_id: PeerId,
        config: QuorumStoreConfig,
        db: Arc<dyn QuorumStoreStorage>,
        mempool_tx: Sender<QuorumStoreRequest>,
        mempool_txn_pull_timeout_ms: u64,
    ) -> Self {
        let batch_id = if let Some(mut id) = db
            .clean_and_get_batch_id(epoch)
            .expect("Could not read from db")
        {
            // If the node shut down mid-batch, then this increment is needed
            id.increment();
            id
        } else {
            BatchId::new(thread_rng().next_u64())
        };
        debug!("Initialized with batch_id of {}", batch_id);
        let mut incremented_batch_id = batch_id;
        incremented_batch_id.increment();
        db.save_batch_id(epoch, incremented_batch_id)
            .expect("Could not save to db");

        Self {
            epoch,
            my_peer_id,
            batch_id,
            db,
            config,
            mempool_proxy: MempoolProxy::new(mempool_tx, mempool_txn_pull_timeout_ms),
            batches_in_progress: HashMap::new(),
            batch_expirations: TimeExpirations::new(),
            latest_block_timestamp: 0,
            last_end_batch_time: Instant::now(),
            back_pressure: BackPressure {
                txn_count: false,
                proof_count: false,
            },
        }
    }

    pub(crate) async fn handle_scheduled_pull(&mut self, max_count: u64) -> Option<Batch> {
        // TODO: as an optimization, we could filter out the txns that have expired

        let exclude_txns: Vec<_> = self
            .batches_in_progress
            .values()
            .flatten()
            .cloned()
            .collect();

        trace!("QS: excluding txs len: {:?}", exclude_txns.len());
        let pulled_txns = self
            .mempool_proxy
            .pull_internal(
                max_count,
                self.config.mempool_txn_pull_max_bytes,
                true,
                exclude_txns,
            )
            .await
            .unwrap_or_default();

        trace!("QS: pulled_txns len: {:?}", pulled_txns.len());

        if pulled_txns.is_empty() {
            counters::PULLED_EMPTY_TXNS_COUNT.inc();
            // Quorum store metrics
            counters::CREATED_EMPTY_BATCHES_COUNT.inc();

            let duration = self.last_end_batch_time.elapsed().as_secs_f64();
            counters::EMPTY_BATCH_CREATION_DURATION
                .observe_duration(Duration::from_secs_f64(duration));

            self.last_end_batch_time = Instant::now();
            return None;
        } else {
            counters::PULLED_TXNS_COUNT.inc();
            counters::PULLED_TXNS_NUM.observe(pulled_txns.len() as f64);
        }

        // Quorum store metrics
        counters::CREATED_BATCHES_COUNT.inc();

        let duration = self.last_end_batch_time.elapsed().as_secs_f64();
        counters::BATCH_CREATION_DURATION.observe_duration(Duration::from_secs_f64(duration));

        counters::NUM_TXN_PER_BATCH.observe(pulled_txns.len() as f64);

        let batch_id = self.batch_id;
        self.batch_id.increment();
        self.db
            .save_batch_id(self.epoch, self.batch_id)
            .expect("Could not save to db");

        let expiry_time = aptos_infallible::duration_since_epoch().as_micros() as u64
            + self.config.batch_expiry_gap_when_init_usecs;
        let txn_summaries: Vec<_> = pulled_txns
            .iter()
            .map(|txn| TransactionSummary {
                sender: txn.sender(),
                sequence_number: txn.sequence_number(),
            })
            .collect();

        let batch = Batch::new(
            batch_id,
            pulled_txns,
            self.epoch,
            expiry_time,
            self.my_peer_id,
        );

        self.batches_in_progress.insert(batch_id, txn_summaries);
        self.batch_expirations.add_item(batch_id, expiry_time);

        self.last_end_batch_time = Instant::now();
        Some(batch)
    }

    pub async fn start(
        mut self,
        mut network_sender: NetworkSender,
        mut cmd_rx: tokio::sync::mpsc::Receiver<BatchGeneratorCommand>,
        mut back_pressure_rx: tokio::sync::mpsc::Receiver<BackPressure>,
        mut interval: Interval,
    ) {
        let start = Instant::now();

        let mut last_non_empty_pull = start;
        let back_pressure_decrease_duration =
            Duration::from_millis(self.config.back_pressure.decrease_duration_ms);
        let back_pressure_increase_duration =
            Duration::from_millis(self.config.back_pressure.increase_duration_ms);
        let mut back_pressure_decrease_latest = start;
        let mut back_pressure_increase_latest = start;
        let mut dynamic_pull_txn_per_s = (self.config.back_pressure.dynamic_min_txn_per_s
            + self.config.back_pressure.dynamic_max_txn_per_s)
            / 2;

        loop {
            let _timer = counters::WRAPPER_MAIN_LOOP.start_timer();

            tokio::select! {
                biased;
                Some(updated_back_pressure) = back_pressure_rx.recv() => {
                    self.back_pressure = updated_back_pressure;
                },
                _ = interval.tick() => monitor!("batch_generator_handle_tick", {

                    let now = Instant::now();
                    // TODO: refactor back_pressure logic into its own function
                    if self.back_pressure.txn_count {
                        // multiplicative decrease, every second
                        if back_pressure_decrease_latest.elapsed() >= back_pressure_decrease_duration {
                            back_pressure_decrease_latest = now;
                            dynamic_pull_txn_per_s = std::cmp::max(
                                (dynamic_pull_txn_per_s as f64 * self.config.back_pressure.decrease_fraction) as u64,
                                self.config.back_pressure.dynamic_min_txn_per_s,
                            );
                            trace!("QS: dynamic_max_pull_txn_per_s: {}", dynamic_pull_txn_per_s);
                        }
                        counters::QS_BACKPRESSURE_TXN_COUNT.observe(1.0);
                        counters::QS_BACKPRESSURE_DYNAMIC_MAX.observe(dynamic_pull_txn_per_s as f64);
                    } else {
                        // additive increase, every second
                        if back_pressure_increase_latest.elapsed() >= back_pressure_increase_duration {
                            back_pressure_increase_latest = now;
                            dynamic_pull_txn_per_s = std::cmp::min(
                                dynamic_pull_txn_per_s + self.config.back_pressure.dynamic_min_txn_per_s,
                                self.config.back_pressure.dynamic_max_txn_per_s,
                            );
                            trace!("QS: dynamic_max_pull_txn_per_s: {}", dynamic_pull_txn_per_s);
                        }
                        counters::QS_BACKPRESSURE_TXN_COUNT.observe(0.0);
                        counters::QS_BACKPRESSURE_DYNAMIC_MAX.observe(dynamic_pull_txn_per_s as f64);
                    }
                    if self.back_pressure.proof_count {
                        counters::QS_BACKPRESSURE_PROOF_COUNT.observe(1.0);
                    } else {
                        counters::QS_BACKPRESSURE_PROOF_COUNT.observe(0.0);
                    }
                    let since_last_non_empty_pull_ms = std::cmp::min(
                        now.duration_since(last_non_empty_pull).as_millis(),
                        self.config.batch_generation_max_interval_ms as u128
                    ) as usize;
                    if (!self.back_pressure.proof_count
                        && since_last_non_empty_pull_ms >= self.config.batch_generation_min_non_empty_interval_ms)
                        || since_last_non_empty_pull_ms == self.config.batch_generation_max_interval_ms {

                        let dynamic_pull_max_txn = std::cmp::max(
                            (since_last_non_empty_pull_ms as f64 / 1000.0 * dynamic_pull_txn_per_s as f64) as u64, 1);
                        if let Some(batch) = self.handle_scheduled_pull(dynamic_pull_max_txn).await {
                            last_non_empty_pull = now;
                            network_sender.broadcast_batch_msg(batch).await;
                        }
                    }
                }),
                Some(cmd) = cmd_rx.recv() => monitor!("batch_generator_handle_command", {
                    match cmd {
                        BatchGeneratorCommand::CommitNotification(block_timestamp) => {
                            trace!(
                                "QS: got clean request from execution, block timestamp {}",
                                block_timestamp
                            );
                            assert!(
                                self.latest_block_timestamp <= block_timestamp,
                                "Decreasing block timestamp"
                            );
                            self.latest_block_timestamp = block_timestamp;
                            // Cleans up all batches that expire in timestamp <= block_timestamp. This is
                            // safe since clean request must occur only after execution result is certified.
                            for batch_id in self.batch_expirations.expire(block_timestamp) {
                                if self.batches_in_progress.remove(&batch_id).is_some() {
                                    debug!(
                                        "QS: logical time based expiration batch w. id {} from batches_in_progress, new size {}",
                                        batch_id,
                                        self.batches_in_progress.len(),
                                    );
                                }
                            }
                        },
                        BatchGeneratorCommand::ProofExpiration(batch_ids) => {
                            for batch_id in batch_ids {
                                debug!(
                                    "QS: received timeout for proof of store, batch id = {}",
                                    batch_id
                                );
                                // Not able to gather the proof, allow transactions to be polled again.
                                self.batches_in_progress.remove(&batch_id);
                            }
                        }
                        BatchGeneratorCommand::Shutdown(ack_tx) => {
                            ack_tx
                                .send(())
                                .expect("Failed to send shutdown ack");
                            break;
                        },
                    }
                })
            }
        }
    }
}
