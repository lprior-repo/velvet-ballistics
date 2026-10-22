#![forbid(unsafe_code)]
//! Wire contract integration tests for RunPolicyPin (vb-h7rex).
//!
//! Verifies that typed JournalEvent::RunAdmission roundtrips FULL-Eq across
//! encode/decode for all four admission variants, and that the wire
//! RecordKind ids are stable and distinct.

use vb_core::{RunId, WorkflowDigest};
use vb_storage::{
    RecordKind, RunPolicyPin,
    codec::{decode_journal_event, encode_record},
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    events::JournalEvent,
    inputs::RunInputBinding,
};

/// Distinct run identity for wire contract tests.
fn test_run() -> RunId {
    RunId::new(777_001)
}

/// Distinct policy digest for wire contract tests.
fn test_policy_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x53; 32])
}

/// Distinct input digest for wire contract tests.
fn test_input_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x54; 32])
}

/// Distinct artifact digest for wire contract tests.
fn test_artifact_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x55; 32])
}

/// Concrete non-empty capability set for fullEq wire roundtrip proof.
fn test_granted_capabilities() -> vb_core::CapabilitySet {
    let cap = vb_core::Capability::new("network".into(), vb_core::ids::ActionId::new(42));
    vb_core::CapabilitySet::from_grants(Box::new([cap]))
}

/// Legacy unbound + unverified (tag 24).
fn admission_event_unbound_unverified() -> JournalEvent {
    JournalEvent::RunAdmission {
        run: test_run(),
        seq: vb_storage::EventSeq::new(1),
        artifact_digest: test_artifact_digest(),
        input_digest: RunInputBinding::LegacyUnbound,
        policy_pin: RunPolicyPin::Unverified,
        granted_capabilities: test_granted_capabilities(),
        policy: vb_core::RuntimePolicy::Relaxed,
    }
}

/// Digest-bound + unverified (tag 36).
fn admission_event_bound_unverified() -> JournalEvent {
    JournalEvent::RunAdmission {
        run: test_run(),
        seq: vb_storage::EventSeq::new(1),
        artifact_digest: test_artifact_digest(),
        input_digest: RunInputBinding::Digest(test_input_digest()),
        policy_pin: RunPolicyPin::Unverified,
        granted_capabilities: test_granted_capabilities(),
        policy: vb_core::RuntimePolicy::Journaled,
    }
}

/// Unbound + pinned (tag 39).
fn admission_event_unbound_pinned() -> JournalEvent {
    JournalEvent::RunAdmission {
        run: test_run(),
        seq: vb_storage::EventSeq::new(1),
        artifact_digest: test_artifact_digest(),
        input_digest: RunInputBinding::LegacyUnbound,
        policy_pin: RunPolicyPin::Pinned(test_policy_digest()),
        granted_capabilities: test_granted_capabilities(),
        policy: vb_core::RuntimePolicy::Strict,
    }
}

/// Digest-bound + pinned (tag 41).
fn admission_event_bound_pinned() -> JournalEvent {
    JournalEvent::RunAdmission {
        run: test_run(),
        seq: vb_storage::EventSeq::new(1),
        artifact_digest: test_artifact_digest(),
        input_digest: RunInputBinding::Digest(test_input_digest()),
        policy_pin: RunPolicyPin::Pinned(test_policy_digest()),
        granted_capabilities: test_granted_capabilities(),
        policy: vb_core::RuntimePolicy::Strict,
    }
}

/// Encode a JournalEvent and decode it back, asserting FULL-Eq roundtrip.
fn roundtrip_event(event: &JournalEvent) -> Result<JournalEvent, String> {
    let payload = postcard::to_allocvec(event).map_err(|e| format!("encode failed: {e}"))?;
    let expected_tag: u8 =
        u8::try_from(event.record_kind().id()).map_err(|_| "record kind id out of u8 range")?;
    assert_eq!(payload.first(), Some(&expected_tag));
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        1,
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("record encode failed: {e}"))?;
    let (envelope, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
            .map_err(|e| format!("decode failed: {e}"))?;
    let expected_kind: u16 = event.record_kind().id();
    if envelope.record_kind != expected_kind {
        return Err(format!(
            "envelope record_kind mismatch: envelope={}, expected={}",
            envelope.record_kind, expected_kind
        ));
    }
    if decoded != *event {
        return Err(format!(
            "roundtrip FULL-Eq failed: original={:?}, decoded={:?}",
            event, decoded
        ));
    }
    Ok(decoded)
}

#[test]
fn legacy_unbound_unverified_roundtrips_as_tag_24() -> Result<(), String> {
    let event = admission_event_unbound_unverified();
    assert_eq!(event.record_kind(), RecordKind::RunAdmission);
    assert_eq!(event.record_kind().id(), 24);
    let decoded = roundtrip_event(&event)?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn digest_bound_unverified_roundtrips_as_tag_36() -> Result<(), String> {
    let event = admission_event_bound_unverified();
    assert_eq!(event.record_kind(), RecordKind::RunAdmissionBound);
    assert_eq!(event.record_kind().id(), 36);
    let decoded = roundtrip_event(&event)?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn unbound_pinned_roundtrips_as_tag_39() -> Result<(), String> {
    let event = admission_event_unbound_pinned();
    assert_eq!(event.record_kind(), RecordKind::RunAdmissionPinned);
    assert_eq!(event.record_kind().id(), 39);
    let decoded = roundtrip_event(&event)?;
    assert_eq!(decoded, event);
    // Verify decoded event retains the pinned policy identity.
    match &decoded {
        JournalEvent::RunAdmission {
            policy_pin: RunPolicyPin::Pinned(pin),
            ..
        } if *pin == test_policy_digest() => {}
        other => {
            return Err(format!("decoded event lost pinned policy: {:?}", other));
        }
    }
    Ok(())
}

#[test]
fn bound_pinned_roundtrips_as_tag_41() -> Result<(), String> {
    let event = admission_event_bound_pinned();
    assert_eq!(event.record_kind(), RecordKind::RunAdmissionBoundPinned);
    assert_eq!(event.record_kind().id(), 41);
    let decoded = roundtrip_event(&event)?;
    assert_eq!(decoded, event);
    // Verify decoded event retains both input and policy identity.
    match &decoded {
        JournalEvent::RunAdmission {
            input_digest: RunInputBinding::Digest(input),
            policy_pin: RunPolicyPin::Pinned(pin),
            ..
        } if *input == test_input_digest() && *pin == test_policy_digest() => {}
        other => {
            return Err(format!(
                "decoded event lost input or policy identity: {:?}",
                other
            ));
        }
    }
    Ok(())
}

#[test]
fn all_admission_record_kinds_have_stable_ids() {
    assert_eq!(RecordKind::RunAdmission.id(), 24);
    assert_eq!(RecordKind::RunAdmissionBound.id(), 36);
    assert_eq!(RecordKind::RunAdmissionPinned.id(), 39);
    assert_eq!(RecordKind::RunAdmissionBoundPinned.id(), 41);
}

#[test]
fn wire_envelope_kind_matches_event_record_kind() -> Result<(), String> {
    let event = admission_event_bound_pinned();
    let payload = postcard::to_allocvec(&event).map_err(|e| format!("encode failed: {e}"))?;
    let expected_tag: u8 =
        u8::try_from(event.record_kind().id()).map_err(|_| "record kind id out of u8 range")?;
    assert_eq!(payload.first(), Some(&expected_tag));
    Ok(())
}
