//! Repeat retry-loop primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

use super::helpers::{jump_to, jump_to_next, require_output};

const REPEAT_SHIFT: u32 = 32;

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

pub fn repeat_attempt(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    body: StepIdx,
    _done: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let packed = expect_i64(*run.read_slot(attempt_slot)?)?;
    let (_max, _current) = decode_repeat_state(packed)?;
    jump_to(run, body)
}

pub fn repeat_check(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    done: StepIdx,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let packed = expect_i64(*run.read_slot(attempt_slot)?)?;
    let (max_attempts, current_attempt) = decode_repeat_state(packed)?;

    let next_attempt = current_attempt.saturating_add(1);
    let updated = encode_repeat_state(max_attempts, next_attempt)?;
    run.write_slot(attempt_slot, SlotValue::I64(updated))?;

    if next_attempt >= max_attempts {
        jump_to(run, done)
    } else {
        let body_entry = next.ok_or(EngineError::MissingNextStep { step })?;
        jump_to(run, body_entry)
    }
}

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
