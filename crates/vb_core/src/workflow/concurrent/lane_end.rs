use super::concurrent_pair::{at, validate_concurrent_pair};
use crate::ids::SlotIdx;
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind as K, WorkflowError, WorkflowParts};

fn derive_join(parts: &WorkflowParts, owner: StepIdx) -> Result<StepIdx, WorkflowError> {
    let node = at(parts, owner)?;
    match node.kind {
        K::ForEachConcurrentStart { join, .. }
        | K::TogetherConcurrentStart { join, .. }
        | K::FirstSuccessStart { join, .. } => Ok(join),
        _ => Err(WorkflowError::LoopRegionUnpaired { control: owner }),
    }
}

fn check_bounds(owner: StepIdx, end: StepIdx, join: StepIdx) -> Result<(), WorkflowError> {
    if owner.as_usize() < end.as_usize() && end.as_usize() < join.as_usize() {
        Ok(())
    } else {
        Err(WorkflowError::LoopRegionCrossing {
            control: owner,
            from: end,
            to: join,
        })
    }
}

fn check_routing(end: &CompiledNode, join: StepIdx) -> Result<(), WorkflowError> {
    if end.routing.next.step() == Some(join) {
        Ok(())
    } else {
        Err(WorkflowError::LoopRegionUnpaired { control: end.id })
    }
}

fn check_slot(parts: &WorkflowParts, result: Option<SlotIdx>) -> Result<(), WorkflowError> {
    if let Some(slot) = result
        && slot.get() >= parts.slot_count
    {
        return Err(WorkflowError::SlotOutOfBounds { slot });
    }
    Ok(())
}

pub(super) fn validate(parts: &WorkflowParts, end: &CompiledNode) -> Result<(), WorkflowError> {
    let owner = match end.kind {
        K::ConcurrentLaneEnd { owner, result } => {
            if end.output.slot().is_some() || end.retry.is_some() {
                return Err(WorkflowError::LoopRegionUnpaired { control: end.id });
            }
            validate_concurrent_pair(parts, owner)?;
            check_slot(parts, result)?;
            owner
        }
        _ => return Err(WorkflowError::LoopRegionUnpaired { control: end.id }),
    };
    let join = derive_join(parts, owner)?;
    check_bounds(owner, end.id, join)?;
    check_routing(end, join)?;
    check_error_routes(parts, end, owner, join)
}

fn check_error_routes(
    parts: &WorkflowParts,
    end: &CompiledNode,
    owner: StepIdx,
    join: StepIdx,
) -> Result<(), WorkflowError> {
    for target in super::concurrent_targets::targets(parts, end)? {
        if target <= owner || target >= join {
            return Err(WorkflowError::LoopRegionCrossing {
                control: owner,
                from: end.id,
                to: target,
            });
        }
    }
    Ok(())
}
