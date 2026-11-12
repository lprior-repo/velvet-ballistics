//! Full JSON value equality tests for event_to_json with RunAdmission policy pins.
//!
//! Exercises BOTH RunPolicyPin::Pinned(distinct policy digest) and
//! RunPolicyPin::Unverified through the actual CLI replay serialization path.
//! Verifies distinct policy and input identity serialization.

#![forbid(unsafe_code)]

use super::event_to_json;
use vb_core::{CapabilitySet, RunId, RuntimePolicy, WorkflowDigest};
use vb_storage::{EventSeq, JournalEvent, RunInputBinding, RunPolicyPin};

fn artifact_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x01; 32])
}

fn input_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x02; 32])
}

fn policy_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x03; 32])
}

fn build_run_admission(policy_pin: RunPolicyPin) -> JournalEvent {
    JournalEvent::RunAdmission {
        input_digest: RunInputBinding::Digest(input_digest()),
        run: RunId::new(1),
        seq: EventSeq::new(1),
        artifact_digest: artifact_digest(),
        policy_pin,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
    }
}

fn expected_json(policy_pin: RunPolicyPin) -> serde_json::Value {
    serde_json::json!({
        "seq": 1,
        "type": "RunAdmission",
        "run": 1,
        "artifact_digest": format!("{:?}", artifact_digest()),
        "input_digest": format!("{:?}", RunInputBinding::Digest(input_digest())),
        "granted_capabilities": format!("{:?}", CapabilitySet::empty()),
        "policy": format!("{:?}", RuntimePolicy::Strict),
        "policy_pin": format!("{:?}", policy_pin)
    })
}

#[test]
fn policy_pin_pinned_full_json_equality() {
    let event = build_run_admission(RunPolicyPin::Pinned(policy_digest()));
    let actual = event_to_json(&event);
    let expected = expected_json(RunPolicyPin::Pinned(policy_digest()));
    assert_eq!(
        actual, expected,
        "RunAdmission with Pinned policy must serialize all fields exactly"
    );
}

#[test]
fn policy_pin_unverified_full_json_equality() {
    let event = build_run_admission(RunPolicyPin::Unverified);
    let actual = event_to_json(&event);
    let expected = expected_json(RunPolicyPin::Unverified);
    assert_eq!(
        actual, expected,
        "RunAdmission with Unverified policy must serialize all fields exactly"
    );
}
