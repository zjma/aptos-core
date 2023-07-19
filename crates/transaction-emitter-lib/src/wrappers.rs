// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    args::{ClusterArgs, EmitArgs},
    cluster::Cluster,
    emitter::{stats::TxnStats, EmitJobMode, EmitJobRequest, TxnEmitter},
    instance::Instance,
};
use anyhow::{bail, Context, Result};
use aptos_logger::{error, info};
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_transaction_generator_lib::args::TransactionTypeArg;
use rand::{rngs::StdRng, SeedableRng};
use std::time::{Duration, Instant};

pub async fn emit_transactions(
    cluster_args: &ClusterArgs,
    emit_args: &EmitArgs,
) -> Result<TxnStats> {
    if emit_args.coordination_delay_between_instances.is_none() {
        let cluster = Cluster::try_from_cluster_args(cluster_args)
            .await
            .context("Failed to build cluster")?;
        emit_transactions_with_cluster(&cluster, emit_args, cluster_args.reuse_accounts).await
    } else {
        let initial_delay_after_minting = emit_args.coordination_delay_between_instances.unwrap();
        let start_time = Instant::now();
        let mut i = 0;
        loop {
            let cur_emit_args = if i > 0 {
                let mut cur_emit_args = emit_args.clone();
                cur_emit_args.coordination_delay_between_instances =
                    initial_delay_after_minting.checked_sub(start_time.elapsed().as_secs());
                if cur_emit_args.coordination_delay_between_instances.is_none() {
                    bail!("txn_emitter couldn't succeed after {} runs", i);
                }
                info!(
                    "Reduced coordination_delay_between_instances to {} for run {}",
                    cur_emit_args.coordination_delay_between_instances.unwrap(),
                    i
                );
                cur_emit_args
            } else {
                emit_args.clone()
            };

            let cluster = Cluster::try_from_cluster_args(cluster_args)
                .await
                .context("Failed to build cluster")?;

            let result = emit_transactions_with_cluster(
                &cluster,
                &cur_emit_args,
                cluster_args.reuse_accounts,
            )
            .await;
            match result {
                Ok(value) => return Ok(value),
                Err(e) => {
                    error!("Couldn't run txn emitter: {:?}, retrying", e)
                },
            }
            i += 1;
        }
    }
}

pub async fn emit_transactions_with_cluster(
    cluster: &Cluster,
    args: &EmitArgs,
    reuse_accounts: bool,
) -> Result<TxnStats> {
    let emitter_mode = EmitJobMode::create(args.mempool_backlog, args.target_tps);

    let duration = Duration::from_secs(args.duration);
    let client = cluster.random_instance().rest_client();
    let mut coin_source_account = cluster.load_coin_source_account(&client).await?;
    let emitter = TxnEmitter::new(
        TransactionFactory::new(cluster.chain_id)
            .with_transaction_expiration_time(args.txn_expiration_time_secs)
            .with_gas_unit_price(aptos_global_constants::GAS_UNIT_PRICE),
        StdRng::from_entropy(),
    );

    let transaction_mix_per_phase = TransactionTypeArg::args_to_transaction_mix_per_phase(
        &args.transaction_type,
        &args.transaction_weights,
        &args.transaction_phases,
        args.module_working_set_size.unwrap_or(1),
        args.sender_use_account_pool.unwrap_or(false),
    );
    let mut emit_job_request =
        EmitJobRequest::new(cluster.all_instances().map(Instance::rest_client).collect())
            .mode(emitter_mode)
            .transaction_mix_per_phase(transaction_mix_per_phase)
            .txn_expiration_time_secs(args.txn_expiration_time_secs)
            .coordination_delay_between_instances(Duration::from_secs(
                args.coordination_delay_between_instances.unwrap_or(0),
            ));
    if reuse_accounts {
        emit_job_request = emit_job_request.reuse_accounts();
    }
    if let Some(max_transactions_per_account) = args.max_transactions_per_account {
        emit_job_request =
            emit_job_request.max_transactions_per_account(max_transactions_per_account);
    }

    if let Some(gas_price) = args.gas_price {
        emit_job_request = emit_job_request.gas_price(gas_price);
    }

    if let Some(max_gas_per_txn) = args.max_gas_per_txn {
        emit_job_request = emit_job_request.max_gas_per_txn(max_gas_per_txn);
    }

    if let Some(init_gas_price_multiplier) = args.init_gas_price_multiplier {
        emit_job_request = emit_job_request.init_gas_price_multiplier(init_gas_price_multiplier);
    }

    if let Some(expected_max_txns) = args.expected_max_txns {
        emit_job_request = emit_job_request.expected_max_txns(expected_max_txns);
    }
    if let Some(expected_gas_per_txn) = args.expected_gas_per_txn {
        emit_job_request = emit_job_request.expected_gas_per_txn(expected_gas_per_txn);
    }
    if !cluster.coin_source_is_root {
        emit_job_request = emit_job_request.prompt_before_spending();
    }

    let stats = emitter
        .emit_txn_for_with_stats(
            &mut coin_source_account,
            emit_job_request,
            duration,
            (args.duration / 10).clamp(1, 10),
        )
        .await?;
    Ok(stats)
}
