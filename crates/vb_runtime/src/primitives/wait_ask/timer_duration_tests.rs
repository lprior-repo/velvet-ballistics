use super::*;
use proptest::prelude::*;
use std::time::Duration;
use vb_core::ids::SymbolId;
use vb_core::value::{FiniteF64, SlotValue};

fn frame_with(slot: SlotIdx, value: SlotValue) -> Result<RunFrame, EngineError> {
    let mut frame = crate::test_harness::fresh_frame(4, 8);
    frame.write_slot(slot, value)?;
    Ok(frame)
}

fn huge_timeout() -> Result<SlotValue, EngineError> {
    Ok(SlotValue::F64(FiniteF64::new(1.0e100)?))
}

#[test]
fn timeout_duration_keeps_zero_negative_and_fractional_millisecond_behavior()
-> Result<(), EngineError> {
    assert_eq!(timeout_duration(SlotValue::I64(0)), Ok(Duration::ZERO));
    assert_eq!(timeout_duration(SlotValue::I64(-1)), Ok(Duration::ZERO));
    assert_eq!(
        timeout_duration(SlotValue::F64(FiniteF64::new(-0.5)?)),
        Ok(Duration::ZERO)
    );
    assert_eq!(
        timeout_duration(SlotValue::F64(FiniteF64::new(0.5)?)),
        Ok(Duration::ZERO)
    );
    assert_eq!(
        timeout_duration(SlotValue::F64(FiniteF64::new(1.5)?)),
        Ok(Duration::from_millis(1))
    );
    assert_eq!(
        timeout_duration(SlotValue::I64(i64::MAX)),
        Ok(Duration::from_millis(i64::MAX.unsigned_abs()))
    );
    Ok(())
}

proptest! {
    #[test]
    fn integer_timeout_conversion_preserves_nonnegative_milliseconds(value in any::<i64>()) {
        let expected = if value <= 0 {
            Duration::ZERO
        } else {
            Duration::from_millis(value.unsigned_abs())
        };
        prop_assert_eq!(timeout_duration(SlotValue::I64(value)), Ok(expected));
    }
}

#[test]
fn positive_finite_timeout_outside_duration_range_is_typed() -> Result<(), EngineError> {
    assert_eq!(
        timeout_duration(huge_timeout()?),
        Err(EngineError::ResourceLimitExceeded {
            resource: TIMER_DURATION_RESOURCE,
        })
    );
    assert_eq!(
        timeout_duration(SlotValue::F64(FiniteF64::new(1.0e20)?)),
        Err(EngineError::ResourceLimitExceeded {
            resource: TIMER_DURATION_RESOURCE,
        })
    );
    Ok(())
}

#[test]
fn ask_rejects_finite_milliseconds_outside_unsigned_timer_range() -> Result<(), EngineError> {
    let prompt = SlotIdx::ZERO;
    let timeout = SlotIdx::new(1);
    let mut frame = frame_with(prompt, SlotValue::Symbol(SymbolId::new(3)))?;
    frame.write_slot(timeout, SlotValue::F64(FiniteF64::new(1.0e20)?))?;

    assert_eq!(
        ask(&mut frame, prompt, Some(timeout)),
        Err(EngineError::ResourceLimitExceeded {
            resource: TIMER_DURATION_RESOURCE,
        })
    );
    Ok(())
}

#[test]
fn wait_until_rejects_unrepresentable_timeout_before_increment() -> Result<(), EngineError> {
    let deadline = SlotIdx::ZERO;
    let mut frame = frame_with(deadline, huge_timeout()?)?;
    let before = frame.executed();

    let result = wait_until(&mut frame, deadline);

    assert_eq!(
        result,
        Err(EngineError::ResourceLimitExceeded {
            resource: TIMER_DURATION_RESOURCE,
        })
    );
    assert_eq!(frame.executed(), before);
    Ok(())
}

#[test]
fn wait_event_rejects_unrepresentable_timeout_before_increment() -> Result<(), EngineError> {
    let mut frame = frame_with(SlotIdx::ZERO, SlotValue::I64(1))?;
    frame.write_slot(SlotIdx::new(1), huge_timeout()?)?;
    let before = frame.executed();

    let result = wait_event(&mut frame, SlotIdx::ZERO, Some(SlotIdx::new(1)));

    assert_eq!(
        result,
        Err(EngineError::ResourceLimitExceeded {
            resource: TIMER_DURATION_RESOURCE,
        })
    );
    assert_eq!(frame.executed(), before);
    Ok(())
}

#[test]
fn ask_rejects_unrepresentable_timeout_before_increment() -> Result<(), EngineError> {
    let mut frame = frame_with(SlotIdx::ZERO, SlotValue::Symbol(SymbolId::new(3)))?;
    frame.write_slot(SlotIdx::new(1), huge_timeout()?)?;
    let before = frame.executed();

    let result = ask(&mut frame, SlotIdx::ZERO, Some(SlotIdx::new(1)));

    assert_eq!(
        result,
        Err(EngineError::ResourceLimitExceeded {
            resource: TIMER_DURATION_RESOURCE,
        })
    );
    assert_eq!(frame.executed(), before);
    Ok(())
}

#[test]
fn wait_event_type_error_precedes_unrepresentable_timeout() -> Result<(), EngineError> {
    let mut frame = frame_with(SlotIdx::ZERO, SlotValue::Bool(true))?;
    frame.write_slot(SlotIdx::new(1), huge_timeout()?)?;

    assert_eq!(
        wait_event(&mut frame, SlotIdx::ZERO, Some(SlotIdx::new(1))),
        Err(EngineError::TypeMismatch {
            expected: "event",
            found: "boolean",
        })
    );
    Ok(())
}

#[test]
fn ask_prompt_error_precedes_unrepresentable_timeout() -> Result<(), EngineError> {
    let mut frame = frame_with(SlotIdx::ZERO, SlotValue::Bool(true))?;
    frame.write_slot(SlotIdx::new(1), huge_timeout()?)?;

    assert_eq!(
        ask(&mut frame, SlotIdx::ZERO, Some(SlotIdx::new(1))),
        Err(EngineError::TypeMismatch {
            expected: "prompt",
            found: "boolean",
        })
    );
    Ok(())
}
