//! Focused helpers for building a registered action evaluation context.
//!
//! Extracted from `Shard::registered_action_context` to keep the caller
//! under 25 logical lines while preserving exact ownership guard, artifact
//! load, input validation, encoding, and output build semantics.

use crate::shard::types::RunState;
use crate::{RuntimeError, RuntimeResult};
use vb_core::action::{ActionOutputReady, ActionTicket};
use vb_core::CompiledNodeKind;
use vb_core::{SlotIdx, SlotValue, Taint};

/// Validate the action node, matching action ID, and Clean input taint.
pub(super) fn validate_action_input(
    state: &RunState,
    ticket: ActionTicket,
) -> RuntimeResult<SlotIdx> {
    let node = state
        .workflow
        .node(ticket.step)
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    let CompiledNodeKind::Do { action, input } = node.kind else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    let input_taint = state
        .frame
        .read_taint(input)
        .map_err(|_| RuntimeError::InvalidActionCompletion)?;
    if action != ticket.action || input_taint != Taint::Clean {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    Ok(input)
}

/// Read an I64 slot value by index, erroring on type mismatch or read failure.
pub(super) fn read_slot_i64(state: &RunState, slot: SlotIdx) -> RuntimeResult<i64> {
    let SlotValue::I64(value) = state
        .frame
        .read_slot(slot)
        .map_err(|_| RuntimeError::InvalidActionCompletion)?
    else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    Ok(*value)
}

/// Read the I64 input value for a registered action and validate the
/// action node, input taint, and slot type.
pub(super) fn resolve_action_input(
    state: &RunState,
    ticket: ActionTicket,
) -> RuntimeResult<(i64, Taint)> {
    let input = validate_action_input(state, ticket)?;
    let value = read_slot_i64(state, input)?;
    Ok((value, Taint::Clean))
}

/// Encode an I64 value to canonical postcard bytes for the evaluation context.
pub(super) fn encode_action_input(value: i64) -> RuntimeResult<Vec<u8>> {
    postcard::to_allocvec(&SlotValue::I64(value)).map_err(|_| RuntimeError::EncodeFailed)
}

/// Build the action output descriptor from the value and encoded bytes.
pub(super) fn build_action_output(
    state: &RunState,
    ticket: ActionTicket,
    value: i64,
    encoded: &[u8],
) -> RuntimeResult<ActionOutputReady> {
    let node = state
        .workflow
        .node(ticket.step)
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    let output_slot = node.output.ok_or(RuntimeError::InvalidActionCompletion)?;
    let encoded_len = u32::try_from(encoded.len()).map_err(|_| RuntimeError::EncodeFailed)?;
    Ok(ActionOutputReady {
        output_slot,
        value: SlotValue::I64(value),
        taint: Taint::Clean,
        encoded_len,
    })
}
