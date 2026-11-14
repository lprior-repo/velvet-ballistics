use crate::ids::StepIdx;
use crate::workflow::{ExprBranch, SlotBranch, WorkflowError};

pub(super) fn validate_branch_targets(
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

pub(super) fn validate_slot_branch_targets(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

pub(super) fn validate_two_step_targets(
    first: StepIdx,
    second: StepIdx,
    node_count: usize,
) -> Result<(), WorkflowError> {
    if first.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: first });
    }
    if second.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: second });
    }
    Ok(())
}
