// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    db_options::{gen_state_kv_cfds, state_kv_db_column_families},
    utils::truncation_helper::{get_state_kv_commit_progress, truncate_state_kv_db_shards},
    COMMIT_POOL, NUM_STATE_SHARDS,
};
use anyhow::Result;
use aptos_config::config::{RocksdbConfig, RocksdbConfigs};
use aptos_logger::prelude::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_types::transaction::Version;
use arr_macro::arr;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub const STATE_KV_DB_FOLDER_NAME: &str = "state_kv_db";
pub const STATE_KV_METADATA_DB_NAME: &str = "state_kv_metadata_db";

pub struct StateKvDb {
    state_kv_metadata_db: Arc<DB>,
    state_kv_db_shards: [Arc<DB>; NUM_STATE_SHARDS],
}

impl StateKvDb {
    // TODO(grao): Support more flexible path to make it easier for people to put different shards
    // on different disks.
    pub(crate) fn new<P: AsRef<Path>>(
        db_root_path: P,
        rocksdb_configs: RocksdbConfigs,
        readonly: bool,
        ledger_db: Arc<DB>,
    ) -> Result<Self> {
        if !rocksdb_configs.use_state_kv_db {
            info!("State K/V DB is not enabled!");
            return Ok(Self {
                state_kv_metadata_db: Arc::clone(&ledger_db),
                state_kv_db_shards: arr![Arc::clone(&ledger_db); 16],
            });
        }

        let state_kv_db_config = rocksdb_configs.state_kv_db_config;
        let state_kv_metadata_db_path = db_root_path
            .as_ref()
            .join(STATE_KV_DB_FOLDER_NAME)
            .join("metadata");

        let state_kv_metadata_db = Arc::new(Self::open_db(
            state_kv_metadata_db_path.clone(),
            STATE_KV_METADATA_DB_NAME,
            &state_kv_db_config,
            readonly,
        )?);

        info!(
            state_kv_metadata_db_path = state_kv_metadata_db_path,
            "Opened state kv metadata db!"
        );

        // TODO(grao): Support sharding here.
        let sharding = false;
        let state_kv_db_shards = {
            if sharding {
                let mut shard_id: usize = 0;
                arr![{
                    let db = Self::open_shard(db_root_path.as_ref(), shard_id as u8, &state_kv_db_config, readonly)?;
                    shard_id += 1;
                    Arc::new(db)
                }; 16]
            } else {
                arr![Arc::clone(&state_kv_metadata_db); 16]
            }
        };

        let state_kv_db = Self {
            state_kv_metadata_db,
            state_kv_db_shards,
        };

        if let Some(overall_kv_commit_progress) = get_state_kv_commit_progress(&state_kv_db)? {
            truncate_state_kv_db_shards(&state_kv_db, overall_kv_commit_progress, None)?;
        }

        Ok(state_kv_db)
    }

    // TODO(grao): Remove this function.
    pub(crate) fn commit_nonsharded(
        &self,
        version: Version,
        state_kv_batch: SchemaBatch,
    ) -> Result<()> {
        state_kv_batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvCommitProgress,
            &DbMetadataValue::Version(version),
        )?;

        self.commit_raw_batch(state_kv_batch)
    }

    pub(crate) fn commit(
        &self,
        version: Version,
        sharded_state_kv_batches: [SchemaBatch; NUM_STATE_SHARDS],
    ) -> Result<()> {
        COMMIT_POOL.scope(|s| {
            let mut batches = sharded_state_kv_batches.into_iter();
            for shard_id in 0..NUM_STATE_SHARDS {
                let state_kv_batch = batches.next().unwrap();
                s.spawn(move |_| {
                    // TODO(grao): Consider propagating the error instead of panic, if necessary.
                    self.commit_single_shard(version, shard_id as u8, state_kv_batch)
                        .unwrap_or_else(|_| panic!("Failed to commit shard {shard_id}."));
                });
            }
        });

        self.write_progress(version)
    }

    pub(crate) fn commit_raw_batch(&self, state_kv_batch: SchemaBatch) -> Result<()> {
        // TODO(grao): Support sharding here.
        self.state_kv_metadata_db.write_schemas(state_kv_batch)
    }

    pub(crate) fn write_progress(&self, version: Version) -> Result<()> {
        self.state_kv_metadata_db.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvCommitProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn metadata_db(&self) -> &DB {
        &self.state_kv_metadata_db
    }

    pub(crate) fn db_shard(&self, shard_id: u8) -> &DB {
        &self.state_kv_db_shards[shard_id as usize]
    }

    pub(crate) fn commit_single_shard(
        &self,
        version: Version,
        shard_id: u8,
        batch: SchemaBatch,
    ) -> Result<()> {
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvShardCommitProgress(shard_id as usize),
            &DbMetadataValue::Version(version),
        )?;
        self.state_kv_db_shards[shard_id as usize].write_schemas(batch)
    }

    fn open_shard<P: AsRef<Path>>(
        db_root_path: P,
        shard_id: u8,
        state_kv_db_config: &RocksdbConfig,
        readonly: bool,
    ) -> Result<DB> {
        let shard_name = format!("shard_{}", shard_id);
        let db_name = format!("state_kv_db_shard_{}", shard_id);
        let path = db_root_path
            .as_ref()
            .join(STATE_KV_DB_FOLDER_NAME)
            .join(Path::new(&shard_name));
        Self::open_db(path, &db_name, state_kv_db_config, readonly)
    }

    fn open_db(
        path: PathBuf,
        name: &str,
        state_kv_db_config: &RocksdbConfig,
        readonly: bool,
    ) -> Result<DB> {
        Ok(if readonly {
            DB::open_cf_readonly(
                &gen_rocksdb_options(state_kv_db_config, true),
                path,
                name,
                state_kv_db_column_families(),
            )?
        } else {
            DB::open_cf(
                &gen_rocksdb_options(state_kv_db_config, false),
                path,
                name,
                gen_state_kv_cfds(state_kv_db_config),
            )?
        })
    }
}
