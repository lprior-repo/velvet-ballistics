//! Tests for ask primitives.

use vb_core::value::SlotValue;

use super::{ask, ask_resume};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::EngineSignal;

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(4, 8)
}

#[test]
fn ask_returns_awaiting_ask() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .expect("slot write must succeed");
    run.write_slot(timeout, SlotValue::I64(300))
        .expect("slot write must succeed");

    let result = ask(&mut run, prompt, Some(timeout));

    assert_eq!(result, Ok(EngineSignal::AwaitingAsk));
}

#[test]
fn ask_returns_awaiting_ask_without_timeout() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .expect("write must succeed");
    let result = ask(&mut run, prompt, None);
    assert_eq!(result, Ok(EngineSignal::AwaitingAsk));
}

#[test]
fn ask_reads_timeout_when_provided() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .expect("write must succeed");
    run.write_slot(timeout, SlotValue::I64(300))
        .expect("write must succeed");
    let result = ask(&mut run, prompt, Some(timeout));
    assert_eq!(result, Ok(EngineSignal::AwaitingAsk));
}

#[test]
fn ask_does_not_change_pc() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .expect("write must succeed");
    let pc_before = run.pc();
    let result = ask(&mut run, prompt, None);
    assert_eq!(result, Ok(EngineSignal::AwaitingAsk));
    assert_eq!(run.pc(), pc_before);
}

#[test]
fn ask_returns_error_when_prompt_uninitialized() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    let result = ask(&mut run, prompt, None);
    assert!(result.is_err());
}

#[test]
fn ask_returns_error_when_timeout_slot_uninitialized() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .expect("write must succeed");
    let result = ask(&mut run, prompt, Some(timeout));
    assert!(result.is_err());
}

#[test]
fn ask_with_zero_timeout_returns_awaiting_ask() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .expect("write");
    run.write_slot(timeout, SlotValue::I64(0))
        .expect("write");
    let result = ask(&mut run, prompt, Some(timeout));
    assert_eq!(result, Ok(EngineSignal::AwaitingAsk));
}

#[test]
fn ask_does_not_increment_executed_counter() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .expect("write must succeed");
    let before = run.executed();
    let result = ask(&mut run, prompt, None);
    assert_eq!(result, Ok(EngineSignal::AwaitingAsk));
    assert_eq!(run.executed(), before);
}

#[test]
fn ask_with_bool_prompt_returns_type_mismatch() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    run.write_slot(prompt, SlotValue::Bool(false))
        .expect("write");
    let result = ask(&mut run, prompt, None);
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "prompt",
            found: "boolean",
        })
    );
}

// ── AskResume tests ──────────────────────────────────────────────────

#[test]
fn ask_resume_writes_answer_and_continues() {
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::I64(42))
        .expect("slot write must succeed");

    let result = ask_resume(
        &mut run,
        answer,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );

    assert_eq!(result, Ok(EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
    assert_eq!(
        *run.read_slot(output).expect("read must succeed"),
        SlotValue::I64(42)
    );
}

#[test]
fn ask_resume_returns_error_when_answer_uninitialized() {
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let result = ask_resume(
        &mut run,
        answer,
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(3)),
        StepIdx::ZERO,
    );
    assert!(result.is_err());
}

#[test]
fn ask_resume_returns_error_when_next_missing() {
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    run.write_slot(answer, SlotValue::I64(42))
        .expect("write must succeed");
    let result = ask_resume(&mut run, answer, Some(SlotIdx::new(1)), None, StepIdx::ZERO);
    match result {
        Err(EngineError::MissingNextStep { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(EngineSignal::Continue));
        }
    }
}

#[test]
fn ask_resume_copies_answer_without_output_slot() {
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::I64(99))
        .expect("write must succeed");
    let result = ask_resume(&mut run, answer, None, Some(next_step), StepIdx::ZERO);
    assert_eq!(result, Ok(EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
}

#[test]
fn ask_resume_increments_executed_counter() {
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::I64(42))
        .expect("write must succeed");
    let executed_before = run.executed();
    let result = ask_resume(
        &mut run,
        answer,
        Some(SlotIdx::new(1)),
        Some(next_step),
        StepIdx::ZERO,
    );
    assert_eq!(result, Ok(EngineSignal::Continue));
    assert_eq!(run.executed(), executed_before + 1);
}

#[test]
fn ask_resume_with_bool_answer() {
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::Bool(true))
        .expect("write must succeed");
    let result = ask_resume(
        &mut run,
        answer,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    assert_eq!(result, Ok(EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
    assert_eq!(
        *run.read_slot(output).expect("read must succeed"),
        SlotValue::Bool(true)
    );
}

#[test]
fn ask_resume_with_i64_answer() {
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(5);
    run.write_slot(answer, SlotValue::I64(12345))
        .expect("write must succeed");
    let result = ask_resume(
        &mut run,
        answer,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    assert_eq!(result, Ok(EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(output).expect("read must succeed"),
        SlotValue::I64(12345)
    );
}

#[test]
fn ask_resume_with_null_answer_copies_null() {
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::Null)
        .expect("write");
    let result = ask_resume(
        &mut run,
        answer,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    assert_eq!(result, Ok(EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(output).expect("must read"),
        SlotValue::Null
    );
}

#[test]
fn ask_resume_same_answer_and_output_slot() {
    let mut run = fresh_frame();
    let slot = SlotIdx::new(0);
    let next_step = StepIdx::new(3);
    run.write_slot(slot, SlotValue::I64(77))
        .expect("write");
    let result = ask_resume(&mut run, slot, Some(slot), Some(next_step), StepIdx::ZERO);
    assert_eq!(result, Ok(EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(slot).expect("must read"),
        SlotValue::I64(77)
    );
}
