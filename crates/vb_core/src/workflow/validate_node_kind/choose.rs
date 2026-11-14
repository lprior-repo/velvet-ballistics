use crate::workflow::validate_refs::{
    validate_branch_route, validate_expr, validate_optional_step, validate_slot, validate_step,
};
use crate::workflow::{BranchContinuation, ExprBranch, SlotBranch, WorkflowError, WorkflowParts};

fn validate_optional_step_continuation(
    continuation: BranchContinuation,
    node_count: usize,
) -> Result<(), WorkflowError> {
    validate_optional_step(continuation.step(), node_count)
}

pub(super) fn validate_slot_choose(
    branches: &[SlotBranch],
    otherwise: BranchContinuation,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise.step())?;
    branches.iter().try_for_each(|branch| {
        validate_slot(branch.condition, parts.slot_count)?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step_continuation(otherwise, parts.nodes.len())
}

pub(super) fn validate_expr_choose(
    branches: &[ExprBranch],
    otherwise: BranchContinuation,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise.step())?;
    branches.iter().try_for_each(|branch| {
        validate_expr(branch.condition, parts.expressions.len())?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step_continuation(otherwise, parts.nodes.len())
}
