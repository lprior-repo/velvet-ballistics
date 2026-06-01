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

#[cfg(any(fuzzing, feature = "fuzz-access"))]
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

    pub fn reject_trailing_bytes(
        declared_end: usize,
        actual_len: usize,
    ) -> Result<(), JournalError> {
        super::payload::reject_trailing_bytes(declared_end, actual_len)
    }
}

pub use self::envelope::decode_envelope_only;
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
    let payload_bytes = postcard::to_allocvec(payload)?;
    let payload_len = self::payload::payload_len_u32(payload_bytes.len(), max_payload_len)?;
    self::payload::encode_record_payload(magic, kind, sequence, &payload_bytes, payload_len)
}

/// Decodes and postcard-deserializes an enveloped record.
///
/// # Errors
///
/// Returns [`JournalError`] if the envelope header is invalid, the declared
/// payload is missing, payload digest verification fails, trailing bytes remain
/// after the declared payload (`JournalError::UnexpectedTrailingBytes`), or the
/// postcard payload cannot be decoded as `T`.
pub fn decode_record<T: DeserializeOwned>(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, T), JournalError> {
    let (envelope, payload) =
        self::payload::decode_record_payload(bytes, expected_magic, max_payload_len)?;
    let value = postcard::from_bytes(payload).map_err(|_| JournalError::PostcardDecodeFailed)?;
    Ok((envelope, value))
}

/// Decodes a `JournalEvent` and validates its semantic constraints.
///
/// Unlike the generic [`decode_record`], this function additionally verifies that the
/// decoded event passes `JournalEvent::is_valid()` — catching run_id=0, seq=u64::MAX,
/// and attempt=0 that can arise from adversarial byte streams even when postcard
/// deserialization succeeds.
///
/// This is the correct decode function for untrusted input streams.
///
/// # Errors
///
/// Returns [`JournalError`] for every [`decode_record`] failure, including
/// `JournalError::UnexpectedTrailingBytes` when bytes remain after the declared
/// payload, and returns `JournalError::InvalidEvent` when the decoded event is
/// semantically invalid.
pub fn decode_journal_event(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, JournalEvent), JournalError> {
    let (envelope, event) = decode_record::<JournalEvent>(bytes, expected_magic, max_payload_len)?;
    if !event.is_valid() {
        return Err(JournalError::InvalidEvent);
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

#[cfg(test)]
mod trailing_bytes_proptests;

// vb-b8i8f: flux_validation requires flux_rs crate (not in workspace).
// Keep as artifact; re-enable when flux_rs dependency is added.
// #[cfg(feature = "flux")]
// pub mod flux_validation;
