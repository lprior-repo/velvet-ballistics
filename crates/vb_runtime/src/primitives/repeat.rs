#![forbid(unsafe_code)]
//! Repeat retry-loop primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

use super::helpers::{jump_to, jump_to_body, jump_to_next, require_output};

/// Shift used to encode `max_attempts` in the high 32 bits of the
/// attempt-slot I64 value.  Low 32 bits hold the current attempt index.
const REPEAT_SHIFT: u32 = 32;

/// Encodes `max_attempts` (high) and `current_attempt` (low) into one I64.
///
/// Layout: bits [47:32] = max_attempts, bits [15:0] = current_attempt.
/// Both fields are u16 so the result always fits in 48 bits (well within i64).
fn encode_repeat_state(max_attempts: u16, current_attempt: u16) -> Result<i64, EngineError> {
    if max_attempts == 0 || current_attempt > max_attempts {
        return Err(invalid_repeat_state());
    }
    let Some(high) = i64::from(max_attempts).checked_shl(REPEAT_SHIFT) else {
        return Err(EngineError::InternalInvariantViolation {
            reason: "repeat_state_encode_overflow",
        });
    };
    let Some(packed) = high.checked_add(i64::from(current_attempt)) else {
        return Err(EngineError::InternalInvariantViolation {
            reason: "repeat_state_encode_overflow",
        });
    };
    Ok(packed)
}

/// Decodes a packed repeat-state I64 into (max_attempts, current_attempt).
fn decode_repeat_state(packed: i64) -> Result<(u16, u16), EngineError> {
    let Ok(bits) = u64::try_from(packed) else {
        return Err(invalid_repeat_state());
    };
    let max_bits = bits >> REPEAT_SHIFT;
    let low_bits = bits & 0xFFFF;
    let reserved_bits = bits & 0xFFFF_0000;
    if reserved_bits != 0 {
        return Err(invalid_repeat_state());
    }
    let Ok(max_attempts) = u16::try_from(max_bits) else {
        return Err(invalid_repeat_state());
    };
    let Ok(current_attempt) = u16::try_from(low_bits) else {
        return Err(invalid_repeat_state());
    };
    if max_attempts == 0 || current_attempt > max_attempts {
        return Err(invalid_repeat_state());
    }
    Ok((max_attempts, current_attempt))
}

/// Executes RepeatStart: initializes attempt counter and jumps to body.
///
/// Writes packed repeat state (max_attempts | attempt=0) to the output
/// slot, then jumps to `body`.
pub fn repeat_start(
    run: &mut RunFrame,
    max_attempts: u16,
    body: StepIdx,
    _done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let attempt_output = require_output(output, run.pc())?;
    let state = encode_repeat_state(max_attempts, 0)?;
    run.write_slot(attempt_output, SlotValue::I64(state))?;
    jump_to(run, body)
}

/// Executes RepeatAttempt: reads the current attempt number from
/// attempt_slot (leaving the packed state intact) and jumps to body.
pub fn repeat_attempt(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    body: StepIdx,
    _done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let packed = expect_i64(*run.read_slot(attempt_slot)?)?;
    // Validate that the slot contains a valid repeat state.
    let (_max, _current) = decode_repeat_state(packed)?;
    // Slot already holds the correct packed state; just jump to body.
    jump_to_body(run, body)
}

/// Executes RepeatCheck: increments attempt counter, writes it back.
/// If `current_attempt >= max_attempts`, jumps to `done` (RepeatFinish).
/// Otherwise jumps to `next` (which points back to the loop body entry).
pub fn repeat_check(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    done: StepIdx,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let packed = expect_i64(*run.read_slot(attempt_slot)?)?;
    let (max_attempts, current_attempt) = decode_repeat_state(packed)?;

    // Increment attempt, clamping at u16::MAX to avoid overflow.
    let next_attempt = current_attempt.saturating_add(1);
    let updated = encode_repeat_state(max_attempts, next_attempt)?;
    run.write_slot(attempt_slot, SlotValue::I64(updated))?;

    if next_attempt >= max_attempts {
        // Exhausted all attempts -- route to the done (finish) node.
        jump_to(run, done)
    } else {
        // Attempts remain -- loop back to the body entry point.
        let body_entry = next.ok_or(EngineError::MissingNextStep { step })?;
        jump_to_body(run, body_entry)
    }
}

/// Executes RepeatFinish: writes the result value and advances.
pub fn repeat_finish(
    run: &mut RunFrame,
    result: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let value = *run.read_slot(result)?;
    let taint = run.read_taint(result)?;
    let out = require_output(output, step)?;
    run.write_slot_with_taint(out, value, taint)?;
    jump_to_next(run, next, step)
}

fn expect_i64(value: SlotValue) -> Result<i64, EngineError> {
    match value {
        SlotValue::I64(v) => Ok(v),
        other => Err(EngineError::TypeMismatch {
            expected: "number",
            found: other.type_name(),
        }),
    }
}

fn invalid_repeat_state() -> EngineError {
    EngineError::InternalInvariantViolation {
        reason: "invalid_repeat_state",
    }
}

#[cfg(test)]
mod repeat_tests;
