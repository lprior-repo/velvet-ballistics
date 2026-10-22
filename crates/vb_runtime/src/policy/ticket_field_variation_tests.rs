//! Tests for digest sensitivity to each ticket field.
//! Each test varies exactly one field and verifies the digest changes.

use super::*;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

fn make_ticket(
    run: u64,
    step: u16,
    seq: u64,
    action: u16,
    run_attempt: u16,
    attempt: u16,
    idempotency_key: u128,
    capacity: u16,
) -> ActionTicket {
    ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action),
        run_attempt,
        attempt,
        idempotency_key,
        capacity,
    }
}

fn baseline_ticket() -> ActionTicket {
    make_ticket(1, 0, 0, 0, 1, 1, 0, 10)
}

#[test]
fn varying_run_field_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let baseline = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    let varied = policy.evaluate(
        make_ticket(2, 0, 0, 0, 1, 1, 0, 10),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_ne!(baseline.input_digest, varied.input_digest);
    assert_eq!(varied.ticket.run, RunId::new(2));
    Ok(())
}

#[test]
fn varying_step_field_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let baseline = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    let varied = policy.evaluate(
        make_ticket(1, 5, 0, 0, 1, 1, 0, 10),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_ne!(baseline.input_digest, varied.input_digest);
    assert_eq!(varied.ticket.step, StepIdx::new(5));
    Ok(())
}

#[test]
fn varying_seq_field_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let baseline = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    let varied = policy.evaluate(
        make_ticket(1, 0, 3, 0, 1, 1, 0, 10),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_ne!(baseline.input_digest, varied.input_digest);
    assert_eq!(varied.ticket.seq, SeqNo::new(3));
    Ok(())
}

#[test]
fn varying_action_field_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let baseline = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    let varied = policy.evaluate(
        make_ticket(1, 0, 0, 7, 1, 1, 0, 10),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_ne!(baseline.input_digest, varied.input_digest);
    assert_eq!(varied.ticket.action, ActionId::new(7));
    Ok(())
}

#[test]
fn varying_run_attempt_field_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let baseline = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    let varied = policy.evaluate(
        make_ticket(1, 0, 0, 0, 3, 1, 0, 10),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_ne!(baseline.input_digest, varied.input_digest);
    assert_eq!(varied.ticket.run_attempt, 3);
    Ok(())
}

#[test]
fn varying_attempt_field_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let baseline = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    let varied = policy.evaluate(
        make_ticket(1, 0, 0, 0, 1, 2, 0, 10),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_ne!(baseline.input_digest, varied.input_digest);
    assert_eq!(varied.ticket.attempt, 2);
    Ok(())
}

#[test]
fn varying_idempotency_key_field_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let baseline = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    let varied = policy.evaluate(
        make_ticket(1, 0, 0, 0, 1, 1, 999, 10),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_ne!(baseline.input_digest, varied.input_digest);
    assert_eq!(varied.ticket.idempotency_key, 999);
    Ok(())
}

#[test]
fn varying_capacity_field_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let baseline = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    let varied = policy.evaluate(
        make_ticket(1, 0, 0, 0, 1, 1, 0, 20),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_ne!(baseline.input_digest, varied.input_digest);
    assert_eq!(varied.ticket.capacity, 20);
    Ok(())
}

#[test]
fn varying_input_bytes_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let r1 = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    let r2 = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"different input",
        Taint::Clean,
    )?;
    let r3 = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"",
        Taint::Clean,
    )?;

    assert_ne!(r1.input_digest, r2.input_digest);
    assert_ne!(r1.input_digest, r3.input_digest);
    assert_ne!(r2.input_digest, r3.input_digest);
    Ok(())
}
