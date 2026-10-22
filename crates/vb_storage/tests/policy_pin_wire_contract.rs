#![forbid(unsafe_code)]
//! Wire contract integration tests for RunPolicyPin (vb-h7rex).
//!
//! Verifies FULL-Eq roundtrip across encode/decode for all four admission
//! variants, stable distinct RecordKind ids, and correct envelope header
//! decoding for policy-pinned events. Also verifies malformed/truncated
//! payloads are rejected by the version-aware checkpoint decoder.

use vb_core::ids::{RunId, WorkflowDigest};
use vb_storage::admission::compute_policy_digest;
use vb_storage::codec::{
    decode_record, encode_record, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
};
use vb_storage::{
    EventSeq, JournalEvent, RecordKind, RunInputBinding, RunPolicyPin,
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

/// Legacy unbound + unverified (tag 24).
fn admission_event_unbound_unverified() -> JournalEvent {
    JournalEvent::RunAdmission {
        input_digest: RunInputBinding::LegacyUnbound,
        run: test_run(),
        seq: EventSeq::new(1),
        artifact_digest: test_artifact_digest(),
        granted_capabilities: vec![],
        policy: vb_core::workflow::RuntimePolicy::Relaxed,
        policy_pin: RunPolicyPin::Unverified,
    }
}

/// Digest-bound + unverified (tag 36).
fn admission_event_bound_unverified() -> JournalEvent {
    JournalEvent::RunAdmission {
        input_digest: RunInputBinding::Digest(test_input_digest()),
        run: test_run(),
        seq: EventSeq::new(2),
        artifact_digest: test_artifact_digest(),
        granted_capabilities: vec![],
        policy: vb_core::workflow::RuntimePolicy::Strict,
        policy_pin: RunPolicyPin::Unverified,
    }
}

/// Unbound + pinned (tag 39).
fn admission_event_unbound_pinned() -> JournalEvent {
    JournalEvent::RunAdmission {
        input_digest: RunInputBinding::LegacyUnbound,
        run: test_run(),
        seq: EventSeq::new(3),
        artifact_digest: test_artifact_digest(),
        granted_capabilities: vec![],
        policy: vb_core::workflow::RuntimePolicy::Strict,
        policy_pin: RunPolicyPin::Pinned(test_policy_digest()),
    }
}

/// Digest-bound + pinned (tag 41).
fn admission_event_bound_pinned() -> JournalEvent {
    JournalEvent::RunAdmission {
        input_digest: RunInputBinding::Digest(test_input_digest()),
        run: test_run(),
        seq: EventSeq::new(4),
        artifact_digest: test_artifact_digest(),
        granted_capabilities: vec![],
        policy: vb_core::workflow::RuntimePolicy::Journaled,
        policy_pin: RunPolicyPin::Pinned(test_policy_digest()),
    }
}

/// Encode a JournalEvent as a raw journal record and decode it back.
fn roundtrip_event(event: &JournalEvent) -> Result<JournalEvent, String> {
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {e}"))?;
    let decoded = decode_record::<JournalEvent>(
        &encoded,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("decode failed: {e}"))?;
    Ok(decoded)
}

#[test]
fn legacy_unbound_unverified_roundtrips_as_tag_24() -> Result<(), String> {
    let event = admission_event_unbound_unverified();
    assert_eq!(event.record_kind(), RecordKind::RunAdmission);
    assert_eq!(RecordKind::RunAdmission.id(), 24);

    let decoded = roundtrip_event(&event)?;
    assert_eq!(decoded, event);

    Ok(())
}

#[test]
fn digest_bound_unverified_roundtrips_as_tag_36() -> Result<(), String> {
    let event = admission_event_bound_unverified();
    assert_eq!(event.record_kind(), RecordKind::RunAdmissionBound);
    assert_eq!(RecordKind::RunAdmissionBound.id(), 36);

    let decoded = roundtrip_event(&event)?;
    assert_eq!(decoded, event);

    Ok(())
}

#[test]
fn unbound_pinned_roundtrips_as_tag_39() -> Result<(), String> {
    let event = admission_event_unbound_pinned();
    assert_eq!(event.record_kind(), RecordKind::RunAdmissionPinned);
    assert_eq!(RecordKind::RunAdmissionPinned.id(), 39);

    let decoded = roundtrip_event(&event)?;
    assert_eq!(decoded, event);

    Ok(())
}

#[test]
fn bound_pinned_roundtrips_as_tag_41() -> Result<(), String> {
    let event = admission_event_bound_pinned();
    assert_eq!(event.record_kind(), RecordKind::RunAdmissionBoundPinned);
    assert_eq!(RecordKind::RunAdmissionBoundPinned.id(), 41);

    let decoded = roundtrip_event(&event)?;
    assert_eq!(decoded, event);

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
    let events = vec![
        admission_event_unbound_unverified(),
        admission_event_bound_unverified(),
        admission_event_unbound_pinned(),
        admission_event_bound_pinned(),
    ];

    for event in events {
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode failed: {e}"))?;

        // Decode and verify the envelope header kind matches
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("decode failed: {e}"))?;

        // Envelope kind must match event's own record kind
        assert_eq!(
            decoded.record_kind(),
            event.record_kind(),
            "envelope kind mismatch for seq {}",
            event.seq()
        );
    }

    Ok(())
}

#[test]
fn malformed_record_kind_bytes_rejected() -> Result<(), String> {
    let event = admission_event_unbound_unverified();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {e}"))?;

    // Corrupt the record kind byte (byte 4 of the header)
    let mut corrupted = encoded;
    corrupted[4] ^= 0xFF;

    let result = decode_record::<JournalEvent>(
        &corrupted,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        result.is_err(),
        "malformed record kind should be rejected"
    );

    Ok(())
}

#[test]
fn truncated_payload_rejected() -> Result<(), String> {
    let event = admission_event_unbound_unverified();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {e}"))?;

    // Truncate the payload portion (after the 60-byte header)
    let truncated = &encoded[..65];

    let result = decode_record::<JournalEvent>(
        truncated,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        result.is_err(),
        "truncated payload should be rejected"
    );

    Ok(())
}

/// Positive-control fullEq before changing pin: encode/decode the pinned event,
/// verify exact roundtrip, then decode the envelope header to confirm the kind.
#[test]
fn positive_control_roundtrip_with_header_check() -> Result<(), String> {
    let event = admission_event_unbound_pinned();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {e}"))?;

    // Full decode
    let decoded = decode_record::<JournalEvent>(
        &encoded,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("decode failed: {e}"))?;
    assert_eq!(decoded, event, "positive-control FULL-Eq must pass");

    // Decode envelope header only to verify kind byte
    use vb_storage::codec::decode_envelope_only;
    let (envelope, _remainder) = decode_envelope_only(&encoded)
        .map_err(|e| format!("envelope decode failed: {e}"))?;
    assert_eq!(
        envelope.kind,
        RecordKind::RunAdmissionPinned,
        "envelope header kind must be RunAdmissionPinned (39)"
    );
    assert_eq!(envelope.kind.id(), 39, "envelope header kind id must be 39");

    Ok(())
}

/// Tampered payload digest is rejected by the codec.
#[test]
fn tampered_payload_digest_rejected() -> Result<(), String> {
    let event = admission_event_unbound_pinned();
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {e}"))?;

    // Tamper with the payload digest bytes (bytes 36-60 of header)
    let mut tampered = encoded;
    tampered[36] ^= 0xFF;

    let result = decode_record::<JournalEvent>(
        &tampered,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        result.is_err(),
        "tampered payload digest should be rejected"
    );

    Ok(())
}

/// Pre-v2 checkpoint payload is rejected by version-aware decoder.
#[test]
fn pre_v2_checkpoint_rejected() -> Result<(), String> {
    let version1_payload: Vec<u8> = vec![1, 0, 0, 0, 0];
    let result = vb_runtime::run_checkpoint::decode_checkpoint_payload(&version1_payload);
    match result {
        Err(vb_runtime::run_checkpoint::RunCheckpointV1Error::UnsupportedVersion) => {}
        Err(other) => {
            return Err(format!("expected UnsupportedVersion, got: {other}"));
        }
        Ok(decoded) => {
            return Err(format!(
                "version 1 should be rejected, not decoded: {decoded:?}"
            ));
        }
    }
    Ok(())
}