use super::facts::{self, invalid};
use super::{Fact, Facts, FlowResult, SlotFlowError, SlotFlowFailure, capacity, failure};
use crate::ids::StepIdx;
use crate::workflow::WorkflowError;

mod check;
mod dispatch;
mod engine;
mod fork;
mod layout;
mod store;
#[path = "../concurrent_values.rs"]
mod values;
mod writers;

fn scratch<T>(count: usize) -> FlowResult<Vec<T>> {
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(|_| capacity())?;
    Ok(result)
}
fn slots<T: Clone>(count: usize, value: T) -> FlowResult<Vec<T>> {
    let mut result = scratch(count)?;
    result.resize(count, value);
    Ok(result)
}
fn model_error(error: WorkflowError, entry: StepIdx) -> SlotFlowError {
    match error {
        WorkflowError::SlotFlow(error) => error,
        WorkflowError::StepOutOfBounds { step } => invalid(step),
        WorkflowError::NodeIdMismatch { actual, .. } => invalid(actual),
        WorkflowError::SlotOutOfBounds { slot } => {
            failure(entry, Some(slot), SlotFlowFailure::InvalidReference)
        }
        WorkflowError::ResourceContractExceeded { .. } => capacity(),
        WorkflowError::LoopRegionUnpaired { control }
        | WorkflowError::LoopRegionUnsupported { control, .. }
        | WorkflowError::LoopRegionCrossing { control, .. } => invalid(control),
        WorkflowError::BackwardEdge { from, .. } => invalid(from),
        _ => failure(entry, None, SlotFlowFailure::UnsupportedSemantics),
    }
}

pub(super) fn validate(parts: &crate::workflow::WorkflowParts) -> FlowResult<()> {
    if parts.nodes.len() > crate::limits::MAX_STEPS_PER_WORKFLOW {
        return Err(capacity());
    }
    let regions = crate::workflow::concurrent_analyze::analyze(parts)
        .map_err(|e| model_error(e, parts.entry))?;
    engine::validate(parts, &regions)
}
