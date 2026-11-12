use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{CompiledWorkflow, ValueStore};

use crate::run_checkpoint::{RunCheckpointV1, validate_bound_symbol_prefix};
use crate::{RuntimeError, RuntimeResult};

pub(super) enum RecoveredStateSource {
    Retry(Box<RunCheckpointV1>),
    Store(ValueStore),
}

pub(super) fn decode_workflow(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> RuntimeResult<CompiledWorkflow> {
    vb_storage::decode_compiled_ir(&artifact.ir, Some(artifact.artifact_version)).map_err(|error| {
        match error {
            vb_storage::JournalError::WorkflowReconstruction(error) => RuntimeError::Recovery {
                error: format!("workflow compile failed: {error}"),
            },
            _ => RuntimeError::Recovery {
                error: "artifact IR decode failed".to_string(),
            },
        }
    })
}

pub(super) fn prepare(
    checkpoint: Option<RunCheckpointV1>,
    restored: Option<ValueStoreCheckpointV1>,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<RecoveredStateSource> {
    match checkpoint {
        Some(checkpoint) => Ok(RecoveredStateSource::Retry(Box::new(checkpoint))),
        None => restore_store(restored, workflow).map(RecoveredStateSource::Store),
    }
}

fn restore_store(
    restored: Option<ValueStoreCheckpointV1>,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<ValueStore> {
    match restored {
        Some(checkpoint) => validate_full_store(checkpoint, workflow),
        None => Ok(ValueStore::with_max_slots(
            workflow.resource_contract().max_slots,
        )),
    }
}

fn validate_full_store(
    checkpoint: ValueStoreCheckpointV1,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<ValueStore> {
    let store = ValueStore::from_checkpoint_v1(checkpoint.clone()).map_err(|error| {
        RuntimeError::Recovery {
            error: format!("restored value store failed validation: {error:?}"),
        }
    })?;
    validate_bound_symbol_prefix(workflow, &checkpoint).map_err(|error| {
        RuntimeError::Recovery {
            error: format!("restored value store failed validation: {error}"),
        }
    })?;
    Ok(store)
}
