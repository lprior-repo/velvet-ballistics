use crate::ids::{ListId, SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::workflow::CompiledNode;

use super::super::ReplayError;

/// Shared deterministic helpers for loop-primitive replay (`for_each`,
/// `reduce`). Mirrors `vb_runtime::primitives::helpers` without depending on
/// the runtime crate.
pub(super) fn expect_list_value(value: SlotValue) -> Result<ListId, ReplayError> {
    match value {
        SlotValue::List(id) => Ok(id),
        _ => Err(ReplayError::Internal {
            reason: "loop slot was not list during replay",
        }),
    }
}

/// Requires the node's declared output slot.
pub(super) fn replay_output(
    node: &CompiledNode,
    reason: &'static str,
) -> Result<SlotIdx, ReplayError> {
    node.output.slot().ok_or(ReplayError::Internal { reason })
}

/// Requires the node's routing continuation step.
pub(super) fn replay_next(
    node: &CompiledNode,
    reason: &'static str,
) -> Result<StepIdx, ReplayError> {
    node.routing
        .next
        .step()
        .ok_or(ReplayError::Internal { reason })
}

/// Builds the 2-element iterator state list encoding `(source_id, cursor)`,
/// mirroring the executor's cursor-based iterator state.
pub(super) fn build_iterator_state(
    source: ListId,
    cursor: usize,
) -> Result<Box<[SlotValue]>, ReplayError> {
    let source_token = i64::from(source.get());
    let cursor_token = i64::try_from(cursor).map_err(|_| ReplayError::Internal {
        reason: "iterator state cursor exceeds i64 range during replay",
    })?;
    Ok(Box::from([
        SlotValue::I64(source_token),
        SlotValue::I64(cursor_token),
    ]))
}

/// Decodes a 2-element iterator state list into `(source_id, cursor)`.
pub(super) fn decode_iterator_state(items: &[SlotValue]) -> Result<(ListId, usize), ReplayError> {
    if items.len() != 2 {
        return Err(ReplayError::Internal {
            reason: "iterator state must be a 2-element list during replay",
        });
    }
    let source_token = match items.first().copied() {
        Some(SlotValue::I64(value)) => value,
        _ => {
            return Err(ReplayError::Internal {
                reason: "iterator source token must be I64 during replay",
            });
        }
    };
    let cursor_token = match items.get(1).copied() {
        Some(SlotValue::I64(value)) => value,
        _ => {
            return Err(ReplayError::Internal {
                reason: "iterator cursor must be I64 during replay",
            });
        }
    };
    let source_id = u32::try_from(source_token).map_err(|_| ReplayError::Internal {
        reason: "iterator source token out of u32 range during replay",
    })?;
    let cursor = usize::try_from(cursor_token).map_err(|_| ReplayError::Internal {
        reason: "iterator cursor out of usize range during replay",
    })?;
    Ok((ListId::new(source_id), cursor))
}

/// Advances an iterator cursor, mirroring the executor overflow guard.
pub(super) fn advance_cursor(cursor: usize) -> Result<usize, ReplayError> {
    cursor.checked_add(1).ok_or(ReplayError::Internal {
        reason: "iterator cursor overflow during replay",
    })
}
