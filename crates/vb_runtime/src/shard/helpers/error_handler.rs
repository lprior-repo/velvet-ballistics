#![forbid(unsafe_code)]
//! Error-handler lookup helpers for failed steps.

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

/// Finds the error handler step and error slot for a failed step.
pub fn find_error_handler_for_failure(
    workflow: &CompiledWorkflow,
    failed: StepIdx,
) -> Option<(StepIdx, Option<SlotIdx>)> {
    if let Some(node) = workflow.node(failed)
        && let vb_core::workflow::ErrorRouting::Handler {
            handler,
            error_slot,
        } = node.routing.error
    {
        return Some((handler, error_slot));
    }
    if let Some(result) = error_handler_on_node(workflow, failed, failed) {
        return Some(result);
    }

    if failed.get() > 0 {
        let previous = StepIdx::new(failed.get().saturating_sub(1));
        if let Some(result) = error_handler_on_node(workflow, previous, failed) {
            return Some(result);
        }
    }

    let mut index = 0usize;
    let count = usize::from(workflow.node_count());
    while index < count {
        let Ok(raw) = u16::try_from(index) else {
            return None;
        };
        if let Some(result) = error_handler_on_node(workflow, StepIdx::new(raw), failed) {
            return Some(result);
        }
        index = index.checked_add(1)?;
    }

    None
}

fn error_handler_on_node(
    workflow: &CompiledWorkflow,
    candidate: StepIdx,
    failed: StepIdx,
) -> Option<(StepIdx, Option<SlotIdx>)> {
    let node = workflow.node(candidate)?;
    match node.kind {
        CompiledNodeKind::ErrorHandler {
            body,
            handler,
            error_slot,
        } if candidate == failed || body == failed => Some((handler, error_slot.slot())),
        _ => None,
    }
}
