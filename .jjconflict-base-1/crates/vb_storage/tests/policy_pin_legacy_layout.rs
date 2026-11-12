#![forbid(unsafe_code)]
//! Legacy layout compatibility tests for policy pin variants (vb-h7rex).
//!
//! Verifies that tag 24 (RunAdmission) and tag 36 (RunAdmissionBound)
//! preserve the exact historical postcard byte layout, ensuring
//! simultaneous encoder/decoder drift detection and cross-version
//! journal readability.

use vb_core::{Capability, RunId, WorkflowDigest, ids::ActionId};
use vb_storage::{
    EventSeq, RecordKind, RunPolicyPin,
    codec::{decode_journal_event, encode_record},
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    events::JournalEvent,
    inputs::RunInputBinding,
};

fn fixture_capabilities() -> vb_core::CapabilitySet {
    let cap = Capability::new("network".into(), ActionId::new(42));
    vb_core::CapabilitySet::from_grants(Box::new([cap]))
}

fn fixture_run() -> RunId {
    RunId::new(777_001)
}

fn fixture_input_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x54; 32])
}

fn fixture_artifact_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0x55; 32])
}

fn expected_unbound_event() -> JournalEvent {
    JournalEvent::RunAdmission {
        run: fixture_run(),
        seq: EventSeq::new(1),
        artifact_digest: fixture_artifact_digest(),
        input_digest: RunInputBinding::LegacyUnbound,
        policy_pin: RunPolicyPin::Unverified,
        granted_capabilities: fixture_capabilities(),
        policy: vb_core::RuntimePolicy::Relaxed,
    }
}

fn expected_bound_event() -> JournalEvent {
    JournalEvent::RunAdmission {
        run: fixture_run(),
        seq: EventSeq::new(1),
        artifact_digest: fixture_artifact_digest(),
        input_digest: RunInputBinding::Digest(fixture_input_digest()),
        policy_pin: RunPolicyPin::Unverified,
        granted_capabilities: fixture_capabilities(),
        policy: vb_core::RuntimePolicy::Strict,
    }
}

fn encode_envelope<T: serde::Serialize>(
    kind: RecordKind,
    payload: &T,
) -> Result<Vec<u8>, vb_storage::JournalError> {
    encode_record(
        MAGIC_JOURNAL_EVENT,
        kind,
        1,
        payload,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
}

fn check_historic_layout<T: serde::Serialize>(
    kind: RecordKind,
    legacy_payload: &T,
    current_event: &JournalEvent,
) -> Result<(), vb_storage::JournalError> {
    let legacy_bytes = encode_envelope(kind, legacy_payload)?;
    let current_bytes = encode_envelope(kind, current_event)?;
    assert_eq!(legacy_bytes, current_bytes);
    let (envelope, decoded) = decode_journal_event(
        &legacy_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.record_kind, kind.id());
    assert_eq!(decoded, *current_event);
    Ok(())
}

#[test]
fn tag_24_unbound_preserves_historical_layout() -> Result<(), vb_storage::JournalError> {
    let legacy_payload = (
        24_u8,
        &fixture_run(),
        &EventSeq::new(1),
        &fixture_artifact_digest(),
        &fixture_capabilities(),
        &vb_core::RuntimePolicy::Relaxed,
    );
    check_historic_layout(
        RecordKind::RunAdmission,
        &legacy_payload,
        &expected_unbound_event(),
    )
}

#[test]
fn tag_36_bound_preserves_historical_layout() -> Result<(), vb_storage::JournalError> {
    let legacy_payload = (
        36_u8,
        &fixture_run(),
        &EventSeq::new(1),
        &fixture_artifact_digest(),
        &fixture_input_digest(),
        &fixture_capabilities(),
        &vb_core::RuntimePolicy::Strict,
    );
    check_historic_layout(
        RecordKind::RunAdmissionBound,
        &legacy_payload,
        &expected_bound_event(),
    )
}
