#![forbid(unsafe_code)]
//! Record encoding and decoding functions.

use crate::{
    error::JournalError,
    events::JournalEvent,
    records::RecordKind,
    types::{EventSeq, RecordEnvelope},
};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub(crate) mod envelope;
pub(crate) mod header;
pub(crate) mod payload;
pub(crate) mod validation;

mod kind_parity;

pub use self::kind_parity::EnforceKindParity;

#[cfg(fuzzing)]
pub mod fuzz_validation {
    //! Public fuzz-only accessors for codec validation invariants.

    use crate::JournalError;

    pub const fn is_known_record_kind(kind: u16) -> bool {
        super::validation::is_known_record_kind(kind)
    }

    pub fn validate_known_kind(kind: u16) -> Result<(), JournalError> {
        super::validation::validate_known_kind(kind)
    }

    pub fn validate_kind_family(magic: u32, kind: u16) -> Result<(), JournalError> {
        super::validation::validate_kind_family(magic, kind)
    }
}

pub use self::header::{decode_record_header, encode_record_header};
pub use self::payload::verify_digest_match;

/// Returns whether a raw wire record-kind value is recognized by storage.
pub const fn is_known_record_kind(kind: u16) -> bool {
    self::validation::is_known_record_kind(kind)
}

/// Validates a raw wire record-kind value before semantic family checks.
pub fn validate_known_record_kind(kind: u16) -> Result<(), JournalError> {
    self::validation::validate_known_kind(kind)
}

/// Validates that a raw wire record-kind value belongs to the magic family.
pub fn validate_record_kind_family(magic: u32, kind: u16) -> Result<(), JournalError> {
    self::validation::validate_kind_family(magic, kind)
}

/// Encodes a postcard payload behind the 60-byte storage envelope.
pub fn encode_record<T: Serialize>(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &T,
    max_payload_len: u32,
) -> Result<Vec<u8>, JournalError> {
    validate_record_kind_family(magic, kind.id())?;
    let payload_bytes = postcard::to_allocvec(payload).map_err(JournalError::Encode)?;
    let payload_len = self::payload::payload_len_u32(payload_bytes.len(), max_payload_len)?;
    self::payload::encode_record_payload(magic, kind, sequence, &payload_bytes, payload_len)
}

/// Decodes and postcard-deserializes an enveloped record.
///
/// `T` must be one of the storage record types that implement
/// [`EnforceKindParity`]. For [`JournalEvent`], parity is enforced automatically:
/// the envelope record kind must match the decoded payload variant, and the
/// event must satisfy `JournalEvent::is_valid()`. For non-journal record types
/// (workflow source, compiled IR, blob, run snapshot, run header) the parity
/// hook is a no-op because those types do not carry a record-kind discriminant
/// in the payload.
pub fn decode_record<T>(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, T), JournalError>
where
    T: DeserializeOwned + EnforceKindParity,
{
    let (envelope, payload) =
        self::payload::decode_record_payload(bytes, expected_magic, max_payload_len)?;
    let value = postcard::from_bytes(payload).map_err(JournalError::PostcardDecodeFailed)?;
    T::enforce_kind_parity(&envelope, &value)?;
    Ok((envelope, value))
}

/// Validates that the envelope kind exactly matches the decoded journal payload variant.
pub fn validate_journal_event_record_kind(
    envelope: &RecordEnvelope,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    let payload_kind = event.record_kind().id();
    if envelope.record_kind == payload_kind {
        Ok(())
    } else {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: envelope.record_kind,
            payload_kind,
        })
    }
}

/// Decodes a `JournalEvent` and validates its envelope/payload parity and semantic constraints.
///
/// This is the correct decode function for untrusted input streams. The kind/payload
/// parity and `JournalEvent::is_valid()` checks are mandatory: callers cannot opt
/// out without bypassing [`decode_record`] entirely. Production parse/replay paths
/// must use this function for journal events.
///
/// # Errors
///
/// Returns an error if the bytes do not form a valid journal event record, including:
/// - [`JournalError::RecordKindPayloadMismatch`] if the envelope kind and payload variant disagree
/// - [`JournalError::InvalidEvent`] if the payload is structurally encoded but semantically invalid
///   (`run_id == 0`, `seq == u64::MAX`, or `attempt == 0`)
pub fn decode_journal_event(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, JournalEvent), JournalError> {
    let (envelope, event) = decode_record::<JournalEvent>(bytes, expected_magic, max_payload_len)?;
    // `decode_record::<JournalEvent>` already enforces parity and is_valid via the
    // `EnforceKindParity` impl. These explicit calls remain as a defense-in-depth
    // self-check and to make the invariants obvious to readers.
    validate_journal_event_record_kind(&envelope, &event)?;
    if !event.is_valid() {
        return Err(JournalError::InvalidEvent);
    }
    // round10-a / Issue 5: the envelope's sequence field must equal the
    // payload's seq. Any divergence indicates the wire record was forged or
    // corrupted after the Fjall keyspace accepted the write; fail closed
    // rather than silently replaying under the wrong identity.
    if envelope.sequence != event.seq().get() {
        return Err(JournalError::ReplayEnvelopeSequenceMismatch {
            run: event.run_id(),
            envelope_seq: envelope.sequence,
            payload_seq: event.seq().get(),
        });
    }
    Ok((envelope, event))
}

pub(crate) fn next_seq(seq: EventSeq) -> Result<EventSeq, JournalError> {
    seq.get()
        .checked_add(1)
        .map(EventSeq::new)
        .ok_or(JournalError::SequenceOverflow)
}

pub(crate) fn validate_replayed_event(
    run: vb_core::RunId,
    expected: EventSeq,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    if event.run_id() != run {
        return Err(JournalError::WrongRun {
            expected: run,
            actual: event.run_id(),
        });
    }
    if event.seq() != expected {
        return Err(JournalError::SequenceGap {
            expected,
            actual: event.seq(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;

// vb-b8i8f: flux_validation requires flux_rs crate (not in workspace).
// Keep as artifact; re-enable when flux_rs dependency is added.
// #[cfg(feature = "flux")]
// pub mod flux_validation;
