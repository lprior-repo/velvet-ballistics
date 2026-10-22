//! Tests for admission policy pinning and emergency overlay behavior.

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

fn test_digest() -> PolicyDigest {
    PolicyDigest::from_bytes([
        0xAB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0xCD,
    ])
}

#[test]
fn per_run_artifact_pin_remains_exact() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let digest_a = test_digest();

    let record_a = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Artifact(digest_a),
        b"input",
        Taint::Clean,
    )?;

    assert_eq!(
        record_a.admission,
        PinnedAdmissionPolicy::Artifact(digest_a)
    );
    Ok(())
}

#[test]
fn emergency_overlay_cannot_be_loosened_by_future_config() -> RuntimeResult<()> {
    let mut policy = LivePolicy::new();
    policy.activate_emergency_deny();

    let record = policy.evaluate(
        baseline_ticket(),
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
fn configured_policy_after_emergency_still_denied() -> RuntimeResult<()> {
    let mut policy = LivePolicy::new();
    policy.activate_emergency_deny();
    policy.configure(
        test_digest(),
        PolicyRule {
            action: ActionId::new(0),
            code: DecisionCode::Allowed,
        },
    );

    let record = policy.evaluate(
        baseline_ticket(),
        PinnedAdmissionPolicy::Unverified,
        b"input",
        Taint::Clean,
    )?;

    assert_eq!(record.outcome, DecisionCode::Denied);
    assert_eq!(record.layer, PolicyLayer::Emergency);
    Ok(())
}

#[test]
fn same_inputs_produce_same_decision_record() -> RuntimeResult<()> {
    let policy = LivePolicy::new();
    let admission = PinnedAdmissionPolicy::Unverified;
    let input = b"same input";
    let taint = Taint::Clean;

    let ticket = baseline_ticket();
    let r1 = policy.evaluate(ticket, admission, input, taint)?;
    let r2 = policy.evaluate(ticket, admission, input, taint)?;

    assert_eq!(r1, r2);
    Ok(())
}
