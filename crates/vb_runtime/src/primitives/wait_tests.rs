//! Tests for wait primitives.

use vb_core::value::SlotValue;

use super::{wait_event, wait_until};
use vb_core::ids::SlotIdx;
use vb_core::EngineSignal;

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(4, 8)
}

#[test]
fn wait_until_returns_awaiting_wait() {
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(1000))
        .expect("slot write must succeed");

    let result = wait_until(&mut run, deadline);

    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
}

#[test]
fn wait_event_returns_awaiting_wait() {
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(event, SlotValue::I64(1))
        .expect("slot write must succeed");
    run.write_slot(timeout, SlotValue::I64(500))
        .expect("slot write must succeed");

    let result = wait_event(&mut run, event, Some(timeout));

    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
}

#[test]
fn wait_event_returns_awaiting_wait_without_timeout() {
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    run.write_slot(event, SlotValue::I64(1))
        .expect("write must succeed");
    let result = wait_event(&mut run, event, None);
    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
}

#[test]
fn wait_event_reads_timeout_when_provided() {
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(event, SlotValue::I64(1))
        .expect("write must succeed");
    run.write_slot(timeout, SlotValue::I64(500))
        .expect("write must succeed");
    let result = wait_event(&mut run, event, Some(timeout));
    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
}

#[test]
fn wait_until_does_not_change_pc() {
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(1000))
        .expect("write must succeed");
    let pc_before = run.pc();
    let result = wait_until(&mut run, deadline);
    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
    assert_eq!(run.pc(), pc_before);
}

#[test]
fn wait_event_does_not_change_pc() {
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    run.write_slot(event, SlotValue::I64(1))
        .expect("write must succeed");
    let pc_before = run.pc();
    let result = wait_event(&mut run, event, None);
    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
    assert_eq!(run.pc(), pc_before);
}

#[test]
fn wait_until_negative_deadline_returns_awaiting_wait() {
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(-1))
        .expect("write");
    let result = wait_until(&mut run, deadline);
    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
}

#[test]
fn wait_until_zero_deadline_returns_awaiting_wait() {
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(0))
        .expect("write");
    let result = wait_until(&mut run, deadline);
    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
}

#[test]
fn wait_event_negative_timeout_returns_awaiting_wait() {
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(event, SlotValue::I64(1))
        .expect("write");
    run.write_slot(timeout, SlotValue::I64(-999))
        .expect("write");
    let result = wait_event(&mut run, event, Some(timeout));
    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
}

#[test]
fn wait_until_does_not_increment_executed_counter() {
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::I64(1000))
        .expect("write");
    let before = run.executed();
    let result = wait_until(&mut run, deadline);
    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
    assert_eq!(run.executed(), before);
}

#[test]
fn wait_event_does_not_increment_executed_counter() {
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    run.write_slot(event, SlotValue::I64(1))
        .expect("write");
    let before = run.executed();
    let result = wait_event(&mut run, event, None);
    assert_eq!(result, Ok(EngineSignal::AwaitingWait));
    assert_eq!(run.executed(), before);
}

#[test]
fn wait_until_returns_error_when_slot_uninitialized() {
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    let result = wait_until(&mut run, deadline);
    assert!(result.is_err());
}

#[test]
fn wait_event_returns_error_when_event_slot_uninitialized() {
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    let result = wait_event(&mut run, event, None);
    assert!(result.is_err());
}

#[test]
fn wait_event_returns_error_when_timeout_slot_uninitialized() {
    let mut run = fresh_frame();
    let event = SlotIdx::new(0);
    let timeout = SlotIdx::new(1);
    run.write_slot(event, SlotValue::I64(1))
        .expect("write must succeed");
    let result = wait_event(&mut run, event, Some(timeout));
    assert!(result.is_err());
}

#[test]
fn wait_until_with_bool_deadline_returns_type_mismatch() {
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::Bool(true))
        .expect("write");
    let result = wait_until(&mut run, deadline);
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "deadline",
            found: "boolean",
        })
    );
}

#[test]
fn wait_until_with_symbol_deadline_returns_type_mismatch() {
    let mut run = fresh_frame();
    let deadline = SlotIdx::new(0);
    run.write_slot(deadline, SlotValue::Symbol(vb_core::ids::SymbolId::new(42)))
        .expect("write");
    let result = wait_until(&mut run, deadline);
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "deadline",
            found: "symbol",
        })
    );
}
