//! Tests for the live policy evaluator.

use super::*;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::policy::CurrentPolicyIdentity;

fn test_ticket(action: u16) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(action),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 10,
    }
}

fn test_digest() -> PolicyDigest {
    PolicyDigest::from_bytes([
        0xAB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0xCD,
    ])
}

#[test]
fn unconfigured_deny() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let ticket = test_ticket(0);
    let record = policy.evaluate(
        ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert_eq!(record.outcome, DecisionCode::Denied);
    assert_eq!(record.layer, PolicyLayer::Live);
    assert!(
        matches!(record.current_live, CurrentPolicyIdentity::Unconfigured),
        "expected unconfigured live identity"
    );
    Ok(())
}

#[test]
fn explicit_permit() -> RuntimeResult<()> {
    let mut policy = LivePolicy::new();
    policy.configure(
        test_digest(),
        PolicyRule {
            action: ActionId::new(5),
            code: DecisionCode::Allowed,
        },
    );

    let ticket = test_ticket(5);
    let record = policy.evaluate(
        ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert_eq!(record.outcome, DecisionCode::Allowed);
    assert_eq!(record.layer, PolicyLayer::Live);
    Ok(())
}

#[test]
fn explicit_deny() -> RuntimeResult<()> {
    let mut policy = LivePolicy::new();
    policy.configure(
        test_digest(),
        PolicyRule {
            action: ActionId::new(5),
            code: DecisionCode::Denied,
        },
    );

    let ticket = test_ticket(5);
    let record = policy.evaluate(
        ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert_eq!(record.outcome, DecisionCode::Denied);
    assert_eq!(record.layer, PolicyLayer::Live);
    Ok(())
}

#[test]
fn explicit_suspend() -> RuntimeResult<()> {
    let mut policy = LivePolicy::new();
    policy.configure(
        test_digest(),
        PolicyRule {
            action: ActionId::new(5),
            code: DecisionCode::Suspended,
        },
    );

    let ticket = test_ticket(5);
    let record = policy.evaluate(
        ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert_eq!(record.outcome, DecisionCode::Suspended);
    assert_eq!(record.layer, PolicyLayer::Live);
    Ok(())
}

#[test]
fn unknown_action_deny() -> RuntimeResult<()> {
    let mut policy = LivePolicy::new();
    policy.configure(
        test_digest(),
        PolicyRule {
            action: ActionId::new(5),
            code: DecisionCode::Allowed,
        },
    );

    let ticket = test_ticket(99);
    let record = policy.evaluate(
        ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert_eq!(record.outcome, DecisionCode::Denied);
    Ok(())
}

#[test]
fn emergency_overlay_denies_all() -> RuntimeResult<()> {
    let mut policy = LivePolicy::new();
    policy.configure(
        test_digest(),
        PolicyRule {
            action: ActionId::new(5),
            code: DecisionCode::Allowed,
        },
    );
    policy.activate_emergency_deny();

    let ticket = test_ticket(5);
    let record = policy.evaluate(
        ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert_eq!(record.outcome, DecisionCode::Denied);
    assert_eq!(record.layer, PolicyLayer::Emergency);
    assert_eq!(record.emergency_influence, InfluenceState::Active);
    Ok(())
}

#[test]
fn overlay_remains_restrictive_after_configure() -> RuntimeResult<()> {
    let mut policy = LivePolicy::new();
    policy.activate_emergency_deny();

    policy.configure(
        test_digest(),
        PolicyRule {
            action: ActionId::new(5),
            code: DecisionCode::Allowed,
        },
    );

    let ticket = test_ticket(5);
    let record = policy.evaluate(
        ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert_eq!(record.outcome, DecisionCode::Denied);
    assert_eq!(record.layer, PolicyLayer::Emergency);
    Ok(())
}

#[test]
fn decision_record_preserves_ticket_identity() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let expected_ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(42),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 12345,
        capacity: 10,
    };
    let record = policy.evaluate(
        expected_ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert_eq!(record.ticket, expected_ticket);
    Ok(())
}

#[test]
fn decision_record_preserves_policy_identities() -> RuntimeResult<()> {
    let admission_digest = PolicyDigest::from_bytes([
        0x11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0x12,
    ]);
    let live_digest = PolicyDigest::from_bytes([
        0x21, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0x22,
    ]);
    let admission = PinnedAdmissionPolicy::Artifact(admission_digest);
    let mut policy = LivePolicy::new();
    policy.configure(
        live_digest,
        PolicyRule {
            action: ActionId::new(0),
            code: DecisionCode::Allowed,
        },
    );

    let ticket = test_ticket(0);
    let record = policy.evaluate(ticket, admission, b"input", Taint::Clean)?;

    assert_eq!(
        record.admission,
        PinnedAdmissionPolicy::Artifact(admission_digest)
    );
    assert!(
        matches!(record.current_live, CurrentPolicyIdentity::Configured { digest } if digest == live_digest),
        "expected configured live identity with live_digest"
    );
    Ok(())
}

#[test]
fn changing_ticket_field_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();

    let t1 = test_ticket(0);
    let r1 = policy.evaluate(
        t1,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    let t2 = test_ticket(1);
    let r2 = policy.evaluate(
        t2,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_ne!(r1.input_digest, r2.input_digest);
    assert_eq!(r1.ticket.action, ActionId::new(0));
    assert_eq!(r2.ticket.action, ActionId::new(1));
    Ok(())
}

#[test]
fn changing_taint_changes_digest() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let ticket = test_ticket(0);

    let r1 = policy.evaluate(
        ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    let r2 = policy.evaluate(
        ticket,
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Secret,
    )?;

    assert_ne!(r1.input_digest, r2.input_digest);
    assert_eq!(r1.taint, Taint::Clean);
    assert_eq!(r2.taint, Taint::Secret);
    Ok(())
}

#[test]
fn missing_admission_distinguishable_from_artifact() -> RuntimeResult<()> {
    let p1 = LivePolicy::new();
    let r1 = p1.evaluate(
        test_ticket(0),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;
    assert!(matches!(r1.admission, PinnedAdmissionPolicy::Unverified));

    let p2 = LivePolicy::new();
    let r2 = p2.evaluate(
        test_ticket(0),
        PinnedAdmissionPolicy::Artifact(test_digest()),
        b"input",
        Taint::Clean,
    )?;
    assert!(matches!(r2.admission, PinnedAdmissionPolicy::Artifact(_)));
    Ok(())
}
