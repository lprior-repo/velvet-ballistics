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
    assert_eq!(digest.as_bytes()[0], 0xAB);
    assert_eq!(digest.as_bytes()[31], 0xCD);
}

#[test]
fn decision_code_discriminants() {
    assert_eq!(DecisionCode::Allowed as u8, 0);
    assert_eq!(DecisionCode::Denied as u8, 1);
    assert_eq!(DecisionCode::Suspended as u8, 2);
}

#[test]
fn policy_layer_discriminants() {
    assert_eq!(PolicyLayer::Live as u8, 0);
    assert_eq!(PolicyLayer::Admission as u8, 1);
    assert_eq!(PolicyLayer::Emergency as u8, 2);
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
    assert_eq!(live.rule().unwrap().action, ActionId::new(0));
    let identity = live.identity();
    match identity {
        CurrentPolicyIdentity::Configured { digest: id_digest } => {
            assert_eq!(id_digest, digest);
        }
        CurrentPolicyIdentity::Unconfigured => panic!("should be configured"),
    }
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

    assert_eq!(record.ticket, record.ticket);
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
    match unverified {
        PinnedAdmissionPolicy::Artifact(_) => panic!("should be unverified"),
        PinnedAdmissionPolicy::Unverified => {}
    }
}

#[test]
fn influence_state_values() {
    assert_eq!(InfluenceState::NotEvaluated, InfluenceState::NotEvaluated);
    assert_ne!(InfluenceState::NotEvaluated, InfluenceState::Active);
    assert_ne!(InfluenceState::Inactive, InfluenceState::Active);
}
