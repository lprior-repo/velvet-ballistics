#![forbid(unsafe_code)]
//! Contextual tag-46 decoding after an allocation-free shape preflight.
//!
//! Accepted lengths/counts are bounded before derived owned deserialization.
//! The owned Serde allocations remain infallible: this is not typed OOM handling.

#[path = "wire_deserialize_grant_preflight.rs"]
mod preflight;

use super::super::super::JournalEvent;
use crate::{EventSeq, JournalError, RecordKind, types::RecordEnvelope};
use vb_core::{RunId, capability_grant::CapabilityGrant};

pub(super) fn decode<'payload>(
    envelope: &RecordEnvelope,
    payload: &'payload [u8],
) -> Result<(JournalEvent, &'payload [u8]), JournalError> {
    decode_with_materializer(envelope, payload, materialize_owned)
}

fn decode_with_materializer<'payload, F>(
    envelope: &RecordEnvelope,
    payload: &'payload [u8],
    materialize: F,
) -> Result<(JournalEvent, &'payload [u8]), JournalError>
where
    F: FnOnce(&'payload [u8]) -> Result<(JournalEvent, &'payload [u8]), JournalError>,
{
    if envelope.record_kind != RecordKind::CapabilityGrant.id() {
        return Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: envelope.record_kind,
            payload_kind: RecordKind::CapabilityGrant.id(),
        });
    }
    preflight::validate(payload)?;
    materialize(payload)
}

fn materialize_owned(payload: &[u8]) -> Result<(JournalEvent, &[u8]), JournalError> {
    type GrantPayload = (u16, RunId, EventSeq, CapabilityGrant);
    let ((tag, run, seq, grant), remaining): (GrantPayload, &[u8]) =
        postcard::take_from_bytes(payload).map_err(JournalError::PostcardDecodeFailed)?;
    if tag != RecordKind::CapabilityGrant.id()
        || run.get() == 0
        || seq.get() == u64::MAX
        || grant.run != run
        || !grant.is_valid()
        || !remaining.is_empty()
    {
        return Err(JournalError::InvalidEvent);
    }
    Ok((
        JournalEvent::CapabilityGrantEvent { run, seq, grant },
        remaining,
    ))
}

#[cfg(test)]
#[path = "wire_deserialize_grant_tests.rs"]
mod tests;
