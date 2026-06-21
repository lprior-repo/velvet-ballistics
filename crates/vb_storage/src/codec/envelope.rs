#![forbid(unsafe_code)]
//! Envelope-only record inspection.
//!
//! `decode_envelope_only` decodes the 60-byte record header and returns
//! the envelope metadata plus the raw payload slice WITHOUT calling
//! postcard deserialization. This is useful for inspection (doctor)
//! and filtering operations that only need header data.

use crate::{
    codec::{header::decode_record_header, payload::verify_digest_match},
    constants::RECORD_HEADER_BYTES,
    error::JournalError,
    types::{RecordEnvelope, RecordHeader},
};

/// Decodes a record header and returns envelope metadata + raw payload.
///
/// Unlike `decode_record`, this does NOT call postcard deserialization
/// on the payload. It returns the raw payload bytes as a subslice of
/// the input, suitable for inspection-only workflows. The BLAKE3 payload
/// digest is still verified against the header before the payload is
/// returned, so callers can rely on the bytes being tamper-evident.
///
/// # Errors
///
/// Returns `JournalError` if the header is malformed, the magic is wrong,
/// the schema is unsupported, the kind is unknown, the length is wrong,
/// the payload is too large, trailing bytes remain, the CRC checksum fails,
/// or the payload BLAKE3 digest does not match the header's claim.
pub fn decode_envelope_only(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, &[u8]), JournalError> {
    // Validate and decode the header.
    let header: RecordHeader = decode_record_header(bytes, expected_magic, max_payload_len)?;

    // Extract the raw payload: bytes[60..60+payload_len]
    let payload_start = RECORD_HEADER_BYTES;
    let payload_len: usize = header
        .payload_len
        .try_into()
        .map_err(|_| JournalError::UnexpectedEof)?;
    let payload_end = payload_start
        .checked_add(payload_len)
        .ok_or(JournalError::UnexpectedEof)?;
    let raw_payload = bytes
        .get(payload_start..payload_end)
        .ok_or(JournalError::UnexpectedEof)?;
    if payload_end != bytes.len() {
        return Err(JournalError::UnexpectedTrailingBytes {
            declared_end: payload_end,
            actual_len: bytes.len(),
        });
    }
    // Verify the BLAKE3 payload digest even though we are skipping
    // postcard deserialization. Skipping the hash would let a bit-flip
    // on disk (the failure mode BLAKE3 is meant to detect) reach doctor
    // and other inspection consumers as authoritative.
    verify_digest_match(raw_payload, header.payload_digest)?;

    let envelope = RecordEnvelope {
        magic: header.magic,
        schema_version: header.schema_version,
        record_kind: header.record_kind,
        sequence: header.sequence,
    };

    Ok((envelope, raw_payload))
}
