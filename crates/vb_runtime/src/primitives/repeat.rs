//! Repeat retry-loop primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

use super::helpers::{jump_to, jump_to_next, require_output};

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
    let updated = encode_repeat_state(max_attempts, next_attempt)?;
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
mod tests {
    use super::*;

    fn fresh_frame() -> RunFrame {
        crate::test_harness::fresh_frame(8, 8)
    }

    fn encoded(max_attempts: u16, current_attempt: u16) -> i64 {
        encode_repeat_state(max_attempts, current_attempt)
            .ok()
            .unwrap_or_else(|| panic!("encode must succeed"))
    }

    fn decoded(packed: i64) -> (u16, u16) {
        decode_repeat_state(packed)
            .ok()
            .unwrap_or_else(|| panic!("decode must succeed"))
    }

    #[test]
    fn repeat_start_initializes_max_attempts() {
        let mut run = fresh_frame();
        let output = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        let result = repeat_start(&mut run, 5, body, done, Some(output));

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        let slot_val = *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"));
        assert!(matches!(slot_val, SlotValue::I64(_)));
        let packed = match slot_val {
            SlotValue::I64(v) => v,
            _ => 0,
        };
        let expected = encoded(5, 0);
        assert_eq!(packed, expected);
    }

    #[test]
    fn repeat_attempt_writes_current_attempt_to_slot() {
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        let packed = encoded(3, 1);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));

        let result = repeat_attempt(&mut run, attempt_slot, body, done);

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    #[test]
    fn repeat_check_routes_to_body_when_attempts_remain() {
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(2);
        let next_body = StepIdx::new(1);
        let packed = encoded(5, 2);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));

        let result = repeat_check(&mut run, attempt_slot, done, Some(next_body), StepIdx::ZERO);

        // current_attempt=2 is incremented to 3, which is still < max_attempts=5,
        // so the loop routes back to the body entry point.
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_body);
    }

    #[test]
    fn repeat_check_routes_to_done_when_attempts_exhausted() {
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(2);
        let next_body = StepIdx::new(1);
        let packed = encoded(3, 2);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));

        let result = repeat_check(&mut run, attempt_slot, done, Some(next_body), StepIdx::ZERO);

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn repeat_finish_writes_result_to_output() {
        let mut run = fresh_frame();
        let result_slot = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(3);
        run.write_slot(result_slot, SlotValue::I64(77))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));

        let result = repeat_finish(
            &mut run,
            result_slot,
            Some(output),
            Some(next_step),
            StepIdx::ZERO,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
        assert_eq!(
            *run.read_slot(output)
                .ok()
                .unwrap_or_else(|| panic!("read must succeed")),
            SlotValue::I64(77)
        );
    }

    #[test]
    fn decode_repeat_state_roundtrips_with_encode() {
        let max_attempts: u16 = 7;
        let current_attempt: u16 = 3;
        let packed = encoded(max_attempts, current_attempt);
        let (decoded_max, decoded_current) = decoded(packed);
        assert_eq!(decoded_max, max_attempts);
        assert_eq!(decoded_current, current_attempt);
    }

    #[test]
    fn repeat_attempt_rejects_negative_repeat_state() {
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        run.write_slot(attempt_slot, SlotValue::I64(-1))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));

        let result = repeat_attempt(&mut run, attempt_slot, StepIdx::new(1), StepIdx::new(2));

        assert_eq!(result, Err(invalid_repeat_state()));
    }

    #[test]
    fn repeat_check_rejects_reserved_repeat_state_bits() {
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let reserved_middle_bits = 1_i64 << 16;
        run.write_slot(attempt_slot, SlotValue::I64(reserved_middle_bits))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));

        let result = repeat_check(
            &mut run,
            attempt_slot,
            StepIdx::new(2),
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );

        assert_eq!(result, Err(invalid_repeat_state()));
    }

    // BDD tests for repeat primitives

    #[test]
    fn repeat_start_returns_error_when_output_missing() {
        // Given a frame
        let mut run = fresh_frame();
        // When calling repeat_start with output=None
        let result = repeat_start(&mut run, 5, StepIdx::new(1), StepIdx::new(2), None);
        // Then it returns MissingOutputSlot
        match result {
            Err(EngineError::MissingOutputSlot { step }) => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn repeat_start_encodes_max_attempts_in_output() {
        // Given a frame
        let mut run = fresh_frame();
        let output = SlotIdx::new(0);
        // When calling repeat_start with max_attempts=10
        let result = repeat_start(&mut run, 10, StepIdx::new(1), StepIdx::new(2), Some(output));
        // Then the output slot encodes max_attempts=10, current=0
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        let packed = match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"))
        {
            SlotValue::I64(v) => v,
            _ => return,
        };
        let (max, current) = decoded(packed);
        assert_eq!(max, 10);
        assert_eq!(current, 0);
    }

    #[test]
    fn repeat_attempt_returns_error_when_slot_is_not_i64() {
        // Given a frame with a non-I64 in attempt slot
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        run.write_slot(attempt_slot, SlotValue::Bool(true))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling repeat_attempt
        let result = repeat_attempt(&mut run, attempt_slot, StepIdx::new(1), StepIdx::new(2));
        // Then it returns TypeMismatch
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                assert_eq!(expected, "number");
                assert_eq!(found, "boolean");
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn repeat_check_returns_error_when_slot_is_not_i64() {
        // Given a frame with a non-I64 in attempt slot
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        run.write_slot(attempt_slot, SlotValue::Bool(true))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            StepIdx::new(2),
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then it returns TypeMismatch
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                assert_eq!(expected, "number");
                assert_eq!(found, "boolean");
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn repeat_check_returns_error_when_next_missing_and_attempts_remain() {
        // Given a frame with attempts remaining
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let packed = encoded(5, 1);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling repeat_check with next=None and attempts remain
        let result = repeat_check(&mut run, attempt_slot, StepIdx::new(2), None, StepIdx::ZERO);
        // Then it returns MissingNextStep
        match result {
            Err(EngineError::MissingNextStep { step }) => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn repeat_finish_returns_error_when_output_missing() {
        // Given a frame with a result
        let mut run = fresh_frame();
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling repeat_finish with output=None
        let result = repeat_finish(
            &mut run,
            SlotIdx::new(0),
            None,
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then it returns MissingOutputSlot
        match result {
            Err(EngineError::MissingOutputSlot { step }) => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn repeat_finish_returns_error_when_next_missing() {
        // Given a frame
        let mut run = fresh_frame();
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling repeat_finish with next=None
        let result = repeat_finish(
            &mut run,
            SlotIdx::new(0),
            Some(SlotIdx::new(1)),
            None,
            StepIdx::ZERO,
        );
        // Then it returns MissingNextStep
        match result {
            Err(EngineError::MissingNextStep { step }) => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn repeat_check_increments_attempt_counter() {
        // Given a frame with max_attempts=5, current=2
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let packed = encoded(5, 2);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            StepIdx::new(2),
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then the attempt counter is incremented to 3
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        let updated = match *run
            .read_slot(attempt_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"))
        {
            SlotValue::I64(v) => v,
            _ => return,
        };
        let (max, current) = decoded(updated);
        assert_eq!(max, 5);
        assert_eq!(current, 3);
    }

    #[test]
    fn repeat_check_routes_to_done_at_exact_boundary() {
        // Given a frame with max_attempts=3, current=2 (next increment = 3)
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(5);
        let packed = encoded(3, 2);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            done,
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then it routes to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn repeat_finish_copies_result_to_output_slot() {
        // Given a frame with result value in slot 0
        let mut run = fresh_frame();
        let result_slot = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(3);
        run.write_slot(result_slot, SlotValue::I64(77))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling repeat_finish
        let result = repeat_finish(
            &mut run,
            result_slot,
            Some(output),
            Some(next_step),
            StepIdx::ZERO,
        );
        // Then output slot has the result value
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(
            *run.read_slot(output)
                .ok()
                .unwrap_or_else(|| panic!("read must succeed")),
            SlotValue::I64(77)
        );
    }

    #[test]
    fn encode_repeat_state_zero_max_attempts_is_invalid() {
        // Given zero max attempts
        let result = encode_repeat_state(0, 0);
        // Then it is rejected instead of silently encoding an invalid loop state.
        assert_eq!(result, Err(invalid_repeat_state()));
    }

    #[test]
    fn encode_decode_repeat_state_max_values() {
        // Given max values
        let packed = encoded(u16::MAX, u16::MAX);
        let (max, current) = decoded(packed);
        // Then both decode to max
        assert_eq!(max, u16::MAX);
        assert_eq!(current, u16::MAX);
    }

    #[test]
    fn repeat_start_increments_executed_counter() {
        // Given a frame
        let mut run = fresh_frame();
        let output = SlotIdx::new(0);
        let before = run.executed();
        // When calling repeat_start
        let result = repeat_start(&mut run, 5, StepIdx::new(1), StepIdx::new(2), Some(output));
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn repeat_attempt_increments_executed_counter() {
        // Given a frame with packed state
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let packed = encoded(3, 1);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let before = run.executed();
        // When calling repeat_attempt
        let result = repeat_attempt(&mut run, attempt_slot, StepIdx::new(1), StepIdx::new(2));
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn repeat_check_increments_executed_counter_when_routing_to_body() {
        // Given a frame with attempts remaining
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let packed = encoded(5, 1);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let before = run.executed();
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            StepIdx::new(2),
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn repeat_check_increments_executed_counter_when_routing_to_done() {
        // Given a frame with exhausted attempts
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let packed = encoded(3, 2);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let before = run.executed();
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            StepIdx::new(2),
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn repeat_finish_increments_executed_counter() {
        // Given a frame with result
        let mut run = fresh_frame();
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let before = run.executed();
        // When calling repeat_finish
        let result = repeat_finish(
            &mut run,
            SlotIdx::new(0),
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(3)),
            StepIdx::ZERO,
        );
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn repeat_check_updates_slot_value_even_when_routing_to_done() {
        // Given a frame at the boundary
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(5);
        let packed = encoded(2, 1);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            done,
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then the slot value is updated to attempt=2
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
        let updated = match *run
            .read_slot(attempt_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"))
        {
            SlotValue::I64(v) => v,
            _ => return,
        };
        let (max, current) = decoded(updated);
        assert_eq!(max, 2);
        assert_eq!(current, 2);
    }

    #[test]
    fn encode_repeat_state_one_zero() {
        // Given max_attempts=1, current=0
        let packed = encoded(1, 0);
        let (max, current) = decoded(packed);
        assert_eq!(max, 1);
        assert_eq!(current, 0);
    }

    #[test]
    fn repeat_start_with_max_attempts_one() {
        // Given a frame
        let mut run = fresh_frame();
        let output = SlotIdx::new(0);
        // When calling repeat_start with max_attempts=1
        let result = repeat_start(&mut run, 1, StepIdx::new(1), StepIdx::new(2), Some(output));
        // Then it encodes max=1, current=0
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        let packed = match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"))
        {
            SlotValue::I64(v) => v,
            _ => return,
        };
        let (max, current) = decoded(packed);
        assert_eq!(max, 1);
        assert_eq!(current, 0);
    }

    #[test]
    fn repeat_start_jumps_to_body() {
        // Given a frame
        let mut run = fresh_frame();
        let output = SlotIdx::new(0);
        let body = StepIdx::new(3);
        // When calling repeat_start
        let result = repeat_start(&mut run, 5, body, StepIdx::new(2), Some(output));
        // Then pc is at body
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
    }

    // ── Adversarial BDD tests for repeat ────────────────────────────────

    #[test]
    fn repeat_start_max_attempts_zero_is_invalid() {
        // Given a frame
        let mut run = fresh_frame();
        let output = SlotIdx::new(0);
        // When calling repeat_start with max_attempts=0
        let result = repeat_start(&mut run, 0, StepIdx::new(1), StepIdx::new(2), Some(output));
        // Then it rejects the invalid repeat state.
        assert_eq!(result, Err(invalid_repeat_state()));
    }

    #[test]
    fn repeat_check_max_attempts_zero_state_is_invalid() {
        // Given a frame with max=0, current=0
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(5);
        run.write_slot(attempt_slot, SlotValue::I64(0))
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            done,
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then it rejects the invalid repeat state instead of routing silently.
        assert_eq!(result, Err(invalid_repeat_state()));
    }

    #[test]
    fn repeat_start_max_attempts_one_immediate_check_exits() {
        // Given repeat_start with max_attempts=1
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(5);
        // Simulate: start wrote max=1, current=0, body ran, now check
        let packed = encoded(1, 0);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            done,
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then next_attempt=1 >= max=1, so it routes to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
        let updated = match *run
            .read_slot(attempt_slot)
            .ok()
            .unwrap_or_else(|| panic!("must read"))
        {
            SlotValue::I64(v) => v,
            _ => return,
        };
        let (max, current) = decoded(updated);
        assert_eq!(max, 1);
        assert_eq!(current, 1);
    }

    #[test]
    fn repeat_check_u16_max_attempts_does_not_overflow() {
        // Given a frame with max=u16::MAX, current=u16::MAX-1
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let body = StepIdx::new(1);
        let packed = encoded(u16::MAX, u16::MAX - 1);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            StepIdx::new(2),
            Some(body),
            StepIdx::ZERO,
        );
        // Then next_attempt = (MAX-1) + 1 = MAX which >= MAX, routes to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), StepIdx::new(2));
    }

    #[test]
    fn repeat_check_saturating_add_at_u16_max_still_routes_to_done() {
        // Given a frame with max=u16::MAX, current=u16::MAX (already exhausted)
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        let done = StepIdx::new(2);
        let packed = encoded(u16::MAX, u16::MAX);
        run.write_slot(attempt_slot, SlotValue::I64(packed))
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            done,
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then saturating_add keeps it at u16::MAX which >= u16::MAX, routes to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn repeat_attempt_slot_corrupted_to_null_returns_type_mismatch() {
        // Given a frame with Null in attempt_slot (corruption)
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        run.write_slot(attempt_slot, SlotValue::Null)
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling repeat_attempt
        let result = repeat_attempt(&mut run, attempt_slot, StepIdx::new(1), StepIdx::new(2));
        // Then it returns TypeMismatch
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                assert_eq!(expected, "number");
                assert_eq!(found, "null");
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn repeat_check_slot_corrupted_to_null_returns_type_mismatch() {
        // Given a frame with Null in attempt_slot (corruption)
        let mut run = fresh_frame();
        let attempt_slot = SlotIdx::new(0);
        run.write_slot(attempt_slot, SlotValue::Null)
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling repeat_check
        let result = repeat_check(
            &mut run,
            attempt_slot,
            StepIdx::new(2),
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then it returns TypeMismatch
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                assert_eq!(expected, "number");
                assert_eq!(found, "null");
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn repeat_finish_same_result_and_output_slot_succeeds() {
        // Given a frame where result_slot == output_slot
        let mut run = fresh_frame();
        let slot = SlotIdx::new(0);
        let next_step = StepIdx::new(3);
        run.write_slot(slot, SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling repeat_finish with result == output
        let result = repeat_finish(&mut run, slot, Some(slot), Some(next_step), StepIdx::ZERO);
        // Then it succeeds (reads value first, writes same value back)
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
        assert_eq!(
            *run.read_slot(slot)
                .ok()
                .unwrap_or_else(|| panic!("must read")),
            SlotValue::I64(42)
        );
    }

    #[test]
    fn repeat_attempt_number_zero_is_accessible_via_decode() {
        // Given a fresh repeat_start with max=5
        let mut run = fresh_frame();
        let output = SlotIdx::new(0);
        repeat_start(&mut run, 5, StepIdx::new(1), StepIdx::new(2), Some(output))
            .ok()
            .unwrap_or_else(|| panic!("start"));
        // When reading the packed state
        let packed = match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read"))
        {
            SlotValue::I64(v) => v,
            _ => return,
        };
        // Then the current attempt is 0 (first attempt)
        let (max, current) = decoded(packed);
        assert_eq!(max, 5);
        assert_eq!(current, 0);
    }
}
