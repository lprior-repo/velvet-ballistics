use super::*;
use crate::prepared_inputs::{PreparedRuntimeInputs, prepare_bound_runtime_inputs};
use vb_core::value_store::InputSymbolBindingError;
use vb_storage::OwnedRunInputsRecordV1;

pub(super) fn prepare_scalar_roots(
    workflow: &CompiledWorkflow,
    roots: Vec<(SlotIdx, SlotValue, Taint)>,
) -> RuntimeResult<RecoveredRetainedInputs> {
    if workflow.symbol_payloads().is_none() {
        return Ok(RecoveredRetainedInputs {
            initial_inputs: roots,
            store: None,
        });
    }
    let checkpoint =
        ValueStore::with_max_slots(workflow.resource_contract().max_slots).checkpoint_v1();
    let prepared = prepare_bound_runtime_inputs(workflow, &checkpoint, &roots)
        .map_err(composition_error)?
        .ok_or(RuntimeError::InvalidRecoveryHydration)?;
    Ok(from_prepared(prepared))
}

fn from_prepared(prepared: PreparedRuntimeInputs) -> RecoveredRetainedInputs {
    RecoveredRetainedInputs {
        initial_inputs: prepared.roots,
        store: Some(prepared.store),
    }
}

fn composition_error(error: InputSymbolBindingError) -> RuntimeError {
    RuntimeError::Recovery {
        error: format!("bound input symbol composition failed: {error:?}"),
    }
}

pub(super) fn validate_owned_identity(
    record: &OwnedRunInputsRecordV1,
    run: RunId,
    admission: &JournalEvent,
) -> RuntimeResult<()> {
    if record.run() != run {
        return Err(RuntimeError::Recovery {
            error: format!(
                "recovery: retained owned input run mismatch: expected {run:?}, got {:?}",
                record.run()
            ),
        });
    }
    record
        .validate_admission(admission)
        .map_err(|error| RuntimeError::Recovery {
            error: format!("recovery: retained owned input admission binding invalid: {error}"),
        })
}

pub(super) fn prepare_owned_record(
    record: &OwnedRunInputsRecordV1,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<RecoveredRetainedInputs> {
    match prepare_bound_runtime_inputs(workflow, record.checkpoint(), record.roots())
        .map_err(composition_error)?
    {
        Some(prepared) => Ok(from_prepared(prepared)),
        None => restore_legacy_owned(record),
    }
}

fn restore_legacy_owned(record: &OwnedRunInputsRecordV1) -> RuntimeResult<RecoveredRetainedInputs> {
    let store = ValueStore::from_checkpoint_v1(record.checkpoint().clone()).map_err(|error| {
        RuntimeError::Recovery {
            error: format!("recovery: retained value store restore failed: {error:?}"),
        }
    })?;
    Ok(RecoveredRetainedInputs {
        initial_inputs: record.roots().to_vec(),
        store: Some(store),
    })
}
