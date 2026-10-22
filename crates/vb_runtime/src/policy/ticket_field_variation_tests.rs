//! Tests for digest sensitivity to each ticket field.
//! Each test varies exactly one field and verifies the digest changes.

use super::*;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

fn baseline_ticket() -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 10,
    }
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
        ActionTicket {
            run: RunId::new(2),
            ..baseline_ticket()
        },
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
        ActionTicket {
            step: StepIdx::new(5),
            ..baseline_ticket()
        },
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
        ActionTicket {
            seq: SeqNo::new(3),
            ..baseline_ticket()
        },
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
        ActionTicket {
            action: ActionId::new(7),
            ..baseline_ticket()
        },
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
        ActionTicket {
            run_attempt: 3,
            ..baseline_ticket()
        },
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
        ActionTicket {
            attempt: 2,
            ..baseline_ticket()
        },
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
        ActionTicket {
            idempotency_key: 999,
            ..baseline_ticket()
        },
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
        ActionTicket {
            capacity: 20,
            ..baseline_ticket()
        },
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
