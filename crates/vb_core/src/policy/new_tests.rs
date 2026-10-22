//! Tests for the new typed policy types.

use super::*;
use crate::ids::{RunId, SeqNo, StepIdx};

fn test_ticket() -> ActionTicket {
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
fn policy_digest_roundtrip() {
    let digest = test_digest();
    let expected: [u8; 32] = [
        0xAB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0xCD,
    ];
    assert_eq!(digest.as_bytes().as_ref(), expected.as_ref());
}

#[test]
fn decision_code_variants_are_distinct() {
    assert_ne!(DecisionCode::Allowed, DecisionCode::Denied);
    assert_ne!(DecisionCode::Allowed, DecisionCode::Suspended);
    assert_ne!(DecisionCode::Denied, DecisionCode::Suspended);
}

#[test]
fn decision_code_discriminants() -> Result<(), Box<dyn std::error::Error>> {
    let allowed_bytes = postcard::to_allocvec(&DecisionCode::Allowed)?;
    let denied_bytes = postcard::to_allocvec(&DecisionCode::Denied)?;
    let suspended_bytes = postcard::to_allocvec(&DecisionCode::Suspended)?;

    assert_eq!(allowed_bytes, vec![0u8]);
    assert_eq!(denied_bytes, vec![1u8]);
    assert_eq!(suspended_bytes, vec![2u8]);
    Ok(())
}

#[test]
fn policy_layer_variants_are_distinct() {
    assert_ne!(PolicyLayer::Live, PolicyLayer::Admission);
    assert_ne!(PolicyLayer::Live, PolicyLayer::Emergency);
    assert_ne!(PolicyLayer::Admission, PolicyLayer::Emergency);
}

#[test]
fn policy_layer_discriminants() -> Result<(), Box<dyn std::error::Error>> {
    let live_bytes = postcard::to_allocvec(&PolicyLayer::Live)?;
    let admission_bytes = postcard::to_allocvec(&PolicyLayer::Admission)?;
    let emergency_bytes = postcard::to_allocvec(&PolicyLayer::Emergency)?;

    assert_eq!(live_bytes, vec![0u8]);
    assert_eq!(admission_bytes, vec![1u8]);
    assert_eq!(emergency_bytes, vec![2u8]);
    Ok(())
}

#[test]
fn current_live_policy_unconfigured_has_no_digest() {
    let live = CurrentLivePolicy::Unconfigured;
    assert_eq!(live.digest(), None);
    assert_eq!(live.rule(), None);
}

#[test]
fn current_live_policy_configured_has_digest() {
    let digest = test_digest();
    let live = CurrentLivePolicy::Configured {
        digest,
        rule: PolicyRule {
            action: ActionId::new(0),
            code: DecisionCode::Allowed,
        },
    };
    assert_eq!(live.digest(), Some(digest));
    assert!(matches!(live.rule(), Some(rule) if rule.action == ActionId::new(0)));
    let identity = live.identity();
    assert!(
        matches!(identity, CurrentPolicyIdentity::Configured { digest: id_digest } if id_digest == digest)
    );
}

#[test]
fn decision_record_preserves_all_fields() {
    let ticket = test_ticket();
    let admission = PinnedAdmissionPolicy::Artifact(test_digest());
    let live_identity = CurrentPolicyIdentity::Unconfigured;
    let input_digest = test_digest();

    let record = DecisionRecord {
        ticket,
        admission,
        current_live: live_identity,
        input_digest,
        taint: Taint::Clean,
        outcome: DecisionCode::Denied,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    };

    assert_eq!(record.ticket, ticket);
    assert_eq!(record.admission, admission);
    assert_eq!(record.current_live, live_identity);
    assert_eq!(record.input_digest, input_digest);
    assert_eq!(record.taint, Taint::Clean);
    assert_eq!(record.outcome, DecisionCode::Denied);
    assert_eq!(record.layer, PolicyLayer::Live);
    assert_eq!(record.emergency_influence, InfluenceState::Inactive);
    assert_eq!(record.compat_influence, InfluenceState::NotEvaluated);
}

#[test]
fn pinned_admission_unverified_is_not_digest() {
    let unverified = PinnedAdmissionPolicy::Unverified;
    assert!(matches!(unverified, PinnedAdmissionPolicy::Unverified));
}

#[test]
fn influence_state_variants_are_distinct() {
    assert_ne!(InfluenceState::NotEvaluated, InfluenceState::Active);
    assert_ne!(InfluenceState::NotEvaluated, InfluenceState::Inactive);
    assert_ne!(InfluenceState::Inactive, InfluenceState::Active);
}
