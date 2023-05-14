// Copyright © Aptos Foundation

use aptos_schemadb::{DB, Options, SchemaBatch};
use std::path::Path;
use std::any::Any;
use anyhow::Error;
use crate::dag::dag_storage::{ContainsKey, DagStorage, DagStoreWriteBatch, ItemId};
use crate::dag::types::{DagInMem, DagInMem_Key, DagInMemSchema, DagRoundList, DagRoundListSchema, MissingNodeIdToStatusMap, MissingNodeIdToStatusMapSchema, WeakLinksCreator, WeakLinksCreatorSchema};

pub struct NaiveDagStoreWriteBatch {
    inner: SchemaBatch,
}

impl NaiveDagStoreWriteBatch {
    pub(crate) fn new() -> Self {
        Self {
            inner: SchemaBatch::new()
        }
    }
}

impl DagStoreWriteBatch for NaiveDagStoreWriteBatch {
    fn put_dag_in_mem(&mut self, obj: &DagInMem) -> anyhow::Result<()> {
        self.inner.put::<DagInMemSchema>(&obj.key(), &obj.partial())?;
        self.put_dag_round_list(obj.get_dag())?;
        self.put_weak_link_creator(obj.get_front())?;
        self.put_missing_node_id_to_status_map(obj.get_missing_nodes())?;
        Ok(())
    }

    fn put_dag_round_list(&mut self, obj: &DagRoundList) -> anyhow::Result<()> {
        self.inner.put::<DagRoundListSchema>(&obj.key(), obj)
    }

    fn put_weak_link_creator(&mut self, obj: &WeakLinksCreator) -> anyhow::Result<()> {
        self.inner.put::<WeakLinksCreatorSchema>(&obj.key(), obj)
    }

    fn put_missing_node_id_to_status_map(&mut self, obj: &MissingNodeIdToStatusMap) -> anyhow::Result<()> {
        self.inner.put::<MissingNodeIdToStatusMapSchema>(&obj.key(), obj)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct NaiveDagStore {
    db: DB,
}

impl NaiveDagStore {
    pub fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![
            "DagInMem",
            "DagRoundList",
            "MissingNodeIdToStatusMap",
            "WeakLinksCreator",
        ];

        let path = db_root_path.as_ref().join(DAG_DB_NAME);
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open(path.clone(), DAG_DB_NAME, column_families, &opts)
            .expect("ReliableBroadcastDB open failed; unable to continue");
        Self {
            db
        }
    }
}


impl DagStorage for NaiveDagStore {

    fn load_dag_in_mem(&self, key: &DagInMem_Key) -> anyhow::Result<Option<DagInMem>> {
        let maybe_partial = self.db.get::<DagInMemSchema>(key)?;
        if let Some(partial) = maybe_partial {
            let maybe_front = self.load_weak_link_creator(&partial.front)?;
            let maybe_dag =  self.load_dag_round_list(&partial.dag)?;
            let maybe_missing_nodes = self.load_missing_node_id_to_status_map(&partial.missing_nodes)?;
            if let (Some(front), Some(dag), Some(missing_nodes)) = (maybe_front, maybe_dag, maybe_missing_nodes) {
                let obj = DagInMem{
                    my_id: partial.my_id,
                    epoch: partial.epoch,
                    current_round: partial.current_round,
                    front,
                    dag,
                    missing_nodes,
                };
                Ok(Some(obj))
            } else {
                Err(Error::msg("Inconsistency."))
            }
        } else {
            Ok(None)
        }
    }

    fn load_weak_link_creator(&self, key: &ItemId) -> anyhow::Result<Option<WeakLinksCreator>> {
        todo!()
    }

    fn load_dag_round_list(&self, key: &ItemId) -> anyhow::Result<Option<DagRoundList>> {
        todo!()
    }

    fn load_missing_node_id_to_status_map(&self, key: &ItemId) -> anyhow::Result<Option<MissingNodeIdToStatusMap>> {
        todo!()
    }

    fn new_write_batch(&self) -> Box<dyn DagStoreWriteBatch> {
        Box::new(NaiveDagStoreWriteBatch::new())
    }

    fn commit_write_batch(&self, batch: Box<dyn DagStoreWriteBatch>) -> anyhow::Result<()> {
        let x = batch.as_any().downcast_ref::<NaiveDagStoreWriteBatch>().unwrap();
        self.db.write_schemas_ref(&x.inner)
    }
}

const DAG_DB_NAME: &str = "DagDB";
