#![forbid(unsafe_code)]
//! Public record encode/decode entry points.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    records::RecordKind,
    types::RecordEnvelope,
};

use super::{ValidatedJournalRecord, validate_record_kind_family};

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
    let payload_len = super::payload::payload_len_u32(payload_bytes.len(), max_payload_len)?;
    super::payload::encode_record_payload(magic, kind, sequence, &payload_bytes, payload_len)
}

/// Encodes a normal journal-event write using the event's canonical record kind.
pub fn encode_journal_event_record(event: &JournalEvent) -> Result<Vec<u8>, JournalError> {
    encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
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
        super::payload::decode_record_payload(bytes, expected_magic, max_payload_len)?;
    let value = postcard::from_bytes(payload).map_err(|_| JournalError::PostcardDecodeFailed)?;
    Ok((envelope, value))
}

/// Decodes a `JournalEvent` and validates its semantic constraints.
///
/// This is the correct decode function for untrusted input streams.
///
/// # Errors
///
/// Returns [`JournalError`] for every [`decode_record`] failure and returns
/// `JournalError::InvalidEvent` when the decoded event is semantically invalid.
pub fn decode_journal_event(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, JournalEvent), JournalError> {
    decode_validated_journal_record(bytes, expected_magic, max_payload_len)
        .map(|record| record.into_parts())
}

/// Decodes a `JournalEvent` into a parity-validated typestate wrapper.
pub fn decode_validated_journal_record(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<ValidatedJournalRecord, JournalError> {
    let (envelope, event) = decode_record::<JournalEvent>(bytes, expected_magic, max_payload_len)?;
    ValidatedJournalRecord::try_new(envelope, event)
}
