use super::*;
use crate::{FiniteF64, RunId, StepIdx, Taint};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn ask_operand_validation_preserves_error_order_and_complete_frame() -> TestResult {
    let mut frame = RunFrame::new(RunId::new(9), StepIdx::ZERO, 1, 2)?;
    let prompt = SlotIdx::ZERO;
    let timeout = SlotIdx::new(1);
    let outside = SlotIdx::MAX;
    let before = frame.checkpoint_v1();
    assert_eq!(
        validate_ask_inputs(&frame, prompt, Some(outside)),
        Err(EngineError::SlotUninitialized { slot: prompt })
    );
    assert_eq!(frame.checkpoint_v1(), before);
    frame.write_slot_with_taint(prompt, SlotValue::Bool(true), Taint::Secret)?;
    let before = frame.checkpoint_v1();
    assert_eq!(
        validate_ask_inputs(&frame, prompt, Some(outside)),
        Err(EngineError::TypeMismatch {
            expected: "prompt",
            found: "boolean"
        })
    );
    assert_eq!(frame.checkpoint_v1(), before);
    frame.write_slot(prompt, SlotValue::I64(8))?;
    let before = frame.checkpoint_v1();
    assert_eq!(
        validate_ask_inputs(&frame, prompt, Some(outside)),
        Err(EngineError::SlotOutOfBounds { slot: outside })
    );
    assert_eq!(frame.checkpoint_v1(), before);
    frame.write_slot(timeout, SlotValue::Bool(false))?;
    let before = frame.checkpoint_v1();
    assert_eq!(
        validate_ask_inputs(&frame, prompt, Some(timeout)),
        Err(EngineError::TypeMismatch {
            expected: "timeout",
            found: "boolean"
        })
    );
    assert_eq!(frame.checkpoint_v1(), before);
    Ok(())
}

#[test]
fn ask_validation_keeps_numeric_timeouts_optional_and_does_not_execute() -> TestResult {
    let mut frame = RunFrame::new(RunId::new(9), StepIdx::ZERO, 1, 2)?;
    for prompt in [
        SlotValue::Null,
        SlotValue::I64(7),
        SlotValue::F64(FiniteF64::new(1.5)?),
    ] {
        frame.write_slot_with_taint(SlotIdx::ZERO, prompt, Taint::DerivedFromSecret)?;
        let before = frame.checkpoint_v1();
        assert_eq!(validate_ask_inputs(&frame, SlotIdx::ZERO, None), Ok(()));
        assert_eq!(frame.checkpoint_v1(), before);
        for value in [SlotValue::I64(0), SlotValue::F64(FiniteF64::new(2.5)?)] {
            frame.write_slot(SlotIdx::new(1), value)?;
            let before = frame.checkpoint_v1();
            assert_eq!(
                validate_ask_inputs(&frame, SlotIdx::ZERO, Some(SlotIdx::new(1))),
                Ok(())
            );
            assert_eq!(frame.checkpoint_v1(), before);
        }
    }
    Ok(())
}
