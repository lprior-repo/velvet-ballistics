#![forbid(unsafe_code)]
//! Forward-edge and loop-nesting validation.
//!
//! Check B: all edges must point forward except recognized loop back-edges.
//! Check D: loop spans must be properly nested (no overlapping loops).

use crate::ids::StepIdx;
use crate::workflow::{CompiledNodeKind, WorkflowError, WorkflowParts};

/// Validates forward-edge ordering and loop-nesting invariants.
pub(crate) fn validate_forward_edges(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let mut loop_spans: Vec<(usize, usize)> = Vec::with_capacity(parts.nodes.len());

    for (index, node) in parts.nodes.iter().enumerate() {
        let current_id = StepIdx::new(u16::try_from(index).map_err(|_| {
            WorkflowError::ResourceContractExceeded {
                resource: "max_steps",
            }
        })?);

        if let Some(next) = node.next {
            validate_forward_target(next, index, current_id)?;
        }

        if let Some(handler) = node.on_error {
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
        | CompiledNodeKind::Finish { .. }
        | CompiledNodeKind::Jump { .. } => Ok(()),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => validate_choose_slot_edges(branches, otherwise, ci, cid),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => validate_choose_expr_edges(branches, otherwise, ci, cid),
        CompiledNodeKind::ForEachStart { body, done, .. } => {
            validate_loop_done_only(*body, *done, ci, cid)
        }
        CompiledNodeKind::ForEachNext { body, done, .. } => {
            validate_loop_done_only(*body, *done, ci, cid)
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            validate_together_start_edges(branches, *join, ci, cid)
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            validate_together_branch_edges(*entry, *join, ci, cid)
        }
        CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            validate_loop_done_only(*body, *done, ci, cid)
        }
        CompiledNodeKind::RepeatCheck { done, .. } => validate_forward_target(*done, ci, cid),
        CompiledNodeKind::RetryCheck {
            body, exhausted, ..
        } => validate_loop_done_only(*body, *exhausted, ci, cid),
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            validate_loop_done_only(*body, *handler, ci, cid)
        }
    }
}

fn validate_choose_slot_edges(
    branches: &[crate::workflow::SlotBranch],
    otherwise: &Option<StepIdx>,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = *otherwise {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_choose_expr_edges(
    branches: &[crate::workflow::ExprBranch],
    otherwise: &Option<StepIdx>,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = *otherwise {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_loop_done_only(
    _body: StepIdx,
    done: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
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

    // Drop already-ended spans BEFORE the nesting comparison so that
    // sequential loops (whose previous span's `done <= ci`) do not
    // register as a stale enclosing loop. The remaining spans are the
    // active enclosing loops, which are the only ones that should
    // constrain the new span's nesting.
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
                        resource: "max_steps",
                    }
                })?),
                outer_done: StepIdx::new(u16::try_from(outer_done).map_err(|_| {
                    WorkflowError::ResourceContractExceeded {
                        resource: "max_steps",
                    }
                })?),
            });
        }
        _ => {}
    }

    spans.push((ci, done_idx));
    Ok(())
}
