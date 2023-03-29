// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    monitor,
    network::QuorumStoreSender,
    quorum_store::{
        batch_generator::BatchGeneratorCommand, batch_store::BatchReader, counters, utils::Timeouts,
    },
};
use aptos_consensus_types::proof_of_store::{
    BatchInfo, ProofOfStore, SignedBatchInfo, SignedBatchInfoError,
};
use aptos_crypto::{bls12381, HashValue};
use aptos_logger::prelude::*;
use aptos_types::{
    aggregate_signature::PartialSignatures, validator_verifier::ValidatorVerifier, PeerId,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{mpsc::Receiver, oneshot as TokioOneshot},
    time,
};

#[derive(Debug)]
pub(crate) enum ProofCoordinatorCommand {
    AppendSignature(SignedBatchInfo),
    Shutdown(TokioOneshot::Sender<()>),
}

struct IncrementalProofState {
    info: BatchInfo,
    aggregated_signature: BTreeMap<PeerId, bls12381::Signature>,
    aggregated_voting_power: u128,
    completed: bool,
}

impl IncrementalProofState {
    fn new(info: BatchInfo) -> Self {
        Self {
            info,
            aggregated_signature: BTreeMap::new(),
            aggregated_voting_power: 0,
            completed: false,
        }
    }

    fn add_signature(
        &mut self,
        signed_batch_info: SignedBatchInfo,
        validator_verifier: &ValidatorVerifier,
    ) -> Result<(), SignedBatchInfoError> {
        if signed_batch_info.batch_info() != &self.info {
            return Err(SignedBatchInfoError::WrongInfo);
        }

        if self
            .aggregated_signature
            .contains_key(&signed_batch_info.signer())
        {
            return Err(SignedBatchInfoError::DuplicatedSignature);
        }

        match validator_verifier.get_voting_power(&signed_batch_info.signer()) {
            Some(voting_power) => {
                let signer = signed_batch_info.signer();
                if self
                    .aggregated_signature
                    .insert(signer, signed_batch_info.signature())
                    .is_none()
                {
                    self.aggregated_voting_power += voting_power as u128;
                } else {
                    error!(
                        "Author already in aggregated_signatures right after rechecking: {}",
                        signer
                    );
                }
            },
            None => {
                error!(
                    "Received signature from author not in validator set: {}",
                    signed_batch_info.signer()
                );
                return Err(SignedBatchInfoError::InvalidAuthor);
            },
        }

        Ok(())
    }

    fn ready(&self, validator_verifier: &ValidatorVerifier) -> bool {
        if self.aggregated_voting_power >= validator_verifier.quorum_voting_power() {
            let recheck = validator_verifier.check_voting_power(self.aggregated_signature.keys());
            if recheck.is_err() {
                error!("Unexpected discrepancy: aggregated_voting_power is {}, while rechecking we get {:?}", self.aggregated_voting_power, recheck);
            }
            recheck.is_ok()
        } else {
            false
        }
    }

    fn take(&mut self, validator_verifier: &ValidatorVerifier) -> ProofOfStore {
        if self.completed {
            panic!("Cannot call take twice, unexpected issue occurred");
        }
        self.completed = true;

        let proof = match validator_verifier
            .aggregate_signatures(&PartialSignatures::new(self.aggregated_signature.clone()))
        {
            Ok(sig) => ProofOfStore::new(self.info.clone(), sig),
            Err(e) => unreachable!("Cannot aggregate signatures on digest err = {:?}", e),
        };
        proof
    }
}

pub(crate) struct ProofCoordinator {
    peer_id: PeerId,
    proof_timeout_ms: usize,
    digest_to_proof: HashMap<HashValue, IncrementalProofState>,
    digest_to_time: HashMap<HashValue, u64>,
    // to record the batch creation time
    timeouts: Timeouts<BatchInfo>,
    batch_reader: Arc<dyn BatchReader>,
    batch_generator_cmd_tx: tokio::sync::mpsc::Sender<BatchGeneratorCommand>,
}

//PoQS builder object - gather signed digest to form PoQS
impl ProofCoordinator {
    pub fn new(
        proof_timeout_ms: usize,
        peer_id: PeerId,
        batch_reader: Arc<dyn BatchReader>,
        batch_generator_cmd_tx: tokio::sync::mpsc::Sender<BatchGeneratorCommand>,
    ) -> Self {
        Self {
            peer_id,
            proof_timeout_ms,
            digest_to_proof: HashMap::new(),
            digest_to_time: HashMap::new(),
            timeouts: Timeouts::new(),
            batch_reader,
            batch_generator_cmd_tx,
        }
    }

    fn init_proof(
        &mut self,
        signed_batch_info: &SignedBatchInfo,
    ) -> Result<(), SignedBatchInfoError> {
        // Check if the signed digest corresponding to our batch
        if signed_batch_info.author() != self.peer_id {
            return Err(SignedBatchInfoError::WrongAuthor);
        }
        let batch_author = self
            .batch_reader
            .exists(signed_batch_info.digest())
            .ok_or(SignedBatchInfoError::WrongAuthor)?;
        if batch_author != signed_batch_info.author() {
            return Err(SignedBatchInfoError::WrongAuthor);
        }

        self.timeouts.add(
            signed_batch_info.batch_info().clone(),
            self.proof_timeout_ms,
        );
        self.digest_to_proof.insert(
            *signed_batch_info.digest(),
            IncrementalProofState::new(signed_batch_info.batch_info().clone()),
        );
        self.digest_to_time
            .entry(*signed_batch_info.digest())
            .or_insert(chrono::Utc::now().naive_utc().timestamp_micros() as u64);
        Ok(())
    }

    fn add_signature(
        &mut self,
        signed_batch_info: SignedBatchInfo,
        validator_verifier: &ValidatorVerifier,
    ) -> Result<Option<ProofOfStore>, SignedBatchInfoError> {
        if !self
            .digest_to_proof
            .contains_key(signed_batch_info.digest())
        {
            self.init_proof(&signed_batch_info)?;
        }
        let digest = *signed_batch_info.digest();
        if let Some(value) = self.digest_to_proof.get_mut(signed_batch_info.digest()) {
            value.add_signature(signed_batch_info, validator_verifier)?;
            if !value.completed && value.ready(validator_verifier) {
                let proof = value.take(validator_verifier);
                // quorum store measurements
                let duration = chrono::Utc::now().naive_utc().timestamp_micros() as u64
                    - self
                        .digest_to_time
                        .remove(&digest)
                        .expect("Batch created without recording the time!");
                counters::BATCH_TO_POS_DURATION.observe_duration(Duration::from_micros(duration));
                return Ok(Some(proof));
            }
        }
        Ok(None)
    }

    async fn expire(&mut self) {
        let mut batch_ids = vec![];
        for signed_batch_info_info in self.timeouts.expire() {
            if let Some(state) = self.digest_to_proof.remove(signed_batch_info_info.digest()) {
                counters::BATCH_RECEIVED_REPLIES_COUNT
                    .observe(state.aggregated_signature.len() as f64);
                counters::BATCH_RECEIVED_REPLIES_VOTING_POWER
                    .observe(state.aggregated_voting_power as f64);
                counters::BATCH_SUCCESSFUL_CREATION
                    .observe(if state.completed { 1.0 } else { 0.0 });
                if !state.completed {
                    counters::TIMEOUT_BATCHES_COUNT.inc();
                    batch_ids.push(signed_batch_info_info.batch_id());
                }
            }
        }
        if self
            .batch_generator_cmd_tx
            .send(BatchGeneratorCommand::ProofExpiration(batch_ids))
            .await
            .is_err()
        {
            warn!("Failed to send proof expiration to batch generator");
        }
    }

    pub async fn start(
        mut self,
        mut rx: Receiver<ProofCoordinatorCommand>,
        mut network_sender: impl QuorumStoreSender,
        validator_verifier: ValidatorVerifier,
    ) {
        let mut interval = time::interval(Duration::from_millis(100));
        loop {
            tokio::select! {
                Some(command) = rx.recv() => monitor!("proof_coordinator_handle_command", {
                    match command {
                        ProofCoordinatorCommand::Shutdown(ack_tx) => {
                            ack_tx
                                .send(())
                                .expect("Failed to send shutdown ack to QuorumStore");
                            break;
                        },
                        ProofCoordinatorCommand::AppendSignature(signed_batch_info) => {
                            let peer_id = signed_batch_info.signer();
                            let digest = *signed_batch_info.digest();
                            match self.add_signature(signed_batch_info, &validator_verifier) {
                                Ok(result) => {
                                    if let Some(proof) = result {
                                        debug!("QS: received quorum of signatures, digest {}", digest);
                                        network_sender.broadcast_proof_of_store(proof).await;
                                    }
                                },
                                Err(e) => {
                                    // TODO: better error messages
                                    // Can happen if we already garbage collected
                                    if peer_id == self.peer_id {
                                        debug!("QS: could not add signature from self, err = {:?}", e);
                                    }
                                },
                            }
                        },
                    }
                }),
                _ = interval.tick() => {
                    monitor!("proof_coordinator_handle_tick", self.expire().await);
                }
            }
        }
    }
}
