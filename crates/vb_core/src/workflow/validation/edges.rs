//! Forward-edge graph validation.

use crate::ids::StepIdx;

use super::super::node::CompiledNodeKind;
use super::super::types::{WorkflowError, WorkflowParts};

/// Check B: all edges must point forward except recognized loop back-edges.
/// Check D: loop spans must be properly nested (no overlapping loops).
pub(crate) fn validate_forward_edges(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let mut loop_spans: Vec<(usize, usize)> = Vec::new();

    for (index, node) in parts.nodes.iter().enumerate() {
        let current_id = StepIdx::new(u16::try_from(index).map_err(|_| {
            WorkflowError::ResourceContractExceeded {
                resource: "max_steps",
            }
        })?);

        if let Some(next) = node.next {
            validate_forward_target(next, index, current_id)?;
        }

        validate_kind_edges(&node.kind, index, current_id)?;

        push_loop_span(&node.kind, index, &mut loop_spans)?;
    }
    Ok(())
}

/// Validates that kind-specific edges respect the forward-only rule.
#[allow(clippy::match_same_arms)]
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
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                validate_forward_target(branch.target, ci, cid)?;
            }
            if let Some(fallback) = *otherwise {
                validate_forward_target(fallback, ci, cid)?;
            }
            Ok(())
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                validate_forward_target(branch.target, ci, cid)?;
            }
            if let Some(fallback) = *otherwise {
                validate_forward_target(fallback, ci, cid)?;
            }
            Ok(())
        }
        CompiledNodeKind::ForEachStart { body, done, .. } => {
            let _ = body;
            validate_forward_target(*done, ci, cid)
        }
        CompiledNodeKind::ForEachNext { body, done, .. } => {
            let _ = body;
            validate_forward_target(*done, ci, cid)
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            let _ = branches;
            validate_forward_target(*join, ci, cid)
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            let _ = entry;
            validate_forward_target(*join, ci, cid)
        }
        CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            let _ = body;
            validate_forward_target(*done, ci, cid)
        }
        CompiledNodeKind::RepeatCheck { done, .. } => validate_forward_target(*done, ci, cid),
        CompiledNodeKind::RetryCheck {
            body, exhausted, ..
        } => {
            let _ = body;
            validate_forward_target(*exhausted, ci, cid)
        }
        CompiledNodeKind::ErrorHandler { body, handler } => {
            let _ = body;
            validate_forward_target(*handler, ci, cid)
        }
        CompiledNodeKind::Jump { .. } => Ok(()),
    }
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

    if let Some(&(_outer_start, outer_done)) = spans.last()
        && done_idx > outer_done
    {
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

    while spans
        .last()
        .is_some_and(|&(_, done): &(usize, usize)| done <= ci)
    {
        spans.pop();
    }

    spans.push((ci, done_idx));
    Ok(())
}
