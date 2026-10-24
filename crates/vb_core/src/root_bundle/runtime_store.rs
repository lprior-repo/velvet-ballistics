//! Structural validation of a runtime arena, including retired allocations.

use super::{OwnedRootBundleError, graph, taint, validation};
use crate::frame::RunFrame;
use crate::value_store::{ValueStoreCheckpointV1, validate_value_reference};
use crate::workflow::CompiledWorkflow;

/// Checks all allocations and actual initialized frame roots against accepted caps.
///
/// Unlike input construction, runtime validation permits unreachable retired pages.
/// This establishes structural validity, not execution or durable provenance.
pub fn validate_runtime_store(
    frame: &RunFrame,
    checkpoint: &ValueStoreCheckpointV1,
    workflow: &CompiledWorkflow,
) -> Result<(), OwnedRootBundleError> {
    validation::preflight(checkpoint, workflow)?;
    let roots = frame
        .initialized_slots()
        .map_err(OwnedRootBundleError::Checkpoint)?;
    for (_, value, _) in &roots {
        validate_value_reference(*value, checkpoint).map_err(OwnedRootBundleError::Checkpoint)?;
    }
    let graph = graph::validate_runtime(checkpoint)?;
    taint::validate(&roots, &graph)
}
