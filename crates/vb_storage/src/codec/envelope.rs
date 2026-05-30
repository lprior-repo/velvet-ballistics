#![forbid(unsafe_code)]
//! Envelope-only record inspection.
//!
//! `decode_envelope_only` decodes the 60-byte record header and returns
//! the envelope metadata plus the raw payload slice WITHOUT calling
//! postcard deserialization. This is useful for inspection (doctor)
//! and filtering operations that only need header data.

use crate::{
    constants::RECORD_HEADER_BYTES,
    error::JournalError,
    types::{RecordEnvelope, RecordHeader},
    codec::header::decode_record_header,
};

/// Decodes a record header and returns envelope metadata + raw payload.
///
/// Unlike `decode_record`, this does NOT call postcard deserialization
/// on the payload. It returns the raw payload bytes as a subslice of
/// the input, suitable for inspection-only workflows.
///
/// # Errors
///
/// Returns `JournalError` if the header is malformed, the magic is wrong,
/// the schema is unsupported, the kind is unknown, the length is wrong,
/// the payload is too large, or the CRC checksum fails.
pub fn decode_envelope_only(
    bytes: &[u8],
) -> Result<(RecordEnvelope, &[u8]), JournalError> {
    // Validate and decode the header.
    // We accept MAGIC_JOURNAL_EVENT as the default expected magic.
    // For mixed-magic input, the caller should pre-filter or use
    // a lower-level decode path.
    let header: RecordHeader = decode_record_header(
        bytes,
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    // Extract the raw payload: bytes[60..60+payload_len]
    let payload_start = RECORD_HEADER_BYTES;
    let payload_len = header.payload_len as usize;
    let payload_end = payload_start
        .checked_add(payload_len)
        .ok_or(JournalError::UnexpectedEof)?;
    let raw_payload = bytes
        .get(payload_start..payload_end)
        .ok_or(JournalError::UnexpectedEof)?;

    let envelope = RecordEnvelope {
        magic: header.magic,
        schema_version: header.schema_version,
        record_kind: header.record_kind,
        sequence: header.sequence,
    };

    Ok((envelope, raw_payload))
}
