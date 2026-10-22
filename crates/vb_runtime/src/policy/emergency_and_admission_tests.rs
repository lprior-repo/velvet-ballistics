//! Tests for admission policy pinning and emergency overlay behavior.

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

    match record_a.admission {
        PinnedAdmissionPolicy::Artifact(d) => {
            assert_eq!(d.as_bytes(), digest_a.as_bytes());
        }
        PinnedAdmissionPolicy::Unverified => panic!("admission should be Artifact"),
    }
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
