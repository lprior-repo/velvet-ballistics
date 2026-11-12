use crate::{RuntimeError, RuntimeResult};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{CompiledWorkflow, RunFrame, ValueStore};

pub(super) fn restore(
    checkpoint: Option<ValueStoreCheckpointV1>,
    frame: &RunFrame,
    workflow: &CompiledWorkflow,
    is_retry: bool,
) -> RuntimeResult<Option<ValueStore>> {
    let Some(checkpoint) = checkpoint else {
        return Ok(None);
    };
    if is_retry {
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    vb_core::root_bundle::validate_runtime_store(frame, &checkpoint, workflow)
        .map_err(|_| RuntimeError::InvalidRecoveryHydration)?;
    ValueStore::from_checkpoint_v1(checkpoint)
        .map(Some)
        .map_err(Into::into)
}
