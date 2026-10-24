#![forbid(unsafe_code)]
//! Gate 15: Suspension resume target validation.
//!
//! Validates that every suspension point (Do/Ask node) has a valid resume
//! target: either terminal (next == None) or a forward node index within
//! bounds. This is a bounded structural check using actual IR guarantees
//! (node index bounds and forward progression) without claiming durability
//! or replay proof.

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

/// Validates that all suspension points have valid resume targets.
///
/// Every Do/Ask node must either be terminal (next == None) or have a next
/// target that is a valid node index strictly greater than the source node's
/// enumeration index (forward-only, no self/backward unmarked loop escape).
pub(crate) fn validate_resume_targets(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();

    for (node_index, node) in parts.nodes.iter().enumerate() {
        if !is_suspension_point(&node.kind) {
            continue;
        }

        if let Some(next_step) = node.next {
            let next_idx = next_step.as_usize();
            if next_idx >= node_count || next_idx <= node_index {
                return Err(ValidationError::NodeKindConstraintViolation {
                    node_index,
                    detail: format!(
                        "suspension resume target {} must be forward and below node_count {}",
                        next_idx, node_count
                    ),
                });
            }
        }
    }

    Ok(())
}

fn is_suspension_point(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::Do { .. } | CompiledNodeKind::Ask { .. }
    )
}

#[cfg(test)]
#[path = "gate_15_resume/tests.rs"]
mod tests;
