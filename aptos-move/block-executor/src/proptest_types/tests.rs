// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::Error,
    executor::BlockExecutor,
    proptest_types::types::{
        DeltaDataView, EmptyDataView, ExpectedOutput, KeyType, Output, Task, Transaction,
        TransactionGen, TransactionGenParams, ValueType,
    },
    txn_commit_hook::NoOpTransactionCommitHook,
};
use aptos_types::executable::ExecutableTestType;
use claims::assert_ok;
use num_cpus;
use proptest::{
    collection::vec,
    prelude::*,
    sample::Index,
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};
use rand::Rng;
use std::{cmp::max, fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};

fn run_transactions<K, V>(
    key_universe: &[K],
    transaction_gens: Vec<TransactionGen<V>>,
    abort_transactions: Vec<Index>,
    skip_rest_transactions: Vec<Index>,
    num_repeat: usize,
    module_access: (bool, bool),
    maybe_block_gas_limit: Option<u64>,
) where
    K: Hash + Clone + Debug + Eq + Send + Sync + PartialOrd + Ord + 'static,
    V: Clone + Eq + Send + Sync + Arbitrary + 'static,
    Vec<u8>: From<V>,
{
    let mut transactions: Vec<_> = transaction_gens
        .into_iter()
        .map(|txn_gen| txn_gen.materialize(key_universe, module_access))
        .collect();

    let length = transactions.len();
    for i in abort_transactions {
        *transactions.get_mut(i.index(length)).unwrap() = Transaction::Abort;
    }
    for i in skip_rest_transactions {
        *transactions.get_mut(i.index(length)).unwrap() = Transaction::SkipRest;
    }

    let data_view = EmptyDataView::<KeyType<K>, ValueType<V>> {
        phantom: PhantomData,
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    for _ in 0..num_repeat {
        let output = BlockExecutor::<
            Transaction<KeyType<K>, ValueType<V>>,
            Task<KeyType<K>, ValueType<V>>,
            EmptyDataView<KeyType<K>, ValueType<V>>,
            NoOpTransactionCommitHook<Output<KeyType<K>, ValueType<V>>, usize>,
            ExecutableTestType,
        >::new(
            num_cpus::get(),
            executor_thread_pool.clone(),
            maybe_block_gas_limit,
            None,
        )
        .execute_transactions_parallel((), &transactions, &data_view);

        if module_access.0 && module_access.1 {
            assert_eq!(output.unwrap_err(), Error::ModulePathReadWrite);
            continue;
        }

        let baseline =
            ExpectedOutput::generate_baseline(&transactions, None, maybe_block_gas_limit);
        baseline.assert_output(&output);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn no_early_termination(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }

    #[test]
    fn abort_only(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }

    #[test]
    fn skip_rest_only(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }

    #[test]
    fn mixed_transactions(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }

    #[test]
    fn dynamic_read_writes_mixed(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any_with::<TransactionGen<[u8;32]>>(TransactionGenParams::new_dynamic()), 3000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 3),
        skip_rest_transactions in vec(any::<Index>(), 3),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), None);
    }
}

fn dynamic_read_writes_with_block_gas_limit(num_txns: usize, maybe_block_gas_limit: Option<u64>) {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 100)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();
    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        num_txns,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();

    run_transactions(
        &universe,
        transaction_gen,
        vec![],
        vec![],
        100,
        (false, false),
        maybe_block_gas_limit,
    );
}

fn deltas_writes_mixed_with_block_gas_limit(num_txns: usize, maybe_block_gas_limit: Option<u64>) {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 50)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();
    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        num_txns,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();

    // Do not allow deletions as resolver can't apply delta to a deleted aggregator.
    let transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| txn_gen.materialize_with_deltas(&universe, 15, false))
        .collect();

    let data_view = DeltaDataView::<KeyType<[u8; 32]>, ValueType<[u8; 32]>> {
        phantom: PhantomData,
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    for _ in 0..20 {
        let output = BlockExecutor::<
            Transaction<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            Task<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            DeltaDataView<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            NoOpTransactionCommitHook<Output<KeyType<[u8; 32]>, ValueType<[u8; 32]>>, usize>,
            ExecutableTestType,
        >::new(
            num_cpus::get(),
            executor_thread_pool.clone(),
            maybe_block_gas_limit,
            None,
        )
        .execute_transactions_parallel((), &transactions, &data_view);

        let baseline =
            ExpectedOutput::generate_baseline(&transactions, None, maybe_block_gas_limit);
        baseline.assert_output(&output);
    }
}

fn deltas_resolver_with_block_gas_limit(num_txns: usize, maybe_block_gas_limit: Option<u64>) {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 50)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();
    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        num_txns,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();

    let data_view = DeltaDataView::<KeyType<[u8; 32]>, ValueType<[u8; 32]>> {
        phantom: PhantomData,
    };

    // Do not allow deletes as that would panic in resolver.
    let transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| txn_gen.materialize_with_deltas(&universe, 15, false))
        .collect();

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    for _ in 0..20 {
        let output = BlockExecutor::<
            Transaction<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            Task<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            DeltaDataView<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            NoOpTransactionCommitHook<Output<KeyType<[u8; 32]>, ValueType<[u8; 32]>>, usize>,
            ExecutableTestType,
        >::new(
            num_cpus::get(),
            executor_thread_pool.clone(),
            maybe_block_gas_limit,
            None,
        )
        .execute_transactions_parallel((), &transactions, &data_view);

        let delta_writes = output
            .as_ref()
            .expect("Must be success")
            .iter()
            .map(|out| out.delta_writes())
            .collect();

        let baseline = ExpectedOutput::generate_baseline(
            &transactions,
            Some(delta_writes),
            maybe_block_gas_limit,
        );
        baseline.assert_output(&output);
    }
}

fn dynamic_read_writes_contended_with_block_gas_limit(
    num_txns: usize,
    maybe_block_gas_limit: Option<u64>,
) {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 10)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();

    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        num_txns,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();

    run_transactions(
        &universe,
        transaction_gen,
        vec![],
        vec![],
        100,
        (false, false),
        maybe_block_gas_limit,
    );
}

fn module_publishing_fallback_with_block_gas_limit(
    num_txns: usize,
    maybe_block_gas_limit: Option<u64>,
) {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 100)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();
    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        num_txns,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();

    run_transactions(
        &universe,
        transaction_gen.clone(),
        vec![],
        vec![],
        2,
        (false, true),
        maybe_block_gas_limit,
    );
    run_transactions(
        &universe,
        transaction_gen.clone(),
        vec![],
        vec![],
        2,
        (false, true),
        maybe_block_gas_limit,
    );
    run_transactions(
        &universe,
        transaction_gen,
        vec![],
        vec![],
        2,
        (true, true),
        maybe_block_gas_limit,
    );
}

fn publishing_fixed_params_with_block_gas_limit(
    num_txns: usize,
    maybe_block_gas_limit: Option<u64>,
) {
    let mut runner = TestRunner::default();

    let universe = vec(any::<[u8; 32]>(), 50)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();
    let transaction_gen = vec(
        any_with::<TransactionGen<[u8; 32]>>(TransactionGenParams::new_dynamic()),
        num_txns,
    )
    .new_tree(&mut runner)
    .expect("creating a new value should succeed")
    .current();
    let indices = vec(any::<Index>(), 4)
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();

    // First 12 keys are normal paths, next 14 are module reads, then writes.
    let mut transactions: Vec<_> = transaction_gen
        .into_iter()
        .map(|txn_gen| txn_gen.materialize_disjoint_module_rw(&universe[0..40], 12, 26))
        .collect();

    // Adjust the writes of txn indices[0] to contain module write to key 42.
    let w_index = indices[0].index(num_txns);
    *transactions.get_mut(w_index).unwrap() = match transactions.get_mut(w_index).unwrap() {
        Transaction::Write {
            incarnation,
            reads,
            writes_and_deltas,
        } => {
            let mut new_writes_and_deltas = vec![];
            for (incarnation_writes, incarnation_deltas) in writes_and_deltas {
                assert!(!incarnation_writes.is_empty());
                let val = incarnation_writes[0].1.clone();
                let insert_idx = indices[1].index(incarnation_writes.len());
                incarnation_writes.insert(insert_idx, (KeyType(universe[42], true), val));
                new_writes_and_deltas
                    .push((incarnation_writes.clone(), incarnation_deltas.clone()));
            }

            Transaction::Write {
                incarnation: incarnation.clone(),
                reads: reads.clone(),
                writes_and_deltas: new_writes_and_deltas,
            }
        },
        _ => {
            unreachable!();
        },
    };

    let data_view = DeltaDataView::<KeyType<[u8; 32]>, ValueType<[u8; 32]>> {
        phantom: PhantomData,
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    // Confirm still no intersection
    let output = BlockExecutor::<
        Transaction<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
        Task<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
        DeltaDataView<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
        NoOpTransactionCommitHook<Output<KeyType<[u8; 32]>, ValueType<[u8; 32]>>, usize>,
        ExecutableTestType,
    >::new(
        num_cpus::get(),
        executor_thread_pool,
        maybe_block_gas_limit,
        None,
    )
    .execute_transactions_parallel((), &transactions, &data_view);
    assert_ok!(output);

    // Adjust the reads of txn indices[2] to contain module read to key 42.
    let r_index = indices[2].index(num_txns);
    *transactions.get_mut(r_index).unwrap() = match transactions.get_mut(r_index).unwrap() {
        Transaction::Write {
            incarnation,
            reads,
            writes_and_deltas,
        } => {
            let mut new_reads = vec![];
            for incarnation_reads in reads {
                assert!(!incarnation_reads.is_empty());
                let insert_idx = indices[3].index(incarnation_reads.len());
                incarnation_reads.insert(insert_idx, KeyType(universe[42], true));
                new_reads.push(incarnation_reads.clone());
            }

            Transaction::Write {
                incarnation: incarnation.clone(),
                reads: new_reads,
                writes_and_deltas: writes_and_deltas.clone(),
            }
        },
        _ => {
            unreachable!();
        },
    };

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap(),
    );

    for _ in 0..200 {
        let output = BlockExecutor::<
            Transaction<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            Task<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            DeltaDataView<KeyType<[u8; 32]>, ValueType<[u8; 32]>>,
            NoOpTransactionCommitHook<Output<KeyType<[u8; 32]>, ValueType<[u8; 32]>>, usize>,
            ExecutableTestType,
        >::new(
            num_cpus::get(),
            executor_thread_pool.clone(),
            Some(max(w_index, r_index) as u64 + 1),
            None,
        ) // Ensure enough gas limit to commit the module txns
        .execute_transactions_parallel((), &transactions, &data_view);

        assert_eq!(output.unwrap_err(), Error::ModulePathReadWrite);
    }
}

#[test]
fn dynamic_read_writes() {
    dynamic_read_writes_with_block_gas_limit(3000, None);
}

#[test]
fn deltas_writes_mixed() {
    deltas_writes_mixed_with_block_gas_limit(1000, None);
}

#[test]
fn deltas_resolver() {
    deltas_resolver_with_block_gas_limit(1000, None);
}

#[test]
fn dynamic_read_writes_contended() {
    dynamic_read_writes_contended_with_block_gas_limit(1000, None);
}

#[test]
fn module_publishing_fallback() {
    module_publishing_fallback_with_block_gas_limit(3000, None);
}

#[test]
// Test a single transaction intersection interleaves with a lot of dependencies and
// not overlapping module r/w keys.
fn module_publishing_races() {
    for _ in 0..5 {
        publishing_fixed_params_with_block_gas_limit(300, None);
    }
}

// The following set of tests are the same tests as above with per-block gas limit.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn no_early_termination_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 5000) as u64));
    }

    #[test]
    fn abort_only_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 10).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 0),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 10) as u64));
    }

    #[test]
    fn skip_rest_only_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 0),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 5000) as u64));
    }

    #[test]
    fn mixed_transactions_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any::<TransactionGen<[u8;32]>>(), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 5),
        skip_rest_transactions in vec(any::<Index>(), 5),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 5000) as u64));
    }

    #[test]
    fn dynamic_read_writes_mixed_with_block_gas_limit(
        universe in vec(any::<[u8; 32]>(), 100),
        transaction_gen in vec(any_with::<TransactionGen<[u8;32]>>(TransactionGenParams::new_dynamic()), 5000).no_shrink(),
        abort_transactions in vec(any::<Index>(), 3),
        skip_rest_transactions in vec(any::<Index>(), 3),
    ) {
        run_transactions(&universe, transaction_gen, abort_transactions, skip_rest_transactions, 1, (false, false), Some(rand::thread_rng().gen_range(0, 5000) as u64));
    }
}

#[test]
fn dynamic_read_writes_with_block_gas_limit_test() {
    dynamic_read_writes_with_block_gas_limit(
        3000,
        Some(rand::thread_rng().gen_range(0, 3000) as u64),
    );
    dynamic_read_writes_with_block_gas_limit(3000, Some(0));
}

#[test]
fn deltas_writes_mixed_with_block_gas_limit_test() {
    deltas_writes_mixed_with_block_gas_limit(
        1000,
        Some(rand::thread_rng().gen_range(0, 1000) as u64),
    );
    deltas_writes_mixed_with_block_gas_limit(1000, Some(0));
}

#[test]
fn deltas_resolver_with_block_gas_limit_test() {
    deltas_resolver_with_block_gas_limit(1000, Some(rand::thread_rng().gen_range(0, 1000) as u64));
    deltas_resolver_with_block_gas_limit(1000, Some(0));
}

#[test]
fn dynamic_read_writes_contended_with_block_gas_limit_test() {
    dynamic_read_writes_contended_with_block_gas_limit(
        1000,
        Some(rand::thread_rng().gen_range(0, 1000) as u64),
    );
    dynamic_read_writes_contended_with_block_gas_limit(1000, Some(0));
}

#[test]
fn module_publishing_fallback_with_block_gas_limit_test() {
    module_publishing_fallback_with_block_gas_limit(
        3000,
        // Need to execute at least 2 txns to trigger module publishing fallback
        Some(rand::thread_rng().gen_range(1, 3000) as u64),
    );
}

#[test]
// Test a single transaction intersection interleaves with a lot of dependencies and
// not overlapping module r/w keys.
fn module_publishing_races_with_block_gas_limit_test() {
    for _ in 0..5 {
        publishing_fixed_params_with_block_gas_limit(
            300,
            Some(rand::thread_rng().gen_range(0, 300) as u64),
        );
    }
}
