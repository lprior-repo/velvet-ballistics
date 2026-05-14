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

pub(crate) mod header;
pub(crate) mod payload;
pub(crate) mod validation;

pub use self::header::{decode_record_header, encode_record_header};
pub use self::payload::verify_digest_match;

/// Encodes a postcard payload behind the 60-byte storage envelope.
pub fn encode_record<T: Serialize>(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &T,
    max_payload_len: u32,
) -> Result<Vec<u8>, JournalError> {
    self::validation::validate_kind_family(magic, kind.id())?;
    let payload_bytes = postcard::to_allocvec(payload)?;
    let payload_len = self::payload::payload_len_u32(payload_bytes.len(), max_payload_len)?;
    self::payload::encode_record_payload(magic, kind, sequence, &payload_bytes, payload_len)
}

/// Decodes and postcard-deserializes an enveloped record.
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
