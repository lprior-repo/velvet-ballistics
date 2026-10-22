#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::*;
use crate::{EventSeq, RecordKind};
use vb_core::{
    RunId, WorkflowDigest,
    policy::{DecisionCode, PolicyDigest, PolicyLayer},
};

fn make_policy_decision_event(
    run: RunId,
    seq: u64,
    layer: PolicyLayer,
    code: DecisionCode,
) -> JournalEvent {
    JournalEvent::PolicyDecisionEvent {
        run,
        seq: EventSeq::new(seq),
        layer,
        code,
        policy_digest: PolicyDigest(WorkflowDigest::from_bytes([1u8; 32])),
        input_digest: PolicyDigest(WorkflowDigest::from_bytes([2u8; 32])),
        emergency_deny_active: false,
        compatibility_influence: false,
    }
}

#[test]
fn policy_decision_event_serialize_deserialize_roundtrip() {
    let event = make_policy_decision_event(RunId::new(1), 42, PolicyLayer::LiveEffect, DecisionCode::Allowed);

    // Verify record kind mapping
    assert_eq!(event.record_kind(), RecordKind::PolicyDecision);
    assert_eq!(event.record_kind().id(), 38);

    // Serialize
    let encoded = postcard::to_allocvec(&event).expect("policy decision event serialize should succeed");

    // Verify tag byte is correct (38 = 0x26)
    assert_eq!(encoded[0], 38);

    // Deserialize
    let decoded: JournalEvent = postcard::from_bytes(&encoded).expect("policy decision event deserialize should succeed");

    // Verify roundtrip
    assert_eq!(decoded.run_id(), RunId::new(1));
    assert_eq!(decoded.seq().get(), 42);

    // Verify all fields match
    match &decoded {
        JournalEvent::PolicyDecisionEvent {
            layer,
            code,
            policy_digest,
            input_digest,
            emergency_deny_active,
            compatibility_influence,
            ..
        } => {
            assert_eq!(*layer, PolicyLayer::LiveEffect);
            assert_eq!(*code, DecisionCode::Allowed);
            assert_eq!(policy_digest.0, WorkflowDigest::from_bytes([1u8; 32]));
            assert_eq!(input_digest.0, WorkflowDigest::from_bytes([2u8; 32]));
            assert!(!*emergency_deny_active);
            assert!(!*compatibility_influence);
        }
        _ => panic!("expected PolicyDecisionEvent variant"),
    }
}

#[test]
fn policy_decision_event_all_decision_codes_roundtrip() {
    let codes = [DecisionCode::Allowed, DecisionCode::Denied, DecisionCode::Suspended];

    for (i, code) in codes.iter().enumerate() {
        let event = make_policy_decision_event(
            RunId::new(100 + i as u64),
            50 + i as u64,
            PolicyLayer::Admission,
            *code,
        );

        let encoded = postcard::to_allocvec(&event).expect("serialize should succeed");
        let decoded: JournalEvent = postcard::from_bytes(&encoded).expect("deserialize should succeed");

        match &decoded {
            JournalEvent::PolicyDecisionEvent {
                code: decoded_code, ..
            } => {
                assert_eq!(*decoded_code, *code, "decision code should roundtrip");
            }
            _ => panic!("expected PolicyDecisionEvent variant"),
        }
    }
}

#[test]
fn policy_decision_event_all_layers_roundtrip() {
    let layers = [
        PolicyLayer::Admission,
        PolicyLayer::Replay,
        PolicyLayer::LiveEffect,
        PolicyLayer::EmergencyDeny,
    ];

    for (i, layer) in layers.iter().enumerate() {
        let event = make_policy_decision_event(
            RunId::new(200 + i as u64),
            60 + i as u64,
            *layer,
            DecisionCode::Allowed,
        );

        let encoded = postcard::to_allocvec(&event).expect("serialize should succeed");
        let decoded: JournalEvent = postcard::from_bytes(&encoded).expect("deserialize should succeed");

        match &decoded {
            JournalEvent::PolicyDecisionEvent {
                layer: decoded_layer, ..
            } => {
                assert_eq!(*decoded_layer, *layer, "policy layer should roundtrip");
            }
            _ => panic!("expected PolicyDecisionEvent variant"),
        }
    }
}

#[test]
fn policy_decision_event_influence_flags_roundtrip() {
    // Test both true and false combinations
    let cases = [
        (false, false),
        (true, false),
        (false, true),
        (true, true),
    ];

    for (i, (emergency, compat)) in cases.iter().enumerate() {
        let event = JournalEvent::PolicyDecisionEvent {
            run: RunId::new(300 + i as u64),
            seq: EventSeq::new(70 + i as u64),
            layer: PolicyLayer::LiveEffect,
            code: DecisionCode::Denied,
            policy_digest: PolicyDigest(WorkflowDigest::from_bytes([1u8; 32])),
            input_digest: PolicyDigest(WorkflowDigest::from_bytes([2u8; 32])),
            emergency_deny_active: *emergency,
            compatibility_influence: *compat,
        };

        let encoded = postcard::to_allocvec(&event).expect("serialize should succeed");
        let decoded: JournalEvent = postcard::from_bytes(&encoded).expect("deserialize should succeed");

        match &decoded {
            JournalEvent::PolicyDecisionEvent {
                emergency_deny_active: decoded_emergency,
                compatibility_influence: decoded_compat,
                ..
            } => {
                assert_eq!(*decoded_emergency, *emergency, "emergency_deny_active should roundtrip");
                assert_eq!(
                    *decoded_compat,
                    *compat,
                    "compatibility_influence should roundtrip"
                );
            }
            _ => panic!("expected PolicyDecisionEvent variant"),
        }
    }
}

#[test]
fn policy_decision_event_distinct_digests_roundtrip() {
    let policy_digest = PolicyDigest(WorkflowDigest::from_bytes([3u8; 32]));
    let input_digest = PolicyDigest(WorkflowDigest::from_bytes([4u8; 32]));

    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(400),
        seq: EventSeq::new(80),
        layer: PolicyLayer::Replay,
        code: DecisionCode::Suspended,
        policy_digest,
        input_digest,
        emergency_deny_active: true,
        compatibility_influence: true,
    };

    let encoded = postcard::to_allocvec(&event).expect("serialize should succeed");
    let decoded: JournalEvent = postcard::from_bytes(&encoded).expect("deserialize should succeed");

    match &decoded {
        JournalEvent::PolicyDecisionEvent {
            policy_digest: decoded_policy,
            input_digest: decoded_input,
            ..
        } => {
            assert_eq!(*decoded_policy, policy_digest, "policy digest should roundtrip");
            assert_eq!(*decoded_input, input_digest, "input digest should roundtrip");
        }
        _ => panic!("expected PolicyDecisionEvent variant"),
    }
}

#[test]
fn policy_decision_event_truncated_payload_rejected() {
    let event = make_policy_decision_event(RunId::new(1), 42, PolicyLayer::LiveEffect, DecisionCode::Allowed);
    let encoded = postcard::to_allocvec(&event).expect("serialize should succeed");

    // Truncate payload
    let truncated = &encoded[..encoded.len() - 5];

    let result = postcard::from_bytes::<JournalEvent>(truncated);
    assert!(result.is_err(), "truncated payload should fail to deserialize");
}

#[test]
fn policy_decision_event_malformed_tag_rejected() {
    let event = make_policy_decision_event(RunId::new(1), 42, PolicyLayer::LiveEffect, DecisionCode::Allowed);
    let mut encoded = postcard::to_allocvec(&event).expect("serialize should succeed");

    // Corrupt the tag byte
    encoded[0] = 99;

    let result = postcard::from_bytes::<JournalEvent>(&encoded);
    assert!(result.is_err(), "malformed tag should fail to deserialize");
}