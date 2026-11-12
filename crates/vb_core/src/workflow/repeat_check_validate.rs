#![forbid(unsafe_code)]
//!// Validates RepeatCheck backward next-edges against RepeatStart loop owners.
//!// RepeatCheck.next must target the owning RepeatStart's body step, and the
//!// check must match the owner's attempt slot and done target.

use super::{CompiledNode, CompiledNodeKind, WorkflowError, WorkflowParts};
use crate::ids::StepIdx;

/// Finds the nearest active enclosing structured start.
/// Active means the structured start's done/join target extends beyond check_index.
fn nearest_active_owner(
    parts: &WorkflowParts,
    check_index: usize,
) -> Option<(usize, &CompiledNode)> {
    for (i, node) in parts.nodes.iter().take(check_index).enumerate().rev() {
        let end_target = match &node.kind {
            CompiledNodeKind::ForEachStart { done, .. }
            | CompiledNodeKind::CollectStart { done, .. }
            | CompiledNodeKind::ReduceStart { done, .. }
            | CompiledNodeKind::RepeatStart { done, .. } => *done,
            CompiledNodeKind::TogetherStart { join, .. } => *join,
            _ => continue,
        };
        if end_target.as_usize() > check_index {
            return Some((i, node));
        }
    }
    None
}

pub(super) fn validate(
    parts: &WorkflowParts,
    check_index: usize,
    check: &CompiledNode,
    back_target: StepIdx,
) -> Result<(), WorkflowError> {
    let Some((owner_index, owner)) = nearest_active_owner(parts, check_index) else {
        return Err(WorkflowError::BackwardEdge {
            from: check.id,
            to: back_target,
        });
    };
    if !matches!(owner.kind, CompiledNodeKind::RepeatStart { .. }) {
        return Err(WorkflowError::BackwardEdge {
            from: check.id,
            to: back_target,
        });
    }
    let owner_body = match &owner.kind {
        CompiledNodeKind::RepeatStart { body, .. } => *body,
        _ => {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
    };
    let owner_done = match &owner.kind {
        CompiledNodeKind::RepeatStart { done, .. } => *done,
        _ => {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
    };
    let check_attempt = match &check.kind {
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => *attempt_slot,
        _ => {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
    };
    let check_done = match &check.kind {
        CompiledNodeKind::RepeatCheck { done, .. } => *done,
        _ => {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
    };
    if owner.output.slot() != Some(check_attempt) {
        return Err(WorkflowError::BackwardEdge {
            from: check.id,
            to: back_target,
        });
    }
    if owner_body != back_target {
        return Err(WorkflowError::BackwardEdge {
            from: check.id,
            to: back_target,
        });
    }
    if owner_done != check_done {
        return Err(WorkflowError::BackwardEdge {
            from: check.id,
            to: back_target,
        });
    }
    if owner_index >= check_index {
        return Err(WorkflowError::BackwardEdge {
            from: check.id,
            to: back_target,
        });
    }
    if owner_body.as_usize() >= check_index {
        return Err(WorkflowError::BackwardEdge {
            from: check.id,
            to: back_target,
        });
    }
    if check_index >= owner_done.as_usize() {
        return Err(WorkflowError::BackwardEdge {
            from: check.id,
            to: back_target,
        });
    }
    Ok(())
}
