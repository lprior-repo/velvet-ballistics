#![forbid(unsafe_code)]
//! Shared numeric Repeat transitions; callers retain body-entry policy.

use crate::{EngineError, RunFrame, SlotIdx, SlotValue, StepIdx};

/// Selected edge after a numeric Repeat transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatFlow {
    /// Ordinary continuation without entry reactivation.
    Forward(StepIdx),
    /// Body continuation requiring the caller's entry policy.
    Body(StepIdx),
}

impl RepeatFlow {
    /// Selected program counter.
    #[must_use]
    pub const fn target(self) -> StepIdx {
        match self {
            Self::Forward(target) | Self::Body(target) => target,
        }
    }
}

const REPEAT_SHIFT: u32 = 32;

/// Encodes maximum and current attempts in the existing packed I64 layout.
pub fn encode_repeat_state(max_attempts: u16, current_attempt: u16) -> Result<i64, EngineError> {
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

/// Decodes and validates the packed I64 attempt state.
pub fn decode_repeat_state(packed: i64) -> Result<(u16, u16), EngineError> {
    let Ok(bits) = u64::try_from(packed) else {
        return Err(invalid_repeat_state());
    };
    let max_bits = bits >> REPEAT_SHIFT;
    let low_bits = bits & 0xFFFF;
    if bits & 0xFFFF_0000 != 0 {
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

/// Initializes the attempt slot and selects the initial body edge.
pub fn repeat_start(
    run: &mut RunFrame,
    max_attempts: u16,
    body: StepIdx,
    output: Option<SlotIdx>,
) -> Result<RepeatFlow, EngineError> {
    let output = output.ok_or_else(|| EngineError::MissingOutputSlot { step: run.pc() })?;
    let state = encode_repeat_state(max_attempts, 0)?;
    run.write_slot(output, SlotValue::I64(state))?;
    Ok(RepeatFlow::Forward(body))
}

/// Validates the existing attempt state and selects body reentry.
pub fn repeat_attempt(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    body: StepIdx,
) -> Result<RepeatFlow, EngineError> {
    let packed = expect_i64(*run.read_slot(attempt_slot)?)?;
    let _state = decode_repeat_state(packed)?;
    Ok(RepeatFlow::Body(body))
}

/// Increments the packed attempt state and selects completion or body reentry.
pub fn repeat_check(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    done: StepIdx,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<RepeatFlow, EngineError> {
    let packed = expect_i64(*run.read_slot(attempt_slot)?)?;
    let (maximum, current) = decode_repeat_state(packed)?;
    let following = current.saturating_add(1);
    let updated = encode_repeat_state(maximum, following)?;
    run.write_slot(attempt_slot, SlotValue::I64(updated))?;
    if following >= maximum {
        Ok(RepeatFlow::Forward(done))
    } else {
        let Some(body) = next else {
            return Err(EngineError::MissingNextStep { step });
        };
        Ok(RepeatFlow::Body(body))
    }
}

/// Copies the final value and taint before selecting the next step.
pub fn repeat_finish(
    run: &mut RunFrame,
    result: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<RepeatFlow, EngineError> {
    let value = *run.read_slot(result)?;
    let taint = run.read_taint(result)?;
    let Some(output) = output else {
        return Err(EngineError::MissingOutputSlot { step });
    };
    run.write_slot_with_taint(output, value, taint)?;
    let Some(target) = next else {
        return Err(EngineError::MissingNextStep { step });
    };
    Ok(RepeatFlow::Forward(target))
}

fn expect_i64(value: SlotValue) -> Result<i64, EngineError> {
    match value {
        SlotValue::I64(value) => Ok(value),
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
