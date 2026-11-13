//! Validate one concurrent-source/join pair (loop-region boundary).

use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind as K, WorkflowError, WorkflowParts};

/// Validate that `start` is a well-formed concurrent-source node paired with
/// its join step. Requires the join to reference `start`, to have a bounded
/// output slot, and to route forward to an in-bounds next step.
pub(super) fn validate_concurrent_pair(
    parts: &WorkflowParts,
    start: StepIdx,
) -> Result<(), WorkflowError> {
    let src = at(parts, start)?;
    if src.output.slot().is_some() {
        return Err(WorkflowError::LoopRegionUnpaired { control: start });
    }
    if src.routing.next.step().is_some() {
        return Err(WorkflowError::LoopRegionUnpaired { control: start });
    }
    if src.retry.is_some() {
        return Err(WorkflowError::LoopRegionUnpaired { control: start });
    }
    let join = match src.kind {
        K::ForEachConcurrentStart { join, .. }
        | K::TogetherConcurrentStart { join, .. }
        | K::FirstSuccessStart { join, .. } => join,
        _ => return Err(WorkflowError::LoopRegionUnpaired { control: start }),
    };
    check_join_direction(start, join)?;
    check_output(parts, join)?;
    check_continuation(parts, start, join)
}

fn check_output(parts: &WorkflowParts, join: StepIdx) -> Result<(), WorkflowError> {
    let j = at(parts, join)?;
    let slot = j
        .output
        .slot()
        .ok_or(WorkflowError::LoopRegionUnpaired { control: join })?;
    if slot.get() >= parts.slot_count {
        return Err(WorkflowError::SlotOutOfBounds { slot });
    }
    Ok(())
}

fn check_continuation(
    parts: &WorkflowParts,
    owner: StepIdx,
    join: StepIdx,
) -> Result<(), WorkflowError> {
    let j = at(parts, join)?;
    if !matches!(j.kind, K::ConcurrentGroupJoin { owner: o } if o == owner) {
        return Err(WorkflowError::LoopRegionUnpaired { control: join });
    }
    let next = j
        .routing
        .next
        .step()
        .ok_or(WorkflowError::LoopRegionUnpaired { control: join })?;
    if next.as_usize() <= join.as_usize() {
        return Err(WorkflowError::BackwardEdge {
            from: join,
            to: next,
        });
    }
    if next.as_usize() >= parts.nodes.len() {
        return Err(WorkflowError::StepOutOfBounds { step: next });
    }
    Ok(())
}

pub(super) fn at(parts: &WorkflowParts, step: StepIdx) -> Result<&CompiledNode, WorkflowError> {
    parts
        .nodes
        .get(step.as_usize())
        .ok_or(WorkflowError::StepOutOfBounds { step })
        .and_then(|node| {
            if node.id != step {
                Err(WorkflowError::NodeIdMismatch {
                    expected: step,
                    actual: node.id,
                })
            } else {
                Ok(node)
            }
        })
}

fn check_join_direction(owner: StepIdx, join: StepIdx) -> Result<(), WorkflowError> {
    if join.as_usize() <= owner.as_usize() {
        Err(WorkflowError::BackwardEdge {
            from: owner,
            to: join,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "pair_tests.rs"]
mod tests;
