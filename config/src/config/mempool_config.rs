// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
    node_config_loader::NodeType, Error, NodeConfig, MAX_APPLICATION_MESSAGE_SIZE,
};
use aptos_global_constants::DEFAULT_BUCKETS;
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MempoolConfig {
    /// Maximum number of transactions allowed in the Mempool
    pub capacity: usize,
    /// Maximum number of bytes allowed in the Mempool
    pub capacity_bytes: usize,
    /// Maximum number of transactions allowed in the Mempool per user
    pub capacity_per_user: usize,
    /// Number of failover peers to broadcast to when the primary network is alive
    pub default_failovers: usize,
    /// The maximum number of broadcasts sent to a single peer that are pending a response ACK at any point.
    pub max_broadcasts_per_peer: usize,
    /// Maximum number of inbound network messages to the Mempool application
    pub max_network_channel_size: usize,
    /// The interval to take a snapshot of the mempool to logs, only used when trace logging is enabled
    pub mempool_snapshot_interval_secs: u64,
    /// The maximum amount of time to wait for an ACK of Mempool submission to an upstream node.
    pub shared_mempool_ack_timeout_ms: u64,
    /// The amount of time to backoff between retries of Mempool submission to an upstream node.
    pub shared_mempool_backoff_interval_ms: u64,
    /// Maximum number of transactions to batch for a Mempool submission to an upstream node.
    pub shared_mempool_batch_size: usize,
    /// Maximum number of bytes to batch for a Mempool submission to an upstream node.
    pub shared_mempool_max_batch_bytes: u64,
    /// Maximum Mempool inbound message workers.  Controls concurrency of Mempool consumption.
    pub shared_mempool_max_concurrent_inbound_syncs: usize,
    /// Interval to broadcast to upstream nodes.
    pub shared_mempool_tick_interval_ms: u64,
    pub shared_mempool_peer_update_interval_ms: u64,
    /// Number of seconds until the transaction will be removed from the Mempool ignoring if the transaction has expired.
    ///
    /// This ensures that the Mempool isn't just full of non-expiring transactions that are way off into the future.
    pub system_transaction_timeout_secs: u64,
    /// Interval to garbage collect and remove transactions that have expired from the Mempool.
    pub system_transaction_gc_interval_ms: u64,
    /// Gas unit price buckets for broadcasting to upstream nodes.
    ///
    /// Overriding this won't make much of a difference if the upstream nodes don't match.
    pub broadcast_buckets: Vec<u64>,
    pub eager_expire_threshold_ms: Option<u64>,
    pub eager_expire_time_ms: u64,
}

impl Default for MempoolConfig {
    fn default() -> MempoolConfig {
        MempoolConfig {
            shared_mempool_tick_interval_ms: 50,
            shared_mempool_backoff_interval_ms: 30_000,
            shared_mempool_batch_size: 200,
            shared_mempool_max_batch_bytes: MAX_APPLICATION_MESSAGE_SIZE as u64,
            shared_mempool_ack_timeout_ms: 2_000,
            shared_mempool_max_concurrent_inbound_syncs: 4,
            max_broadcasts_per_peer: 2,
            max_network_channel_size: 1024,
            mempool_snapshot_interval_secs: 180,
            capacity: 2_000_000,
            capacity_bytes: 2 * 1024 * 1024 * 1024,
            capacity_per_user: 100,
            default_failovers: 1,
            shared_mempool_peer_update_interval_ms: 1_000,
            system_transaction_timeout_secs: 600,
            system_transaction_gc_interval_ms: 60_000,
            broadcast_buckets: DEFAULT_BUCKETS.to_vec(),
            eager_expire_threshold_ms: Some(10_000),
            eager_expire_time_ms: 3_000,
        }
    }
}

impl ConfigSanitizer for MempoolConfig {
    fn sanitize(
        _node_config: &mut NodeConfig,
        _node_type: NodeType,
        _chain_id: ChainId,
    ) -> Result<(), Error> {
        Ok(()) // TODO: add reasonable verifications
    }
}

impl ConfigOptimizer for MempoolConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        node_type: NodeType,
        _chain_id: ChainId,
    ) -> Result<bool, Error> {
        let mempool_config = &mut node_config.mempool;
        let local_mempool_config_yaml = &local_config_yaml["mempool"];

        // Change the default configs for VFNs
        let mut modified_config = false;
        if node_type.is_validator_fullnode() {
            // Set the shared_mempool_max_concurrent_inbound_syncs to 16 (default is 4)
            if local_mempool_config_yaml["shared_mempool_max_concurrent_inbound_syncs"].is_null() {
                mempool_config.shared_mempool_max_concurrent_inbound_syncs = 16;
                modified_config = true;
            }

            // Set the max_broadcasts_per_peer to 4 (default is 2)
            if local_mempool_config_yaml["max_broadcasts_per_peer"].is_null() {
                mempool_config.max_broadcasts_per_peer = 4;
                modified_config = true;
            }

            // Set the default_failovers to 0 (default is 1)
            if local_mempool_config_yaml["default_failovers"].is_null() {
                mempool_config.default_failovers = 0;
                modified_config = true;
            }

            // Set the shared_mempool_tick_interval_ms to 10 (default is 50)
            if local_mempool_config_yaml["shared_mempool_tick_interval_ms"].is_null() {
                mempool_config.shared_mempool_tick_interval_ms = 10;
                modified_config = true;
            }
        }

        Ok(modified_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_vfn_configs() {
        // Create the default VFN config
        let mut node_config = NodeConfig::get_default_vfn_config();

        // Optimize the config and verify modifications are made
        let modified_config = MempoolConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::ValidatorFullnode,
            ChainId::testnet(),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that all relevant fields are modified
        let mempool_config = &node_config.mempool;
        assert_eq!(
            mempool_config.shared_mempool_max_concurrent_inbound_syncs,
            16
        );
        assert_eq!(mempool_config.max_broadcasts_per_peer, 4);
        assert_eq!(mempool_config.default_failovers, 0);
        assert_eq!(mempool_config.shared_mempool_batch_size, 200);
        assert_eq!(mempool_config.shared_mempool_tick_interval_ms, 10);
    }

    #[test]
    fn test_optimize_validator_config() {
        // Create the default validator config
        let mut node_config = NodeConfig::get_default_validator_config();

        // Optimize the config and verify no modifications are made
        let modified_config = MempoolConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            ChainId::mainnet(),
        )
        .unwrap();
        assert!(!modified_config);

        // Verify that all relevant fields are not modified
        let mempool_config = &node_config.mempool;
        let default_mempool_config = MempoolConfig::default();
        assert_eq!(
            mempool_config.shared_mempool_max_concurrent_inbound_syncs,
            default_mempool_config.shared_mempool_max_concurrent_inbound_syncs
        );
        assert_eq!(
            mempool_config.max_broadcasts_per_peer,
            default_mempool_config.max_broadcasts_per_peer
        );
        assert_eq!(
            mempool_config.default_failovers,
            default_mempool_config.default_failovers
        );
        assert_eq!(
            mempool_config.shared_mempool_batch_size,
            default_mempool_config.shared_mempool_batch_size
        );
        assert_eq!(
            mempool_config.shared_mempool_tick_interval_ms,
            default_mempool_config.shared_mempool_tick_interval_ms
        );
    }

    #[test]
    fn test_optimize_vfn_config_no_overrides() {
        // Create the default validator config
        let mut node_config = NodeConfig::get_default_vfn_config();

        // Create a local config YAML with some local overrides
        let local_config_yaml = serde_yaml::from_str(
            r#"
            mempool:
              shared_mempool_max_concurrent_inbound_syncs: 4
              max_broadcasts_per_peer: 1
            "#,
        )
        .unwrap();

        // Optimize the config and verify modifications are made
        let modified_config = MempoolConfig::optimize(
            &mut node_config,
            &local_config_yaml,
            NodeType::ValidatorFullnode,
            ChainId::mainnet(),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that only the relevant fields are modified
        let mempool_config = &node_config.mempool;
        let default_mempool_config = MempoolConfig::default();
        assert_eq!(
            mempool_config.shared_mempool_max_concurrent_inbound_syncs,
            4
        );
        assert_eq!(mempool_config.max_broadcasts_per_peer, 2);
        assert_ne!(
            mempool_config.default_failovers,
            default_mempool_config.default_failovers
        );
        assert_ne!(
            mempool_config.shared_mempool_tick_interval_ms,
            default_mempool_config.shared_mempool_tick_interval_ms
        );
    }
}
