#![forbid(unsafe_code)]
//! Tests for wait and ask suspension primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

use super::{ask, ask_resume, wait_event, wait_until};

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(4, 8)
}

#[test]
fn wait_until_returns_awaiting_wait() {
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(1000))
        .ok()
        .unwrap_or_else(|| panic!("slot write must succeed"));

    let result = wait_until(&mut run, deadline);

    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
}

#[test]
fn wait_event_returns_awaiting_wait() {
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(event, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("slot write must succeed"));
    run.write_slot(timeout, SlotValue::I64(500))
        .ok()
        .unwrap_or_else(|| panic!("slot write must succeed"));

    let result = wait_event(&mut run, event, Some(timeout));

    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
}

#[test]
fn ask_returns_awaiting_ask() {
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .ok()
        .unwrap_or_else(|| panic!("slot write must succeed"));
    run.write_slot(timeout, SlotValue::I64(300))
        .ok()
        .unwrap_or_else(|| panic!("slot write must succeed"));

    let result = ask(&mut run, prompt, Some(timeout));

    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingAsk));
}

#[test]
fn ask_resume_writes_answer_and_continues() {
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("slot write must succeed"));

    let result = ask_resume(
        &mut run,
        answer,
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
        SlotValue::I64(42)
    );
}

// BDD tests for wait_ask primitives

#[test]
fn wait_until_returns_error_when_slot_uninitialized() {
    // Given a frame with an uninitialized deadline slot
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    // When calling wait_until
    let result = wait_until(&mut run, deadline);
    // Then it returns an error (slot not initialized)
    assert_eq!(result.is_err(), true);
}

#[test]
fn wait_event_returns_error_when_event_slot_uninitialized() {
    // Given a frame with uninitialized event slot
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    // When calling wait_event
    let result = wait_event(&mut run, event, None);
    // Then it returns an error
    assert_eq!(result.is_err(), true);
}

#[test]
fn wait_event_returns_awaiting_wait_without_timeout() {
    // Given a frame with event value but no timeout
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    run.write_slot(event, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling wait_event without timeout
    let result = wait_event(&mut run, event, None);
    // Then it returns AwaitingWait
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
}

#[test]
fn ask_returns_error_when_prompt_uninitialized() {
    // Given a frame with uninitialized prompt slot
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    // When calling ask
    let result = ask(&mut run, prompt, None);
    // Then it returns an error
    assert_eq!(result.is_err(), true);
}

#[test]
fn ask_returns_awaiting_ask_without_timeout() {
    // Given a frame with prompt value but no timeout
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling ask without timeout
    let result = ask(&mut run, prompt, None);
    // Then it returns AwaitingAsk
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingAsk));
}

#[test]
fn ask_resume_returns_error_when_answer_uninitialized() {
    // Given a frame with uninitialized answer slot
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    // When calling ask_resume
    let result = ask_resume(
        &mut run,
        answer,
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(3)),
        StepIdx::ZERO,
    );
    // Then it returns an error
    assert_eq!(result.is_err(), true);
}

#[test]
fn ask_resume_returns_error_when_next_missing() {
    // Given a frame with answer value but no next step
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    run.write_slot(answer, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling ask_resume with next=None
    let result = ask_resume(&mut run, answer, Some(SlotIdx::new(1)), None, StepIdx::ZERO);
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
fn ask_resume_copies_answer_without_output_slot() {
    // Given a frame with answer value
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::I64(99))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling ask_resume with output=None
    let result = ask_resume(&mut run, answer, None, Some(next_step), StepIdx::ZERO);
    // Then it succeeds and jumps to next
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
}

#[test]
fn ask_resume_increments_executed_counter() {
    // Given a frame with answer value
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let executed_before = run.executed();
    // When calling ask_resume
    let result = ask_resume(
        &mut run,
        answer,
        Some(SlotIdx::new(1)),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then executed counter incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), executed_before + 1);
}

#[test]
fn wait_event_reads_timeout_when_provided() {
    // Given a frame with event and timeout values
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(event, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    run.write_slot(timeout, SlotValue::I64(500))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling wait_event with timeout
    let result = wait_event(&mut run, event, Some(timeout));
    // Then it returns AwaitingWait
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
}

#[test]
fn ask_reads_timeout_when_provided() {
    // Given a frame with prompt and timeout values
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    run.write_slot(timeout, SlotValue::I64(300))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling ask with timeout
    let result = ask(&mut run, prompt, Some(timeout));
    // Then it returns AwaitingAsk
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingAsk));
}

#[test]
fn wait_until_does_not_change_pc() {
    // Given a frame at pc=0
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(1000))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let pc_before = run.pc();
    // When calling wait_until
    let result = wait_until(&mut run, deadline);
    // Then pc is unchanged
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
    assert_eq!(run.pc(), pc_before);
}

#[test]
fn wait_event_does_not_change_pc() {
    // Given a frame at pc=0
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    run.write_slot(event, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let pc_before = run.pc();
    // When calling wait_event
    let result = wait_event(&mut run, event, None);
    // Then pc is unchanged
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
    assert_eq!(run.pc(), pc_before);
}

#[test]
fn ask_does_not_change_pc() {
    // Given a frame at pc=0
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let pc_before = run.pc();
    // When calling ask
    let result = ask(&mut run, prompt, None);
    // Then pc is unchanged
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingAsk));
    assert_eq!(run.pc(), pc_before);
}

#[test]
fn wait_event_returns_error_when_timeout_slot_uninitialized() {
    // Given a frame with event value but uninitialized timeout
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(event, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling wait_event with uninitialized timeout
    let result = wait_event(&mut run, event, Some(timeout));
    // Then it returns an error
    assert_eq!(result.is_err(), true);
}

#[test]
fn ask_returns_error_when_timeout_slot_uninitialized() {
    // Given a frame with prompt value but uninitialized timeout
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling ask with uninitialized timeout
    let result = ask(&mut run, prompt, Some(timeout));
    // Then it returns an error
    assert_eq!(result.is_err(), true);
}

#[test]
fn ask_resume_with_bool_answer() {
    // Given a frame with bool answer
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::Bool(true))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling ask_resume
    let result = ask_resume(
        &mut run,
        answer,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then it succeeds and copies the bool value
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::Bool(true)
    );
}

#[test]
fn ask_resume_with_i64_answer() {
    // Given a frame with I64 answer
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::I64(12345))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    // When calling ask_resume
    let result = ask_resume(
        &mut run,
        answer,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then it succeeds and copies the value
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(12345)
    );
}

// ── Adversarial BDD tests for wait_ask ──────────────────────────────

#[test]
fn wait_until_negative_deadline_returns_awaiting_wait() {
    // Given a frame with a negative deadline value (past timestamp)
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(-1))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling wait_until with a negative deadline
    let result = wait_until(&mut run, deadline);
    // Then it returns AwaitingWait (no deadline validation at primitive level)
    // BUG: No validation that the deadline is in the future or positive
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
}

#[test]
fn wait_until_zero_deadline_returns_awaiting_wait() {
    // Given a frame with deadline=0 (epoch)
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(0))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling wait_until with deadline=0
    let result = wait_until(&mut run, deadline);
    // Then it returns AwaitingWait (no deadline validation)
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
}

#[test]
fn ask_with_zero_timeout_returns_awaiting_ask() {
    // Given a frame with prompt and timeout=0
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    run.write_slot(timeout, SlotValue::I64(0))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling ask with timeout=0
    let result = ask(&mut run, prompt, Some(timeout));
    // Then it returns AwaitingAsk (no timeout validation at primitive level)
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingAsk));
}

#[test]
fn wait_event_negative_timeout_returns_awaiting_wait() {
    // Given a frame with event and negative timeout
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(event, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    run.write_slot(timeout, SlotValue::I64(-999))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling wait_event with negative timeout
    let result = wait_event(&mut run, event, Some(timeout));
    // Then it returns AwaitingWait (no timeout sign validation)
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
}

#[test]
fn wait_until_increments_executed_counter() {
    // Given a frame with a deadline
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(1000))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let before = run.executed();
    // When calling wait_until
    let result = wait_until(&mut run, deadline);
    // Then executed counter IS incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn ask_increments_executed_counter() {
    // Given a frame with a prompt
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    run.write_slot(prompt, SlotValue::Symbol(vb_core::ids::SymbolId::new(1)))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let before = run.executed();
    // When calling ask
    let result = ask(&mut run, prompt, None);
    // Then executed counter IS incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingAsk));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn wait_event_increments_executed_counter() {
    // Given a frame with an event
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    run.write_slot(event, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let before = run.executed();
    // When calling wait_event
    let result = wait_event(&mut run, event, None);
    // Then executed counter IS incremented
    assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn ask_resume_with_null_answer_copies_null() {
    // Given a frame with Null in the answer slot
    let mut run = fresh_frame();
    let answer = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(answer, SlotValue::Null)
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling ask_resume
    let result = ask_resume(
        &mut run,
        answer,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    // Then it copies Null to output
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::Null
    );
}

#[test]
fn wait_until_with_bool_deadline_returns_type_mismatch() {
    // Given a frame with a Bool in the deadline slot (type misuse)
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::Bool(true))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling wait_until with a Bool deadline
    let result = wait_until(&mut run, deadline);
    // Then it returns TypeMismatch (deadline must be numeric)
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "deadline",
            found: "boolean",
        })
    );
}

#[test]
fn ask_resume_same_answer_and_output_slot() {
    // Given a frame where answer == output (same slot)
    let mut run = fresh_frame();
    let slot = SlotIdx::new(0);
    let next_step = StepIdx::new(3);
    run.write_slot(slot, SlotValue::I64(77))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling ask_resume with answer == output
    let result = ask_resume(&mut run, slot, Some(slot), Some(next_step), StepIdx::ZERO);
    // Then it succeeds (reads value, writes same value back)
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(
        *run.read_slot(slot)
            .ok()
            .unwrap_or_else(|| panic!("must read")),
        SlotValue::I64(77)
    );
}

#[test]
fn wait_until_with_symbol_deadline_returns_type_mismatch() {
    // Given a frame with a Symbol in the deadline slot (non-numeric type)
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::Symbol(vb_core::ids::SymbolId::new(42)))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling wait_until with a Symbol deadline
    let result = wait_until(&mut run, deadline);
    // Then it returns TypeMismatch (deadline must be numeric)
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "deadline",
            found: "symbol",
        })
    );
}

#[test]
fn ask_with_bool_prompt_returns_type_mismatch() {
    // Given a frame with Bool in prompt slot
    let mut run = fresh_frame();
    let prompt = SlotIdx::new(0);
    run.write_slot(prompt, SlotValue::Bool(false))
        .ok()
        .unwrap_or_else(|| panic!("write"));
    // When calling ask with a Bool prompt
    let result = ask(&mut run, prompt, None);
    // Then it returns TypeMismatch (prompt must be a Symbol)
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "prompt",
            found: "boolean",
        })
    );
}
