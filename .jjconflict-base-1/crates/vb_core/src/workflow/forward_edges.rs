#![forbid(unsafe_code)]
//! Forward-edge and loop-span validation.

use super::repeat_check_validate;
use super::{
    BranchContinuation, CompiledNodeKind, ContractResource, ExprBranch, SlotBranch, WorkflowError,
    WorkflowParts,
};
use crate::ids::StepIdx;

/// Check B: all edges must point forward except recognized loop back-edges.
/// Check D: loop spans must be properly nested (no overlapping loops).
pub(super) fn validate_forward_edges(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let mut loop_spans: Vec<(usize, usize)> = Vec::new();

    for (index, node) in parts.nodes.iter().enumerate() {
        let current_id = StepIdx::new(u16::try_from(index).map_err(|_| {
            WorkflowError::ResourceContractExceeded {
                resource: ContractResource::Steps,
            }
        })?);

        if let Some(next) = node.routing.next.step() {
            if matches!(node.kind, CompiledNodeKind::RepeatCheck { .. }) && next.as_usize() <= index
            {
                repeat_check_validate::validate(parts, index, node, next)?;
            } else {
                validate_forward_target(next, index, current_id)?;
            }
        }

        if let Some(handler) = node.routing.error.handler() {
            validate_forward_target(handler, index, current_id)?;
        }

        validate_kind_edges(&node.kind, index, current_id)?;

        push_loop_span(&node.kind, index, &mut loop_spans)?;
    }
    Ok(())
}

/// Validates that kind-specific edges respect the forward-only rule.
fn validate_kind_edges(
    kind: &CompiledNodeKind,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::Finish { .. } => Ok(()),
        CompiledNodeKind::Jump { target } => validate_forward_target(*target, ci, cid),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => validate_choose_slot_edges(branches, otherwise, ci, cid),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => validate_choose_expr_edges(branches, otherwise, ci, cid),
        CompiledNodeKind::ForEachStart { body, done, .. } => {
            validate_loop_start_edges(*body, *done, ci, cid)
        }
        CompiledNodeKind::ForEachNext { done, .. } => validate_loop_done_only(*done, ci, cid),
        CompiledNodeKind::TogetherStart { branches, join } => {
            validate_together_start_edges(branches, *join, ci, cid)
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            validate_together_branch_edges(*entry, *join, ci, cid)
        }
        CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. } => {
            validate_loop_start_edges(*body, *done, ci, cid)
        }
        CompiledNodeKind::CollectPage { done, .. }
        | CompiledNodeKind::CollectNext { done, .. }
        | CompiledNodeKind::ReduceNext { done, .. }
        | CompiledNodeKind::RepeatAttempt { done, .. } => validate_loop_done_only(*done, ci, cid),
        CompiledNodeKind::RepeatCheck { done, .. } => validate_forward_target(*done, ci, cid),
        CompiledNodeKind::RetryCheck { exhausted, .. } => {
            validate_loop_done_only(*exhausted, ci, cid)
        }
        CompiledNodeKind::ErrorHandler { handler, .. } => {
            validate_loop_done_only(*handler, ci, cid)
        }
    }
}

fn validate_choose_slot_edges(
    branches: &[SlotBranch],
    otherwise: &BranchContinuation,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = otherwise.step() {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_choose_expr_edges(
    branches: &[ExprBranch],
    otherwise: &BranchContinuation,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = otherwise.step() {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_loop_done_only(done: StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(done, ci, cid)
}

/// Validates that both `body` and `done` edges of a `*Start` loop node point
/// strictly forward from the current node. `body` here is the entry into the
/// loop body (a forward edge); only `*Next` carries the back-edge in its
/// `body` slot, so `*Start` must require `body` to be forward.
fn validate_loop_start_edges(
    body: StepIdx,
    done: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    validate_forward_target(body, ci, cid)?;
    validate_forward_target(done, ci, cid)
}

fn validate_together_start_edges(
    branches: &[StepIdx],
    join: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(*branch, ci, cid)?;
    }
    validate_forward_target(join, ci, cid)
}

fn validate_together_branch_edges(
    entry: StepIdx,
    join: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    validate_forward_target(entry, ci, cid)?;
    validate_forward_target(join, ci, cid)
}

/// Validates a target step is strictly forward from the current node.
fn validate_forward_target(target: StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    if target.as_usize() > ci {
        Ok(())
    } else {
        Err(WorkflowError::BackwardEdge {
            from: cid,
            to: target,
        })
    }
}

/// Tracks loop spans for nesting validation (Check D).
fn push_loop_span(
    kind: &CompiledNodeKind,
    ci: usize,
    spans: &mut Vec<(usize, usize)>,
) -> Result<(), WorkflowError> {
    let done_usize: Option<usize> = match kind {
        CompiledNodeKind::ForEachStart { done, .. }
        | CompiledNodeKind::CollectStart { done, .. }
        | CompiledNodeKind::ReduceStart { done, .. }
        | CompiledNodeKind::RepeatStart { done, .. } => Some(done.as_usize()),
        CompiledNodeKind::TogetherStart { join, .. } => Some(join.as_usize()),
        _ => None,
    };

    let Some(done_idx) = done_usize else {
        return Ok(());
    };

    // Retire finished spans BEFORE the nesting comparison so that
    // sequential loops are not rejected as improperly nested.
    while spans
        .last()
        .is_some_and(|&(_, done): &(usize, usize)| done <= ci)
    {
        spans.pop();
    }

    match spans.last().copied() {
        Some((_outer_start, outer_done)) if done_idx > outer_done => {
            return Err(WorkflowError::ImproperLoopNesting {
                inner: StepIdx::new(u16::try_from(ci).map_err(|_| {
                    WorkflowError::ResourceContractExceeded {
                        resource: ContractResource::Steps,
                    }
                })?),
                outer_done: StepIdx::new(u16::try_from(outer_done).map_err(|_| {
                    WorkflowError::ResourceContractExceeded {
                        resource: ContractResource::Steps,
                    }
                })?),
            });
        }
        _ => {}
    }

    spans.push((ci, done_idx));
    Ok(())
}
