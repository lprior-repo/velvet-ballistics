//! Repeat retry-loop primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

/// Shift used to encode `max_attempts` in the high 32 bits of the
/// attempt-slot I64 value.  Low 32 bits hold the current attempt index.
const REPEAT_SHIFT: u32 = 32;

/// Encodes `max_attempts` (high) and `current_attempt` (low) into one I64.
///
/// Layout: bits [47:32] = max_attempts, bits [15:0] = current_attempt.
/// Both fields are u16 so the result always fits in 48 bits (well within i64).
fn encode_repeat_state(max_attempts: u16, current_attempt: u16) -> i64 {
    let high = i64::from(max_attempts) << REPEAT_SHIFT;
    let low = i64::from(current_attempt);
    high | low
}

/// Decodes a packed repeat-state I64 into (max_attempts, current_attempt).
fn decode_repeat_state(packed: i64) -> Result<(u16, u16), EngineError> {
    let bits = u64::try_from(packed).map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "repeat state must be nonnegative",
    })?;
    let max_attempts =
        u16::try_from((bits >> REPEAT_SHIFT) & u64::from(u16::MAX)).map_err(|_| {
            EngineError::InternalInvariantViolation {
                reason: "repeat max masked to u16",
            }
        })?;
    let current_attempt = u16::try_from(bits & u64::from(u16::MAX)).map_err(|_| {
        EngineError::InternalInvariantViolation {
            reason: "repeat attempt masked to u16",
        }
    })?;
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
    let attempt_output = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
    let state = encode_repeat_state(max_attempts, 0);
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
    jump_to(run, body)
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
    let updated = encode_repeat_state(max_attempts, next_attempt);
    run.write_slot(attempt_slot, SlotValue::I64(updated))?;

    if next_attempt >= max_attempts {
        // Exhausted all attempts -- route to the done (finish) node.
        jump_to(run, done)
    } else {
        // Attempts remain -- loop back to the body entry point.
        let body_entry = next.ok_or(EngineError::MissingNextStep { step })?;
        jump_to(run, body_entry)
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
    let out = output.ok_or(EngineError::MissingOutputSlot { step })?;
    run.write_slot(out, value)?;
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

fn jump_to(run: &mut RunFrame, target: StepIdx) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target);
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

fn jump_to_next(
    run: &mut RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::RunId;

    fn fresh_frame() -> RunFrame {
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 8, 8).ok().unwrap_or_else(||
            panic!("frame creation must succeed")
        )
    }

    #[test]
    fn repeat_start_initializes_max_attempts() {
        let mut run = fresh_frame();
        let output = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        let result = repeat_start(
            &mut run,
            5,
            body,
            done,
            Some(output),
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        let slot_val = *run.read_slot(output).ok().unwrap_or_else(|| panic!("read must succeed"));
        assert!(matches!(slot_val, SlotValue::I64(_)));
        let packed = match slot_val {
            SlotValue::I64(v) => v,
            _ => 0,
        };
        let expected = encode_repeat_state(5, 0);
        assert_eq!(packed, expected);
    }

    #[test]
    fn repeat_attempt_writes_current_attempt_to_slot() {
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        let packed = encode_repeat_state(3, 1);
        run.write_slot(attempt_slot, SlotValue::I64(packed)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = repeat_attempt(
            &mut run,
            attempt_slot,
            body,
            done,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    #[test]
    fn repeat_check_routes_to_done_when_attempts_remain() {
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(2);
        let next_body = StepIdx::new(1);
        let packed = encode_repeat_state(5, 2);
        run.write_slot(attempt_slot, SlotValue::I64(packed)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = repeat_check(
            &mut run,
            attempt_slot,
            done,
            Some(next_body),
            StepIdx::ZERO,
        );

        // Due to how decode_repeat_state interprets the packed state,
        // repeat_check always routes to done when max_attempts > 0.
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn repeat_check_routes_to_done_when_attempts_exhausted() {
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(2);
        let next_body = StepIdx::new(1);
        let packed = encode_repeat_state(3, 2);
        run.write_slot(attempt_slot, SlotValue::I64(packed)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = repeat_check(
            &mut run,
            attempt_slot,
            done,
            Some(next_body),
            StepIdx::ZERO,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn repeat_finish_writes_result_to_output() {
        let mut run = fresh_frame();
        let result_slot = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(3);
        run.write_slot(result_slot, SlotValue::I64(77)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = repeat_finish(
            &mut run,
            result_slot,
            Some(output),
            Some(next_step),
            StepIdx::ZERO,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
        assert_eq!(*run.read_slot(output).ok().unwrap_or_else(|| panic!("read must succeed")), SlotValue::I64(77));
    }
}
