#![forbid(unsafe_code)]
//! Envelope-only record inspection.
//!
//! `decode_envelope_only` decodes the 60-byte record header, verifies the
//! BLAKE3 payload digest against the actual payload bytes, and returns the
//! envelope metadata plus the raw payload slice WITHOUT calling postcard
//! deserialization. This is useful for inspection (doctor) and filtering
//! operations that only need header data.

use crate::{
    codec::header::decode_record_header,
    codec::payload::verify_digest_match,
    constants::RECORD_HEADER_BYTES,
    error::JournalError,
    types::{RecordEnvelope, RecordHeader},
};

/// Decodes a record header, verifies the BLAKE3 payload digest, and returns
/// envelope metadata + raw payload.
///
/// Unlike `decode_record`, this does NOT call postcard deserialization
/// on the payload. It returns the raw payload bytes as a subslice of
/// the input, suitable for inspection-only workflows.
///
/// # Preconditions
///
/// `bytes` must contain at least `RECORD_HEADER_BYTES + header.payload_len`
/// bytes starting at offset 0. The 60-byte header at offset 0 must satisfy
/// `decode_record_header` (valid magic, schema, kind, family, length, and
/// CRC32C header checksum).
///
/// # Postconditions
///
/// On `Ok`, the returned `&[u8]` slice exactly matches the bytes whose
/// BLAKE3 hash equals the digest stored in the envelope header. The
/// returned slice borrows from `bytes`.
///
/// # Errors
///
/// Returns `JournalError` if the header is malformed, the magic is wrong,
/// the schema is unsupported, the kind is unknown, the length is wrong,
/// the payload is too large, the CRC checksum fails, or the BLAKE3
/// payload digest does not match the payload bytes.
#[allow(
    dead_code,
    reason = "inspection-only entry point retained for doctor/filtering workflows"
)]
pub(crate) fn decode_envelope_only(bytes: &[u8]) -> Result<(RecordEnvelope, &[u8]), JournalError> {
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

    // Fail closed: envelope-only decode must still detect payload tampering.
    verify_digest_match(raw_payload, header.payload_digest)?;

    let envelope = RecordEnvelope {
        magic: header.magic,
        schema_version: header.schema_version,
        record_kind: header.record_kind,
        sequence: header.sequence,
    };

    Ok((envelope, raw_payload))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::as_conversions
)]
mod tests {
    //! Regression tests for SC-011: envelope-only decode must verify
    //! the BLAKE3 payload digest stored in the record header.

    use super::decode_envelope_only;
    use crate::{
        codec::encode_record_header,
        constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
        error::JournalError,
        records::RecordKind,
    };

    fn build_valid_record(payload: &[u8]) -> Vec<u8> {
        let header = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            1,
            payload,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encode_record_header must succeed for valid inputs");
        let mut bytes = Vec::with_capacity(header.len() + payload.len());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn decode_envelope_only_accepts_valid_record() {
        let payload = b"valid payload bytes";
        let record = build_valid_record(payload);

        let (envelope, returned_payload) =
            decode_envelope_only(&record).expect("valid record must decode");

        assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(envelope.sequence, 1);
        assert_eq!(returned_payload, payload);
    }

    #[test]
    fn decode_envelope_only_rejects_corrupted_payload() {
        let original_payload = b"original payload bytes";
        let mut record = build_valid_record(original_payload);

        // Tamper with the first payload byte so it no longer matches the
        // BLAKE3 digest stored in the header. The header itself remains
        // valid (digest and CRC still describe the original payload).
        let tampered_offset = record.len() - original_payload.len();
        record[tampered_offset] ^= 0xFF;

        let result = decode_envelope_only(&record);

        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "corrupted payload must yield PayloadDigestMismatch, got {:?}",
            result
        );
    }

    #[test]
    fn decode_envelope_only_rejects_truncated_payload() {
        let payload = b"payload to truncate";
        let mut record = build_valid_record(payload);

        // Drop the last payload byte so the trailing slice no longer
        // covers the full digest input.
        let truncated_len = record.len() - 1;
        record.truncate(truncated_len);

        let result = decode_envelope_only(&record);

        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "truncated payload must yield UnexpectedEof, got {:?}",
            result
        );
    }
}
