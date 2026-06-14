//! Factory function tests for `issue_action_ticket`.
//!
//! Verifies that the factory captures all fields correctly and is pure
//! (same inputs always produce the same output).

#![forbid(unsafe_code)]

use vb_core::action::issue_action_ticket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

#[test]
fn test_issue_action_ticket_captures_all_fields() {
    let ticket = issue_action_ticket(
        RunId::new(0x1234),
        StepIdx::new(0x5678),
        SeqNo::new(0x9ABC),
        ActionId::new(0xDEF0),
        42,
        0xDEAD_BEEF,
        7,
    );

    assert_eq!(ticket.run, RunId::new(0x1234), "run must be captured");
    assert_eq!(ticket.step, StepIdx::new(0x5678), "step must be captured");
    assert_eq!(ticket.seq, SeqNo::new(0x9ABC), "seq must be captured");
    assert_eq!(ticket.action, ActionId::new(0xDEF0), "action must be captured");
    assert_eq!(ticket.attempt, 42, "attempt must be captured");
    assert_eq!(
        ticket.idempotency_key,
        0xDEAD_BEEF,
        "idempotency_key must be captured"
    );
    assert_eq!(ticket.capacity, 7, "capacity must be captured");
}

#[test]
fn test_issue_action_ticket_pure() {
    let t1 = issue_action_ticket(
        RunId::new(1),
        StepIdx::new(2),
        SeqNo::new(3),
        ActionId::new(4),
        5,
        6,
        7,
    );
    let t2 = issue_action_ticket(
        RunId::new(1),
        StepIdx::new(2),
        SeqNo::new(3),
        ActionId::new(4),
        5,
        6,
        7,
    );

    assert_eq!(t1, t2, "same inputs must produce identical tickets");
}

#[test]
fn test_issue_action_ticket_different_inputs() {
    let t1 = issue_action_ticket(
        RunId::new(1),
        StepIdx::new(2),
        SeqNo::new(3),
        ActionId::new(4),
        5,
        6,
        7,
    );
    let t2 = issue_action_ticket(
        RunId::new(10),
        StepIdx::new(20),
        SeqNo::new(30),
        ActionId::new(40),
        50,
        60,
        70,
    );

    assert_ne!(t1, t2, "different inputs must produce different tickets");
}

#[test]
fn test_issue_action_ticket_zero_values() {
    let ticket = issue_action_ticket(
        RunId::new(0),
        StepIdx::new(0),
        SeqNo::new(0),
        ActionId::new(0),
        0,
        0,
        0,
    );

    assert_eq!(ticket.run.get(), 0, "zero run must be captured");
    assert_eq!(ticket.step.get(), 0, "zero step must be captured");
    assert_eq!(ticket.seq.get(), 0, "zero seq must be captured");
    assert_eq!(ticket.action.get(), 0, "zero action must be captured");
    assert_eq!(ticket.attempt, 0, "zero attempt must be captured");
    assert_eq!(ticket.idempotency_key, 0, "zero idempotency_key must be captured");
    assert_eq!(ticket.capacity, 0, "zero capacity must be captured");
}

#[test]
fn test_issue_action_ticket_max_values() {
    let ticket = issue_action_ticket(
        RunId::new(u64::MAX),
        StepIdx::new(u16::MAX),
        SeqNo::new(u64::MAX),
        ActionId::new(u16::MAX),
        u16::MAX,
        u128::MAX,
        u16::MAX,
    );

    assert_eq!(ticket.run.get(), u64::MAX, "max run must be captured");
    assert_eq!(ticket.step.get(), u16::MAX, "max step must be captured");
    assert_eq!(ticket.seq.get(), u64::MAX, "max seq must be captured");
    assert_eq!(ticket.action.get(), u16::MAX, "max action must be captured");
    assert_eq!(ticket.attempt, u16::MAX, "max attempt must be captured");
    assert_eq!(
        ticket.idempotency_key,
        u128::MAX,
        "max idempotency_key must be captured"
    );
    assert_eq!(ticket.capacity, u16::MAX, "max capacity must be captured");
}

#[test]
fn test_issue_action_ticket_returns_copy() {
    // Verifies that issue_action_ticket returns a Copy type.
    let ticket = issue_action_ticket(
        RunId::new(1),
        StepIdx::new(2),
        SeqNo::new(3),
        ActionId::new(4),
        5,
        6,
        7,
    );

    // Using ticket after it was "moved" proves Copy.
    let _copied = ticket;
    assert_eq!(ticket.run.get(), 1, "ticket must be usable after Copy");
}
