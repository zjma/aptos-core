// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::{
        discard_error_output, discard_error_vm_status, PreprocessedTransaction, VMAdapter,
    },
    aptos_vm_impl::{get_transaction_output, AptosVMImpl, AptosVMInternals},
    block_executor::BlockAptosVM,
    counters::*,
    data_cache::{AsMoveResolver, IntoMoveResolver, StorageAdapter},
    delta_state_view::DeltaStateView,
    errors::expect_only_successful_execution,
    move_vm_ext::{MoveResolverExt, SessionExt, SessionId},
    system_module_names::*,
    transaction_metadata::TransactionMetadata,
    verifier, VMExecutor, VMValidator,
};
use anyhow::{anyhow, Result};
use aptos_aggregator::{
    delta_change_set::DeltaChangeSet,
    transaction::{ChangeSetExt, TransactionOutputExt},
};
use aptos_crypto::HashValue;
use aptos_framework::natives::code::PublishRequest;
use aptos_gas::{
    AptosGasMeter, AptosGasParameters, ChangeSetConfigs, Gas, StandardGasMeter,
    StorageGasParameters,
};
use aptos_logger::prelude::*;
use aptos_state_view::StateView;
use aptos_types::{
    account_config,
    account_config::new_block_event_key,
    block_metadata::BlockMetadata,
    on_chain_config::{new_epoch_event_key, FeatureFlag, TimedFeatureOverride},
    transaction::{
        ChangeSet, EntryFunction, ExecutionError, ExecutionStatus, ModuleBundle, Multisig,
        MultisigTransactionPayload, SignatureCheckedTransaction, SignedTransaction, Transaction,
        TransactionOutput, TransactionPayload, TransactionStatus, VMValidatorResult,
        WriteSetPayload,
    },
    vm_status::{AbortLocation, DiscardedVMStatus, StatusCode, VMStatus},
    write_set::WriteSet,
};
use aptos_vm_logging::{init_speculative_logs, log_schema::AdapterLogSchema};
use fail::fail_point;
use move_binary_format::{
    access::ModuleAccess,
    compatibility::Compatibility,
    errors::{verification_error, Location, PartialVMError, VMError, VMResult},
    CompiledModule, IndexKind,
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    transaction_argument::convert_txn_args,
    value::{serialize_values, MoveValue},
};
use move_vm_runtime::session::SerializedReturnValues;
use move_vm_types::gas::UnmeteredGasMeter;
use num_cpus;
use once_cell::sync::OnceCell;
use std::{
    cmp::min,
    collections::{BTreeMap, BTreeSet},
    convert::{AsMut, AsRef},
    marker::Sync,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

static EXECUTION_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
static NUM_PROOF_READING_THREADS: OnceCell<usize> = OnceCell::new();
static PARANOID_TYPE_CHECKS: OnceCell<bool> = OnceCell::new();
static PROCESSED_TRANSACTIONS_DETAILED_COUNTERS: OnceCell<bool> = OnceCell::new();
static TIMED_FEATURE_OVERRIDE: OnceCell<TimedFeatureOverride> = OnceCell::new();

/// Remove this once the bundle is removed from the code.
static MODULE_BUNDLE_DISALLOWED: AtomicBool = AtomicBool::new(true);
pub fn allow_module_bundle_for_test() {
    MODULE_BUNDLE_DISALLOWED.store(false, Ordering::Relaxed);
}

#[derive(Clone)]
pub struct AptosVM(pub(crate) AptosVMImpl);

struct AptosSimulationVM(AptosVM);

macro_rules! unwrap_or_discard {
    ($res:expr) => {
        match $res {
            Ok(s) => s,
            Err(e) => return discard_error_vm_status(e),
        }
    };
}

impl AptosVM {
    pub fn new<S: StateView>(state: &S) -> Self {
        Self(AptosVMImpl::new(state))
    }

    pub fn new_for_validation<S: StateView>(state: &S) -> Self {
        info!(
            AdapterLogSchema::new(state.id(), 0),
            "Adapter created for Validation"
        );
        Self::new(state)
    }

    /// Sets execution concurrency level when invoked the first time.
    pub fn set_concurrency_level_once(mut concurrency_level: usize) {
        concurrency_level = min(concurrency_level, num_cpus::get());
        // Only the first call succeeds, due to OnceCell semantics.
        EXECUTION_CONCURRENCY_LEVEL.set(concurrency_level).ok();
    }

    /// Get the concurrency level if already set, otherwise return default 1
    /// (sequential execution).
    ///
    /// The concurrency level is fixed to 1 if gas profiling is enabled.
    pub fn get_concurrency_level() -> usize {
        match EXECUTION_CONCURRENCY_LEVEL.get() {
            Some(concurrency_level) => *concurrency_level,
            None => 1,
        }
    }

    /// Sets runtime config when invoked the first time.
    pub fn set_paranoid_type_checks(enable: bool) {
        // Only the first call succeeds, due to OnceCell semantics.
        PARANOID_TYPE_CHECKS.set(enable).ok();
    }

    /// Get the paranoid type check flag if already set, otherwise return default true
    pub fn get_paranoid_checks() -> bool {
        match PARANOID_TYPE_CHECKS.get() {
            Some(enable) => *enable,
            None => true,
        }
    }

    // Set the override profile for timed features.
    pub fn set_timed_feature_override(profile: TimedFeatureOverride) {
        TIMED_FEATURE_OVERRIDE.set(profile).ok();
    }

    pub fn get_timed_feature_override() -> Option<TimedFeatureOverride> {
        TIMED_FEATURE_OVERRIDE.get().cloned()
    }

    /// Sets the # of async proof reading threads.
    pub fn set_num_proof_reading_threads_once(mut num_threads: usize) {
        // TODO(grao): Do more analysis to tune this magic number.
        num_threads = min(num_threads, 256);
        // Only the first call succeeds, due to OnceCell semantics.
        NUM_PROOF_READING_THREADS.set(num_threads).ok();
    }

    /// Returns the # of async proof reading threads if already set, otherwise return default value
    /// (32).
    pub fn get_num_proof_reading_threads() -> usize {
        match NUM_PROOF_READING_THREADS.get() {
            Some(num_threads) => *num_threads,
            None => 32,
        }
    }

    /// Sets addigional details in counters when invoked the first time.
    pub fn set_processed_transactions_detailed_counters() {
        // Only the first call succeeds, due to OnceCell semantics.
        PROCESSED_TRANSACTIONS_DETAILED_COUNTERS.set(true).ok();
    }

    /// Get whether we should capture additional details in counters
    pub fn get_processed_transactions_detailed_counters() -> bool {
        match PROCESSED_TRANSACTIONS_DETAILED_COUNTERS.get() {
            Some(value) => *value,
            None => false,
        }
    }

    pub fn internals(&self) -> AptosVMInternals {
        AptosVMInternals::new(&self.0)
    }

    /// Load a module into its internal MoveVM's code cache.
    pub fn load_module<S: MoveResolverExt>(
        &self,
        module_id: &ModuleId,
        state: &S,
    ) -> VMResult<Arc<CompiledModule>> {
        self.0.load_module(module_id, state)
    }

    /// Generates a transaction output for a transaction that encountered errors during the
    /// execution process. This is public for now only for tests.
    pub fn failed_transaction_cleanup<S: MoveResolverExt>(
        &self,
        error_code: VMStatus,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        storage: &S,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
    ) -> TransactionOutputExt {
        self.failed_transaction_cleanup_and_keep_vm_status(
            error_code,
            gas_meter,
            txn_data,
            storage,
            log_context,
            change_set_configs,
        )
        .1
    }

    fn failed_transaction_cleanup_and_keep_vm_status<S: MoveResolverExt>(
        &self,
        error_code: VMStatus,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        storage: &S,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
    ) -> (VMStatus, TransactionOutputExt) {
        let resolver = self.0.new_move_resolver(storage);
        let mut session = self.0.new_session(&resolver, SessionId::txn_meta(txn_data));

        match TransactionStatus::from(error_code.clone()) {
            TransactionStatus::Keep(status) => {
                // Inject abort info if available.
                let status = match status {
                    ExecutionStatus::MoveAbort {
                        location: AbortLocation::Module(module),
                        code,
                        ..
                    } => {
                        let info = self.0.extract_abort_info(&module, code);
                        ExecutionStatus::MoveAbort {
                            location: AbortLocation::Module(module),
                            code,
                            info,
                        }
                    },
                    _ => status,
                };
                // The transaction should be charged for gas, so run the epilogue to do that.
                // This is running in a new session that drops any side effects from the
                // attempted transaction (e.g., spending funds that were needed to pay for gas),
                // so even if the previous failure occurred while running the epilogue, it
                // should not fail now. If it somehow fails here, there is no choice but to
                // discard the transaction.
                if let Err(e) = self.0.run_failure_epilogue(
                    &mut session,
                    gas_meter.balance(),
                    txn_data,
                    log_context,
                ) {
                    return discard_error_vm_status(e);
                }
                let txn_output = get_transaction_output(
                    &mut (),
                    session,
                    gas_meter.balance(),
                    txn_data,
                    status,
                    change_set_configs,
                )
                .unwrap_or_else(|e| discard_error_vm_status(e).1);
                (error_code, txn_output)
            },
            TransactionStatus::Discard(status) => {
                (VMStatus::Error(status, None), discard_error_output(status))
            },
            TransactionStatus::Retry => unreachable!(),
        }
    }

    fn success_transaction_cleanup<S: MoveResolverExt>(
        &self,
        storage: &S,
        user_txn_change_set_ext: ChangeSetExt,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, TransactionOutputExt), VMStatus> {
        let storage_with_changes =
            DeltaStateView::new(storage, user_txn_change_set_ext.write_set());
        // TODO: at this point we know that delta application failed
        // (and it should have occurred in user transaction in general).
        // We need to rerun the epilogue and charge gas. Currently, the use
        // case of an aggregator is for gas fees (which are computed in
        // the epilogue), and therefore this should never happen.
        // Also, it is worth mentioning that current VM error handling is
        // rather ugly and has a lot of legacy code. This makes proper error
        // handling quite challenging.
        let delta_write_set_mut = user_txn_change_set_ext
            .delta_change_set()
            .clone()
            .try_into_write_set_mut(storage)
            .expect("something terrible happened when applying aggregator deltas");
        let delta_write_set = delta_write_set_mut
            .freeze()
            .map_err(|_err| VMStatus::Error(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, None))?;
        let storage_with_changes =
            DeltaStateView::new(&storage_with_changes, &delta_write_set).into_move_resolver();

        let resolver = self.0.new_move_resolver(&storage_with_changes);
        let mut session = self.0.new_session(&resolver, SessionId::txn_meta(txn_data));

        self.0
            .run_success_epilogue(&mut session, gas_meter.balance(), txn_data, log_context)?;

        let epilogue_change_set_ext = session
            .finish(&mut (), change_set_configs)
            .map_err(|e| e.into_vm_status())?;
        let change_set_ext = user_txn_change_set_ext
            .squash(epilogue_change_set_ext)
            .map_err(|_err| VMStatus::Error(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, None))?;

        let (delta_change_set, change_set) = change_set_ext.into_inner();
        let (write_set, events) = change_set.into_inner();

        let gas_used = txn_data
            .max_gas_amount()
            .checked_sub(gas_meter.balance())
            .expect("Balance should always be less than or equal to max gas amount");

        let txn_output = TransactionOutput::new(
            write_set,
            events,
            gas_used.into(),
            TransactionStatus::Keep(ExecutionStatus::Success),
        );

        Ok((
            VMStatus::Executed,
            TransactionOutputExt::new(delta_change_set, txn_output),
        ))
    }

    fn validate_and_execute_entry_function<SS: MoveResolverExt>(
        &self,
        session: &mut SessionExt<SS>,
        gas_meter: &mut impl AptosGasMeter,
        senders: Vec<AccountAddress>,
        script_fn: &EntryFunction,
    ) -> Result<SerializedReturnValues, VMStatus> {
        let function = session.load_function(
            script_fn.module(),
            script_fn.function(),
            script_fn.ty_args(),
        )?;
        let struct_constructors = self
            .0
            .get_features()
            .is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS);
        let args = verifier::transaction_arg_validation::validate_combine_signer_and_txn_args(
            session,
            senders,
            script_fn.args().to_vec(),
            &function,
            struct_constructors,
        )?;
        session
            .execute_entry_function(
                script_fn.module(),
                script_fn.function(),
                script_fn.ty_args().to_vec(),
                args,
                gas_meter,
            )
            .map_err(|e| e.into_vm_status())
    }

    fn execute_script_or_entry_function<S: MoveResolverExt, SS: MoveResolverExt>(
        &self,
        storage: &S,
        mut session: SessionExt<SS>,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        payload: &TransactionPayload,
        log_context: &AdapterLogSchema,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, TransactionOutputExt), VMStatus> {
        fail_point!("move_adapter::execute_script_or_entry_function", |_| {
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        // Run the execution logic
        {
            gas_meter
                .charge_intrinsic_gas_for_transaction(txn_data.transaction_size())
                .map_err(|e| e.into_vm_status())?;

            match payload {
                TransactionPayload::Script(script) => {
                    let mut senders = vec![txn_data.sender()];
                    senders.extend(txn_data.secondary_signers());
                    let loaded_func =
                        session.load_script(script.code(), script.ty_args().to_vec())?;
                    let args =
                        verifier::transaction_arg_validation::validate_combine_signer_and_txn_args(
                            &mut session,
                            senders,
                            convert_txn_args(script.args()),
                            &loaded_func,
                            self.0
                                .get_features()
                                .is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
                        )?;
                    session
                        .execute_script(script.code(), script.ty_args().to_vec(), args, gas_meter)
                        .map_err(|e| e.into_vm_status())?;
                },
                TransactionPayload::EntryFunction(script_fn) => {
                    let mut senders = vec![txn_data.sender()];

                    senders.extend(txn_data.secondary_signers());
                    self.validate_and_execute_entry_function(
                        &mut session,
                        gas_meter,
                        senders,
                        script_fn,
                    )?;
                },

                // Not reachable as this function should only be invoked for entry or script
                // transaction payload.
                _ => {
                    return Err(VMStatus::Error(StatusCode::UNREACHABLE, None));
                },
            };

            self.resolve_pending_code_publish(
                &mut session,
                gas_meter,
                new_published_modules_loaded,
            )?;

            let change_set_ext = session
                .finish(&mut (), change_set_configs)
                .map_err(|e| e.into_vm_status())?;
            gas_meter.charge_io_gas_for_write_set(change_set_ext.write_set().iter())?;
            gas_meter.charge_storage_fee_for_all(
                change_set_ext.write_set().iter(),
                change_set_ext.change_set().events(),
                txn_data.transaction_size,
                txn_data.gas_unit_price,
            )?;
            // TODO(Gas): Charge for aggregator writes

            self.success_transaction_cleanup(
                storage,
                change_set_ext,
                gas_meter,
                txn_data,
                log_context,
                change_set_configs,
            )
        }
    }

    // Execute a multisig transaction:
    // 1. Obtain the payload of the transaction to execute. This could have been stored on chain
    // when the multisig transaction was created.
    // 2. Execute the target payload. If this fails, discard the session and keep the gas meter and
    // failure object. In case of success, keep the session and also do any necessary module publish
    // cleanup.
    // 3. Call post transaction cleanup function in multisig account module with the result from (2)
    fn execute_multisig_transaction<S: MoveResolverExt + StateView, SS: MoveResolverExt>(
        &self,
        storage: &S,
        mut session: SessionExt<SS>,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        txn_payload: &Multisig,
        log_context: &AdapterLogSchema,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, TransactionOutputExt), VMStatus> {
        fail_point!("move_adapter::execute_multisig_transaction", |_| {
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        gas_meter
            .charge_intrinsic_gas_for_transaction(txn_data.transaction_size())
            .map_err(|e| e.into_vm_status())?;

        // Step 1: Obtain the payload. If any errors happen here, the entire transaction should fail
        let invariant_violation_error =
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .finish(Location::Undefined);
        let provided_payload = if let Some(payload) = &txn_payload.transaction_payload {
            bcs::to_bytes(&payload).map_err(|_| invariant_violation_error.clone())?
        } else {
            // Default to empty bytes if payload is not provided.
            bcs::to_bytes::<Vec<u8>>(&vec![]).map_err(|_| invariant_violation_error.clone())?
        };
        // Failures here will be propagated back.
        let payload_bytes: Vec<Vec<u8>> = session
            .execute_function_bypass_visibility(
                &MULTISIG_ACCOUNT_MODULE,
                GET_NEXT_TRANSACTION_PAYLOAD,
                vec![],
                serialize_values(&vec![
                    MoveValue::Address(txn_payload.multisig_address),
                    MoveValue::vector_u8(provided_payload),
                ]),
                gas_meter,
            )?
            .return_values
            .into_iter()
            .map(|(bytes, _ty)| bytes)
            .collect::<Vec<_>>();
        let payload_bytes = payload_bytes
            .first()
            // We expect the payload to either exists on chain or be passed along with the
            // transaction.
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .finish(Location::Undefined)
            })?;
        // We have to deserialize twice as the first time returns the actual return type of the
        // function, which is vec<u8>. The second time deserializes it into the correct
        // EntryFunction payload type.
        // If either deserialization fails for some reason, that means the user provided incorrect
        // payload data either during transaction creation or execution.
        let deserialization_error = PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
            .finish(Location::Undefined);
        let payload_bytes =
            bcs::from_bytes::<Vec<u8>>(payload_bytes).map_err(|_| deserialization_error.clone())?;
        let payload = bcs::from_bytes::<MultisigTransactionPayload>(&payload_bytes)
            .map_err(|_| deserialization_error)?;

        // Step 2: Execute the target payload. Transaction failure here is tolerated. In case of any
        // failures, we'll discard the session and start a new one. This ensures that any data
        // changes are not persisted.
        // The multisig transaction would still be considered executed even if execution fails.
        let execution_result = match payload {
            MultisigTransactionPayload::EntryFunction(entry_function) => self
                .execute_multisig_entry_function(
                    &mut session,
                    gas_meter,
                    txn_payload.multisig_address,
                    &entry_function,
                    new_published_modules_loaded,
                ),
        };

        // Step 3: Call post transaction cleanup function in multisig account module with the result
        // from Step 2.
        // Note that we don't charge execution or writeset gas for cleanup routines. This is
        // consistent with the high-level success/failure cleanup routines for user transactions.
        let cleanup_args = serialize_values(&vec![
            MoveValue::Address(txn_data.sender),
            MoveValue::Address(txn_payload.multisig_address),
            MoveValue::vector_u8(payload_bytes),
        ]);
        let final_change_set_ext = if let Err(execution_error) = execution_result {
            // Invalidate the loader cache in case there was a new module loaded from a module
            // publish request that failed.
            // This is redundant with the logic in execute_user_transaction but unfortunately is
            // necessary here as executing the underlying call can fail without this function
            // returning an error to execute_user_transaction.
            if *new_published_modules_loaded {
                self.0.mark_loader_cache_as_invalid();
            };
            self.failure_multisig_payload_cleanup(
                storage,
                execution_error,
                txn_data,
                cleanup_args,
                change_set_configs,
            )?
        } else {
            self.success_multisig_payload_cleanup(
                storage,
                session,
                gas_meter,
                txn_data,
                cleanup_args,
                change_set_configs,
            )?
        };

        // TODO(Gas): Charge for aggregator writes
        self.success_transaction_cleanup(
            storage,
            final_change_set_ext,
            gas_meter,
            txn_data,
            log_context,
            change_set_configs,
        )
    }

    fn execute_multisig_entry_function<SS: MoveResolverExt>(
        &self,
        session: &mut SessionExt<SS>,
        gas_meter: &mut impl AptosGasMeter,
        multisig_address: AccountAddress,
        payload: &EntryFunction,
        new_published_modules_loaded: &mut bool,
    ) -> Result<(), VMStatus> {
        // If txn args are not valid, we'd still consider the transaction as executed but
        // failed. This is primarily because it's unrecoverable at this point.
        self.validate_and_execute_entry_function(
            session,
            gas_meter,
            vec![multisig_address],
            payload,
        )?;

        // Resolve any pending module publishes in case the multisig transaction is deploying
        // modules.
        self.resolve_pending_code_publish(session, gas_meter, new_published_modules_loaded)?;
        Ok(())
    }

    fn success_multisig_payload_cleanup<S: MoveResolverExt + StateView, SS: MoveResolverExt>(
        &self,
        storage: &S,
        session: SessionExt<SS>,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        cleanup_args: Vec<Vec<u8>>,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<ChangeSetExt, VMStatus> {
        // Charge gas for writeset before we do cleanup. This ensures we don't charge gas for
        // cleanup writeset changes, which is consistent with outer-level success cleanup
        // flow. We also wouldn't need to worry that we run out of gas when doing cleanup.
        let inner_function_change_set_ext = session
            .finish(&mut (), change_set_configs)
            .map_err(|e| e.into_vm_status())?;
        gas_meter.charge_io_gas_for_write_set(inner_function_change_set_ext.write_set().iter())?;
        gas_meter.charge_storage_fee_for_all(
            inner_function_change_set_ext.write_set().iter(),
            inner_function_change_set_ext.change_set().events(),
            txn_data.transaction_size,
            txn_data.gas_unit_price,
        )?;

        let storage_with_changes =
            DeltaStateView::new(storage, inner_function_change_set_ext.write_set());
        let delta_write_set_mut = inner_function_change_set_ext
            .delta_change_set()
            .clone()
            .try_into_write_set_mut(storage)
            .expect("something terrible happened when applying aggregator deltas");
        let delta_write_set = delta_write_set_mut
            .freeze()
            .map_err(|_err| VMStatus::Error(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, None))?;
        let storage_with_changes =
            DeltaStateView::new(&storage_with_changes, &delta_write_set).into_move_resolver();
        let resolver = self.0.new_move_resolver(&storage_with_changes);
        let mut cleanup_session = self.0.new_session(&resolver, SessionId::txn_meta(txn_data));
        cleanup_session.execute_function_bypass_visibility(
            &MULTISIG_ACCOUNT_MODULE,
            SUCCESSFUL_TRANSACTION_EXECUTION_CLEANUP,
            vec![],
            cleanup_args,
            &mut UnmeteredGasMeter,
        )?;
        let cleanup_change_set_ext = cleanup_session
            .finish(&mut (), change_set_configs)
            .map_err(|e| e.into_vm_status())?;
        // Merge the inner function writeset with cleanup writeset.
        inner_function_change_set_ext
            .squash(cleanup_change_set_ext)
            .map_err(|_err| VMStatus::Error(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, None))
    }

    fn failure_multisig_payload_cleanup<S: MoveResolverExt + StateView>(
        &self,
        storage: &S,
        execution_error: VMStatus,
        txn_data: &TransactionMetadata,
        mut cleanup_args: Vec<Vec<u8>>,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<ChangeSetExt, VMStatus> {
        // Start a fresh session for running cleanup that does not contain any changes from
        // the inner function call earlier (since it failed).
        let mut cleanup_session = self.0.new_session(storage, SessionId::txn_meta(txn_data));
        let execution_error = ExecutionError::try_from(execution_error)
            .map_err(|_| VMStatus::Error(StatusCode::UNREACHABLE, None))?;
        // Serialization is not expected to fail so we're using invariant_violation error here.
        cleanup_args.push(bcs::to_bytes(&execution_error).map_err(|_| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .finish(Location::Undefined)
        })?);
        cleanup_session.execute_function_bypass_visibility(
            &MULTISIG_ACCOUNT_MODULE,
            FAILED_TRANSACTION_EXECUTION_CLEANUP,
            vec![],
            cleanup_args,
            &mut UnmeteredGasMeter,
        )?;
        cleanup_session
            .finish(&mut (), change_set_configs)
            .map_err(|e| e.into_vm_status())
    }

    fn verify_module_bundle<S: MoveResolverExt>(
        session: &mut SessionExt<S>,
        module_bundle: &ModuleBundle,
    ) -> VMResult<()> {
        for module_blob in module_bundle.iter() {
            match CompiledModule::deserialize(module_blob.code()) {
                Ok(module) => {
                    // verify the module doesn't exist
                    if session
                        .get_data_store()
                        .load_module(&module.self_id())
                        .is_ok()
                    {
                        return Err(verification_error(
                            StatusCode::DUPLICATE_MODULE_NAME,
                            IndexKind::AddressIdentifier,
                            module.self_handle_idx().0,
                        )
                        .finish(Location::Undefined));
                    }
                },
                Err(err) => return Err(err.finish(Location::Undefined)),
            }
        }
        Ok(())
    }

    /// Execute all module initializers.
    fn execute_module_initialization<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
        gas_meter: &mut impl AptosGasMeter,
        modules: &[CompiledModule],
        exists: BTreeSet<ModuleId>,
        senders: &[AccountAddress],
        new_published_modules_loaded: &mut bool,
    ) -> VMResult<()> {
        let init_func_name = ident_str!("init_module");
        for module in modules {
            if exists.contains(&module.self_id()) {
                // Call initializer only on first publish.
                continue;
            }
            *new_published_modules_loaded = true;
            let init_function = session.load_function(&module.self_id(), init_func_name, &[]);
            // it is ok to not have init_module function
            // init_module function should be (1) private and (2) has no return value
            // Note that for historic reasons, verification here is treated
            // as StatusCode::CONSTRAINT_NOT_SATISFIED, there this cannot be unified
            // with the general verify_module above.
            if init_function.is_ok() {
                if verifier::module_init::verify_module_init_function(module).is_ok() {
                    let args: Vec<Vec<u8>> = senders
                        .iter()
                        .map(|s| MoveValue::Signer(*s).simple_serialize().unwrap())
                        .collect();
                    session.execute_function_bypass_visibility(
                        &module.self_id(),
                        init_func_name,
                        vec![],
                        args,
                        gas_meter,
                    )?;
                } else {
                    return Err(PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
                        .finish(Location::Undefined));
                }
            }
        }
        Ok(())
    }

    /// Deserialize a module bundle.
    fn deserialize_module_bundle(&self, modules: &ModuleBundle) -> VMResult<Vec<CompiledModule>> {
        let max_version = if self
            .0
            .get_features()
            .is_enabled(FeatureFlag::VM_BINARY_FORMAT_V6)
        {
            6
        } else {
            5
        };
        let mut result = vec![];
        for module_blob in modules.iter() {
            match CompiledModule::deserialize_with_max_version(module_blob.code(), max_version) {
                Ok(module) => {
                    result.push(module);
                },
                Err(_err) => {
                    return Err(PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                        .finish(Location::Undefined))
                },
            }
        }
        Ok(result)
    }

    /// Execute a module bundle load request.
    /// TODO: this is going to be deprecated and removed in favor of code publishing via
    /// NativeCodeContext
    fn execute_modules<S: MoveResolverExt, SS: MoveResolverExt>(
        &self,
        storage: &S,
        mut session: SessionExt<SS>,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        modules: &ModuleBundle,
        log_context: &AdapterLogSchema,
        new_published_modules_loaded: &mut bool,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMStatus, TransactionOutputExt), VMStatus> {
        if MODULE_BUNDLE_DISALLOWED.load(Ordering::Relaxed) {
            return Err(VMStatus::Error(StatusCode::FEATURE_UNDER_GATING, None));
        }
        fail_point!("move_adapter::execute_module", |_| {
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        gas_meter
            .charge_intrinsic_gas_for_transaction(txn_data.transaction_size())
            .map_err(|e| e.into_vm_status())?;

        Self::verify_module_bundle(&mut session, modules)?;
        session
            .publish_module_bundle_with_compat_config(
                modules.clone().into_inner(),
                txn_data.sender(),
                gas_meter,
                Compatibility::new(
                    true,
                    true,
                    !self
                        .0
                        .get_features()
                        .is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
                ),
            )
            .map_err(|e| e.into_vm_status())?;

        // call init function of the each module
        self.execute_module_initialization(
            &mut session,
            gas_meter,
            &self.deserialize_module_bundle(modules)?,
            BTreeSet::new(),
            &[txn_data.sender()],
            new_published_modules_loaded,
        )?;

        let change_set_ext = session
            .finish(&mut (), change_set_configs)
            .map_err(|e| e.into_vm_status())?;
        gas_meter.charge_io_gas_for_write_set(change_set_ext.write_set().iter())?;
        gas_meter.charge_storage_fee_for_all(
            change_set_ext.write_set().iter(),
            change_set_ext.change_set().events(),
            txn_data.transaction_size,
            txn_data.gas_unit_price,
        )?;
        // TODO(Gas): Charge for aggregator writes

        self.success_transaction_cleanup(
            storage,
            change_set_ext,
            gas_meter,
            txn_data,
            log_context,
            change_set_configs,
        )
    }

    /// Resolve a pending code publish request registered via the NativeCodeContext.
    fn resolve_pending_code_publish<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
        gas_meter: &mut impl AptosGasMeter,
        new_published_modules_loaded: &mut bool,
    ) -> VMResult<()> {
        if let Some(PublishRequest {
            destination,
            bundle,
            expected_modules,
            allowed_deps,
            check_compat: _,
        }) = session.extract_publish_request()
        {
            // TODO: unfortunately we need to deserialize the entire bundle here to handle
            // `init_module` and verify some deployment conditions, while the VM need to do
            // the deserialization again. Consider adding an API to MoveVM which allows to
            // directly pass CompiledModule.
            let modules = self.deserialize_module_bundle(&bundle)?;

            // Validate the module bundle
            self.validate_publish_request(session, &modules, expected_modules, allowed_deps)?;

            // Check what modules exist before publishing.
            let mut exists = BTreeSet::new();
            for m in &modules {
                let id = m.self_id();
                if session.get_data_store().exists_module(&id)? {
                    exists.insert(id);
                }
            }

            // Publish the bundle and execute initializers
            // publish_module_bundle doesn't actually load the published module into
            // the loader cache. It only puts the module data in the data cache.
            session
                .publish_module_bundle_with_compat_config(
                    bundle.into_inner(),
                    destination,
                    gas_meter,
                    Compatibility::new(
                        true,
                        true,
                        !self
                            .0
                            .get_features()
                            .is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
                    ),
                )
                .and_then(|_| {
                    self.execute_module_initialization(
                        session,
                        gas_meter,
                        &modules,
                        exists,
                        &[destination],
                        new_published_modules_loaded,
                    )
                })
        } else {
            Ok(())
        }
    }

    /// Validate a publish request.
    fn validate_publish_request<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
        modules: &[CompiledModule],
        mut expected_modules: BTreeSet<String>,
        allowed_deps: Option<BTreeMap<AccountAddress, BTreeSet<String>>>,
    ) -> VMResult<()> {
        for m in modules {
            if !expected_modules.remove(m.self_id().name().as_str()) {
                return Err(Self::metadata_validation_error(&format!(
                    "unregistered module: '{}'",
                    m.self_id().name()
                )));
            }
            if let Some(allowed) = &allowed_deps {
                for dep in m.immediate_dependencies() {
                    if !allowed
                        .get(dep.address())
                        .map(|modules| {
                            modules.contains("") || modules.contains(dep.name().as_str())
                        })
                        .unwrap_or(false)
                    {
                        return Err(Self::metadata_validation_error(&format!(
                            "unregistered dependency: '{}'",
                            dep
                        )));
                    }
                }
            }
            aptos_framework::verify_module_metadata(m, self.0.get_features())
                .map_err(|err| Self::metadata_validation_error(&err.to_string()))?;
        }
        verifier::resource_groups::validate_resource_groups(session, modules)?;

        if !expected_modules.is_empty() {
            return Err(Self::metadata_validation_error(
                "not all registered modules published",
            ));
        }
        Ok(())
    }

    fn metadata_validation_error(msg: &str) -> VMError {
        PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
            .with_message(format!("metadata and code bundle mismatch: {}", msg))
            .finish(Location::Undefined)
    }

    fn make_standard_gas_meter(
        &self,
        balance: Gas,
        log_context: &AdapterLogSchema,
    ) -> Result<StandardGasMeter, VMStatus> {
        Ok(StandardGasMeter::new(
            self.0.get_gas_feature_version(),
            self.0.get_gas_parameters(log_context)?.clone(),
            self.0.get_storage_gas_parameters(log_context)?.clone(),
            balance,
        ))
    }

    fn execute_user_transaction_impl<S, G>(
        &self,
        storage: &S,
        txn: &SignatureCheckedTransaction,
        log_context: &AdapterLogSchema,
        gas_meter: &mut G,
    ) -> (VMStatus, TransactionOutputExt)
    where
        G: AptosGasMeter,
        S: MoveResolverExt + StateView,
    {
        // Revalidate the transaction.
        let resolver = self.0.new_move_resolver(storage);
        let mut session = self.0.new_session(&resolver, SessionId::txn(txn));
        if let Err(err) = self.validate_signature_checked_transaction(
            &mut session,
            storage,
            txn,
            false,
            log_context,
        ) {
            return discard_error_vm_status(err);
        };

        if self.0.get_gas_feature_version() >= 1 {
            // Create a new session so that the data cache is flushed.
            // This is to ensure we correctly charge for loading certain resources, even if they
            // have been previously cached in the prologue.
            //
            // TODO(Gas): Do this in a better way in the future, perhaps without forcing the data cache to be flushed.
            session = self.0.new_session(&resolver, SessionId::txn(txn));
        }

        let storage_gas_params = unwrap_or_discard!(self.0.get_storage_gas_parameters(log_context));
        let txn_data = TransactionMetadata::new(txn);

        // We keep track of whether any newly published modules are loaded into the Vm's loader
        // cache as part of executing transactions. This would allow us to decide whether the cache
        // should be flushed later.
        let mut new_published_modules_loaded = false;
        let result = match txn.payload() {
            payload @ TransactionPayload::Script(_)
            | payload @ TransactionPayload::EntryFunction(_) => self
                .execute_script_or_entry_function(
                    storage,
                    session,
                    gas_meter,
                    &txn_data,
                    payload,
                    log_context,
                    &mut new_published_modules_loaded,
                    &storage_gas_params.change_set_configs,
                ),
            TransactionPayload::Multisig(payload) => self.execute_multisig_transaction(
                storage,
                session,
                gas_meter,
                &txn_data,
                payload,
                log_context,
                &mut new_published_modules_loaded,
                &storage_gas_params.change_set_configs,
            ),

            // Deprecated. Will be removed in the future.
            TransactionPayload::ModuleBundle(m) => self.execute_modules(
                storage,
                session,
                gas_meter,
                &txn_data,
                m,
                log_context,
                &mut new_published_modules_loaded,
                &storage_gas_params.change_set_configs,
            ),
        };

        let gas_usage = txn_data
            .max_gas_amount()
            .checked_sub(gas_meter.balance())
            .expect("Balance should always be less than or equal to max gas amount set");
        TXN_GAS_USAGE.observe(u64::from(gas_usage) as f64);

        match result {
            Ok(output) => output,
            Err(err) => {
                // Invalidate the loader cache in case there was a new module loaded from a module
                // publish request that failed.
                // This ensures the loader cache is flushed later to align storage with the cache.
                // None of the modules in the bundle will be committed to storage,
                // but some of them may have ended up in the cache.
                if new_published_modules_loaded {
                    self.0.mark_loader_cache_as_invalid();
                };

                let txn_status = TransactionStatus::from(err.clone());
                if txn_status.is_discarded() {
                    discard_error_vm_status(err)
                } else {
                    self.failed_transaction_cleanup_and_keep_vm_status(
                        err,
                        gas_meter,
                        &txn_data,
                        storage,
                        log_context,
                        &storage_gas_params.change_set_configs,
                    )
                }
            },
        }
    }

    pub(crate) fn execute_user_transaction<S: MoveResolverExt + StateView>(
        &self,
        storage: &S,
        txn: &SignatureCheckedTransaction,
        log_context: &AdapterLogSchema,
    ) -> (VMStatus, TransactionOutputExt) {
        let balance = TransactionMetadata::new(txn).max_gas_amount();
        // TODO: would we end up having a diverging behavior by creating the gas meter at an earlier time?
        let mut gas_meter = unwrap_or_discard!(self.make_standard_gas_meter(balance, log_context));

        self.execute_user_transaction_impl(storage, txn, log_context, &mut gas_meter)
    }

    pub fn execute_user_transaction_with_custom_gas_meter<S, G, F>(
        state_view: &S,
        txn: &SignatureCheckedTransaction,
        log_context: &AdapterLogSchema,
        make_gas_meter: F,
    ) -> Result<(VMStatus, TransactionOutput, G), VMStatus>
    where
        S: StateView,
        G: AptosGasMeter,
        F: FnOnce(u64, AptosGasParameters, StorageGasParameters, Gas) -> Result<G, VMStatus>,
    {
        // TODO(Gas): revisit this.
        init_speculative_logs(1);

        let storage = StorageAdapter::new(state_view);
        let vm = AptosVM::new(&storage);

        // TODO(Gas): avoid creating txn metadata twice.
        let balance = TransactionMetadata::new(txn).max_gas_amount();
        let mut gas_meter = make_gas_meter(
            vm.0.get_gas_feature_version(),
            vm.0.get_gas_parameters(log_context)?.clone(),
            vm.0.get_storage_gas_parameters(log_context)?.clone(),
            balance,
        )?;

        let (status, output) =
            vm.execute_user_transaction_impl(&storage, txn, log_context, &mut gas_meter);

        Ok((status, output.into_transaction_output(&storage), gas_meter))
    }

    fn execute_writeset<S: MoveResolverExt>(
        &self,
        storage: &S,
        writeset_payload: &WriteSetPayload,
        txn_sender: Option<AccountAddress>,
        session_id: SessionId,
    ) -> Result<ChangeSetExt, Result<(VMStatus, TransactionOutputExt), VMStatus>> {
        let mut gas_meter = UnmeteredGasMeter;
        let change_set_configs =
            ChangeSetConfigs::unlimited_at_gas_feature_version(self.0.get_gas_feature_version());

        Ok(match writeset_payload {
            WriteSetPayload::Direct(change_set) => ChangeSetExt::new(
                DeltaChangeSet::empty(),
                change_set.clone(),
                Arc::new(change_set_configs),
            ),
            WriteSetPayload::Script { script, execute_as } => {
                let resolver = self.0.new_move_resolver(storage);
                let mut tmp_session = self.0.new_session(&resolver, session_id);
                let senders = match txn_sender {
                    None => vec![*execute_as],
                    Some(sender) => vec![sender, *execute_as],
                };

                let loaded_func = tmp_session
                    .load_script(script.code(), script.ty_args().to_vec())
                    .map_err(|e| Err(e.into_vm_status()))?;
                let args =
                    verifier::transaction_arg_validation::validate_combine_signer_and_txn_args(
                        &mut tmp_session,
                        senders,
                        convert_txn_args(script.args()),
                        &loaded_func,
                        self.0
                            .get_features()
                            .is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
                    )
                    .map_err(Err)?;

                tmp_session
                    .execute_script(
                        script.code(),
                        script.ty_args().to_vec(),
                        args,
                        &mut gas_meter,
                    )
                    .and_then(|_| tmp_session.finish(&mut (), &change_set_configs))
                    .map_err(|e| Err(e.into_vm_status()))?
            },
        })
    }

    fn read_writeset(
        &self,
        state_view: &impl StateView,
        write_set: &WriteSet,
    ) -> Result<(), VMStatus> {
        // All Move executions satisfy the read-before-write property. Thus we need to read each
        // access path that the write set is going to update.
        for (state_key, _) in write_set.iter() {
            state_view
                .get_state_value_bytes(state_key)
                .map_err(|_| VMStatus::Error(StatusCode::STORAGE_ERROR, None))?;
        }
        Ok(())
    }

    fn validate_waypoint_change_set(
        change_set: &ChangeSet,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let has_new_block_event = change_set
            .events()
            .iter()
            .any(|e| *e.key() == new_block_event_key());
        let has_new_epoch_event = change_set
            .events()
            .iter()
            .any(|e| *e.key() == new_epoch_event_key());
        if has_new_block_event && has_new_epoch_event {
            Ok(())
        } else {
            error!(
                *log_context,
                "[aptos_vm] waypoint txn needs to emit new epoch and block"
            );
            Err(VMStatus::Error(StatusCode::INVALID_WRITE_SET, None))
        }
    }

    pub(crate) fn process_waypoint_change_set<S: MoveResolverExt>(
        &self,
        storage: &S,
        writeset_payload: WriteSetPayload,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, TransactionOutputExt), VMStatus> {
        // TODO: user specified genesis id to distinguish different genesis write sets
        let genesis_id = HashValue::zero();
        let change_set_ext = match self.execute_writeset(
            storage,
            &writeset_payload,
            Some(aptos_types::account_config::reserved_vm_address()),
            SessionId::genesis(genesis_id),
        ) {
            Ok(cse) => cse,
            Err(e) => return e,
        };

        let (delta_change_set, change_set) = change_set_ext.into_inner();
        Self::validate_waypoint_change_set(&change_set, log_context)?;
        let (write_set, events) = change_set.into_inner();
        self.read_writeset(storage, &write_set)?;
        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let txn_output = TransactionOutput::new(write_set, events, 0, VMStatus::Executed.into());
        Ok((
            VMStatus::Executed,
            TransactionOutputExt::new(delta_change_set, txn_output),
        ))
    }

    pub(crate) fn process_block_prologue<S: MoveResolverExt>(
        &self,
        storage: &S,
        block_metadata: BlockMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, TransactionOutputExt), VMStatus> {
        fail_point!("move_adapter::process_block_prologue", |_| {
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        let txn_data = TransactionMetadata {
            sender: account_config::reserved_vm_address(),
            max_gas_amount: 0.into(),
            ..Default::default()
        };
        let mut gas_meter = UnmeteredGasMeter;
        let resolver = self.0.new_move_resolver(storage);
        let mut session = self
            .0
            .new_session(&resolver, SessionId::block_meta(&block_metadata));

        let args = serialize_values(&block_metadata.get_prologue_move_args(txn_data.sender));
        session
            .execute_function_bypass_visibility(
                &BLOCK_MODULE,
                BLOCK_PROLOGUE,
                vec![],
                args,
                &mut gas_meter,
            )
            .map(|_return_vals| ())
            .or_else(|e| {
                expect_only_successful_execution(e, BLOCK_PROLOGUE.as_str(), log_context)
            })?;
        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let output = get_transaction_output(
            &mut (),
            session,
            0.into(),
            &txn_data,
            ExecutionStatus::Success,
            &self
                .0
                .get_storage_gas_parameters(log_context)?
                .change_set_configs,
        )?;
        Ok((VMStatus::Executed, output))
    }

    pub fn simulate_signed_transaction(
        txn: &SignedTransaction,
        state_view: &impl StateView,
    ) -> (VMStatus, TransactionOutputExt) {
        let vm = AptosVM::new(state_view);
        let simulation_vm = AptosSimulationVM(vm);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        simulation_vm.simulate_signed_transaction(&state_view.as_move_resolver(), txn, &log_context)
    }

    pub fn execute_view_function(
        state_view: &impl StateView,
        module_id: ModuleId,
        func_name: Identifier,
        type_args: Vec<TypeTag>,
        arguments: Vec<Vec<u8>>,
        gas_budget: u64,
    ) -> Result<Vec<Vec<u8>>> {
        let vm = AptosVM::new(state_view);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        let mut gas_meter = StandardGasMeter::new(
            vm.0.get_gas_feature_version(),
            vm.0.get_gas_parameters(&log_context)?.clone(),
            vm.0.get_storage_gas_parameters(&log_context)?.clone(),
            gas_budget,
        );
        let resolver = &state_view.as_move_resolver();
        let resolver = vm.0.new_move_resolver(resolver);
        let mut session = vm.new_session(&resolver, SessionId::Void);

        let func_inst = session.load_function(&module_id, &func_name, &type_args)?;
        let metadata = vm.0.extract_module_metadata(&module_id);
        let arguments = verifier::view_function::validate_view_function(
            &mut session,
            arguments,
            func_name.as_ident_str(),
            &func_inst,
            metadata.as_ref(),
            vm.0.get_features()
                .is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS),
        )?;

        Ok(session
            .execute_function_bypass_visibility(
                &module_id,
                func_name.as_ident_str(),
                type_args,
                arguments,
                &mut gas_meter,
            )
            .map_err(|err| anyhow!("Failed to execute function: {:?}", err))?
            .return_values
            .into_iter()
            .map(|(bytes, _ty)| bytes)
            .collect::<Vec<_>>())
    }

    fn run_prologue_with_payload<S: MoveResolverExt, SS: MoveResolverExt>(
        &self,
        session: &mut SessionExt<SS>,
        storage: &S,
        payload: &TransactionPayload,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
        // Whether the prologue is run as part of tx simulation.
        is_simulation: bool,
    ) -> Result<(), VMStatus> {
        match payload {
            TransactionPayload::Script(_) => {
                self.0.check_gas(storage, txn_data, log_context)?;
                self.0.run_script_prologue(session, txn_data, log_context)
            },
            TransactionPayload::EntryFunction(_) => {
                // NOTE: Script and EntryFunction shares the same prologue
                self.0.check_gas(storage, txn_data, log_context)?;
                self.0.run_script_prologue(session, txn_data, log_context)
            },
            TransactionPayload::Multisig(multisig_payload) => {
                self.0.check_gas(storage, txn_data, log_context)?;
                // Still run script prologue for multisig transaction to ensure the same tx
                // validations are still run for this multisig execution tx, which is submitted by
                // one of the owners.
                self.0.run_script_prologue(session, txn_data, log_context)?;
                // Skip validation if this is part of tx simulation.
                // This allows simulating multisig txs without having to first create the multisig
                // tx.
                if !is_simulation {
                    self.0
                        .run_multisig_prologue(session, txn_data, multisig_payload, log_context)
                } else {
                    Ok(())
                }
            },

            // Deprecated. Will be removed in the future.
            TransactionPayload::ModuleBundle(_module) => {
                if MODULE_BUNDLE_DISALLOWED.load(Ordering::Relaxed) {
                    return Err(VMStatus::Error(StatusCode::FEATURE_UNDER_GATING, None));
                }
                self.0.check_gas(storage, txn_data, log_context)?;
                self.0.run_module_prologue(session, txn_data, log_context)
            },
        }
    }
}

// Executor external API
impl VMExecutor for AptosVM {
    /// Execute a block of `transactions`. The output vector will have the exact same length as the
    /// input vector. The discarded transactions will be marked as `TransactionStatus::Discard` and
    /// have an empty `WriteSet`. Also `state_view` is immutable, and does not have interior
    /// mutability. Writes to be applied to the data view are encoded in the write set part of a
    /// transaction output.
    fn execute_block(
        transactions: Vec<Transaction>,
        state_view: &(impl StateView + Sync),
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        fail_point!("move_adapter::execute_block", |_| {
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        info!(
            log_context,
            "Executing block, transaction count: {}",
            transactions.len()
        );

        let count = transactions.len();
        let ret =
            BlockAptosVM::execute_block(transactions, state_view, Self::get_concurrency_level());
        if ret.is_ok() {
            // Record the histogram count for transactions per block.
            BLOCK_TRANSACTION_COUNT.observe(count as f64);
        }
        ret
    }
}

// VMValidator external API
impl VMValidator for AptosVM {
    /// Determine if a transaction is valid. Will return `None` if the transaction is accepted,
    /// `Some(Err)` if the VM rejects it, with `Err` as an error code. Verification performs the
    /// following steps:
    /// 1. The signature on the `SignedTransaction` matches the public key included in the
    ///    transaction
    /// 2. The script to be executed is under given specific configuration.
    /// 3. Invokes `Account.prologue`, which checks properties such as the transaction has the
    /// right sequence number and the sender has enough balance to pay for the gas.
    /// TBD:
    /// 1. Transaction arguments matches the main function's type signature.
    ///    We don't check this item for now and would execute the check at execution time.
    fn validate_transaction(
        &self,
        transaction: SignedTransaction,
        state_view: &impl StateView,
    ) -> VMValidatorResult {
        let _timer = TXN_VALIDATION_SECONDS.start_timer();
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        let txn = match Self::check_signature(transaction) {
            Ok(t) => t,
            _ => {
                return VMValidatorResult::error(StatusCode::INVALID_SIGNATURE);
            },
        };

        let inner_resolver = &state_view.as_move_resolver();
        let resolver = self.0.new_move_resolver(inner_resolver);
        let mut session = self.new_session(&resolver, SessionId::txn(&txn));
        let validation_result = self.validate_signature_checked_transaction(
            &mut session,
            &resolver,
            &txn,
            true,
            &log_context,
        );

        // Increment the counter for transactions verified.
        let (counter_label, result) = match validation_result {
            Ok(_) => (
                "success",
                VMValidatorResult::new(None, txn.gas_unit_price()),
            ),
            Err(err) => (
                "failure",
                VMValidatorResult::new(Some(err.status_code()), 0),
            ),
        };

        TRANSACTIONS_VALIDATED
            .with_label_values(&[counter_label])
            .inc();

        result
    }
}

impl VMAdapter for AptosVM {
    fn new_session<'r, R: MoveResolverExt>(
        &self,
        remote: &'r R,
        session_id: SessionId,
    ) -> SessionExt<'r, '_, R> {
        self.0.new_session(remote, session_id)
    }

    fn check_signature(txn: SignedTransaction) -> Result<SignatureCheckedTransaction> {
        txn.check_signature()
    }

    fn check_transaction_format(&self, txn: &SignedTransaction) -> Result<(), VMStatus> {
        if txn.contains_duplicate_signers() {
            return Err(VMStatus::Error(
                StatusCode::SIGNERS_CONTAIN_DUPLICATES,
                None,
            ));
        }

        Ok(())
    }

    fn run_prologue<S: MoveResolverExt, SS: MoveResolverExt>(
        &self,
        session: &mut SessionExt<SS>,
        storage: &S,
        transaction: &SignatureCheckedTransaction,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let txn_data = TransactionMetadata::new(transaction);
        self.run_prologue_with_payload(
            session,
            storage,
            transaction.payload(),
            &txn_data,
            log_context,
            false,
        )
    }

    fn should_restart_execution(vm_output: &TransactionOutput) -> bool {
        let new_epoch_event_key = aptos_types::on_chain_config::new_epoch_event_key();
        vm_output
            .events()
            .iter()
            .any(|event| *event.key() == new_epoch_event_key)
    }

    fn execute_single_transaction<S: MoveResolverExt>(
        &self,
        txn: &PreprocessedTransaction,
        data_cache: &S,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, TransactionOutputExt, Option<String>), VMStatus> {
        Ok(match txn {
            PreprocessedTransaction::BlockMetadata(block_metadata) => {
                fail_point!("aptos_vm::execution::block_metadata");
                let (vm_status, output) =
                    self.process_block_prologue(data_cache, block_metadata.clone(), log_context)?;
                (vm_status, output, Some("block_prologue".to_string()))
            },
            PreprocessedTransaction::WaypointWriteSet(write_set_payload) => {
                let (vm_status, output) = self.process_waypoint_change_set(
                    data_cache,
                    write_set_payload.clone(),
                    log_context,
                )?;
                (vm_status, output, Some("waypoint_write_set".to_string()))
            },
            PreprocessedTransaction::UserTransaction(txn) => {
                fail_point!("aptos_vm::execution::user_transaction");
                let sender = txn.sender().to_string();
                let _timer = TXN_TOTAL_SECONDS.start_timer();
                let (vm_status, output) =
                    self.execute_user_transaction(data_cache, txn, log_context);

                if let Err(DiscardedVMStatus::UNKNOWN_INVARIANT_VIOLATION_ERROR) =
                    vm_status.clone().keep_or_discard()
                {
                    error!(
                        *log_context,
                        "[aptos_vm] Transaction breaking invariant violation. txn: {:?}",
                        bcs::to_bytes::<SignedTransaction>(&**txn),
                    );
                    TRANSACTIONS_INVARIANT_VIOLATION.inc();
                }

                // Increment the counter for user transactions executed.
                let counter_label = match output.txn_output().status() {
                    TransactionStatus::Keep(_) => Some("success"),
                    TransactionStatus::Discard(_) => Some("discarded"),
                    TransactionStatus::Retry => None,
                };
                if let Some(label) = counter_label {
                    USER_TRANSACTIONS_EXECUTED.with_label_values(&[label]).inc();
                }
                (vm_status, output, Some(sender))
            },
            PreprocessedTransaction::InvalidSignature => {
                let (vm_status, output) =
                    discard_error_vm_status(VMStatus::Error(StatusCode::INVALID_SIGNATURE, None));
                (vm_status, output, None)
            },
            PreprocessedTransaction::StateCheckpoint => {
                let output = TransactionOutput::new(
                    WriteSet::default(),
                    Vec::new(),
                    0,
                    TransactionStatus::Keep(ExecutionStatus::Success),
                );
                (
                    VMStatus::Executed,
                    TransactionOutputExt::from(output),
                    Some("state_checkpoint".into()),
                )
            },
        })
    }
}

impl AsRef<AptosVMImpl> for AptosVM {
    fn as_ref(&self) -> &AptosVMImpl {
        &self.0
    }
}

impl AsMut<AptosVMImpl> for AptosVM {
    fn as_mut(&mut self) -> &mut AptosVMImpl {
        &mut self.0
    }
}

impl AptosSimulationVM {
    fn validate_simulated_transaction<S: MoveResolverExt, SS: MoveResolverExt>(
        &self,
        session: &mut SessionExt<SS>,
        storage: &S,
        transaction: &SignedTransaction,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        self.0.check_transaction_format(transaction)?;
        self.0.run_prologue_with_payload(
            session,
            storage,
            transaction.payload(),
            txn_data,
            log_context,
            true,
        )
    }

    /*
    Executes a SignedTransaction without performing signature verification
     */
    fn simulate_signed_transaction<S: MoveResolverExt>(
        &self,
        storage: &S,
        txn: &SignedTransaction,
        log_context: &AdapterLogSchema,
    ) -> (VMStatus, TransactionOutputExt) {
        // simulation transactions should not carry valid signatures, otherwise malicious fullnodes
        // may execute them without user's explicit permission.
        if txn.signature_is_valid() {
            return discard_error_vm_status(VMStatus::Error(StatusCode::INVALID_SIGNATURE, None));
        }

        // Revalidate the transaction.
        let txn_data = TransactionMetadata::new(txn);
        let resolver = self.0 .0.new_move_resolver(storage);
        let mut session = self
            .0
            .new_session(&resolver, SessionId::txn_meta(&txn_data));
        if let Err(err) =
            self.validate_simulated_transaction(&mut session, storage, txn, &txn_data, log_context)
        {
            return discard_error_vm_status(err);
        };

        let gas_params = match self.0 .0.get_gas_parameters(log_context) {
            Err(err) => return discard_error_vm_status(err),
            Ok(s) => s,
        };
        let storage_gas_params = match self.0 .0.get_storage_gas_parameters(log_context) {
            Err(err) => return discard_error_vm_status(err),
            Ok(s) => s,
        };

        let mut gas_meter = StandardGasMeter::new(
            self.0 .0.get_gas_feature_version(),
            gas_params.clone(),
            storage_gas_params.clone(),
            txn_data.max_gas_amount(),
        );

        let mut new_published_modules_loaded = false;
        let result = match txn.payload() {
            payload @ TransactionPayload::Script(_)
            | payload @ TransactionPayload::EntryFunction(_) => {
                self.0.execute_script_or_entry_function(
                    storage,
                    session,
                    &mut gas_meter,
                    &txn_data,
                    payload,
                    log_context,
                    &mut new_published_modules_loaded,
                    &storage_gas_params.change_set_configs,
                )
            },
            TransactionPayload::Multisig(multisig) => {
                if let Some(payload) = multisig.transaction_payload.clone() {
                    match payload {
                        MultisigTransactionPayload::EntryFunction(entry_function) => {
                            self.0
                                .execute_multisig_entry_function(
                                    &mut session,
                                    &mut gas_meter,
                                    multisig.multisig_address,
                                    &entry_function,
                                    &mut new_published_modules_loaded,
                                )
                                .and_then(|_| {
                                    // TODO: Deduplicate this against execute_multisig_transaction
                                    // A bit tricky since we need to skip success/failure cleanups,
                                    // which is in the middle. Introducing a boolean would make the code
                                    // messier.
                                    let change_set_ext = session
                                        .finish(&mut (), &storage_gas_params.change_set_configs)
                                        .map_err(|e| e.into_vm_status())?;
                                    gas_meter.charge_io_gas_for_write_set(
                                        change_set_ext.write_set().iter(),
                                    )?;
                                    gas_meter.charge_storage_fee_for_all(
                                        change_set_ext.write_set().iter(),
                                        change_set_ext.change_set().events(),
                                        txn_data.transaction_size,
                                        txn_data.gas_unit_price,
                                    )?;
                                    self.0.success_transaction_cleanup(
                                        storage,
                                        change_set_ext,
                                        &mut gas_meter,
                                        &txn_data,
                                        log_context,
                                        &storage_gas_params.change_set_configs,
                                    )
                                })
                        },
                    }
                } else {
                    Err(VMStatus::Error(StatusCode::MISSING_DATA, None))
                }
            },

            // Deprecated. Will be removed in the future.
            TransactionPayload::ModuleBundle(m) => self.0.execute_modules(
                storage,
                session,
                &mut gas_meter,
                &txn_data,
                m,
                log_context,
                &mut new_published_modules_loaded,
                &storage_gas_params.change_set_configs,
            ),
        };

        match result {
            Ok(output) => output,
            Err(err) => {
                // Invalidate the loader cache in case there was a new module loaded from a module
                // publish request that failed.
                // This ensures the loader cache is flushed later to align storage with the cache.
                // None of the modules in the bundle will be committed to storage,
                // but some of them may have ended up in the cache.
                if new_published_modules_loaded {
                    self.0 .0.mark_loader_cache_as_invalid();
                };
                let txn_status = TransactionStatus::from(err.clone());
                if txn_status.is_discarded() {
                    discard_error_vm_status(err)
                } else {
                    let (vm_status, output) = self.0.failed_transaction_cleanup_and_keep_vm_status(
                        err,
                        &mut gas_meter,
                        &txn_data,
                        storage,
                        log_context,
                        &storage_gas_params.change_set_configs,
                    );
                    (vm_status, output)
                }
            },
        }
    }
}
