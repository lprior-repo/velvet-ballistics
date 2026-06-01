use crate::codec::header::{build_record_header, decode_record_header};
use crate::{
    constants::{DIGEST_BYTES, RECORD_HEADER_BYTES},
    error::JournalError,
    records::RecordKind,
    types::{RecordEnvelope, RecordHeader},
};

pub fn verify_digest_match(
    payload: &[u8],
    expected_digest: [u8; DIGEST_BYTES],
) -> Result<(), JournalError> {
    if blake3::hash(payload).as_bytes() == &expected_digest {
        Ok(())
    } else {
        Err(JournalError::PayloadDigestMismatch)
    }
}

pub(crate) fn payload_len_u32(len: usize, max: u32) -> Result<u32, JournalError> {
    let payload_len = u32::try_from(len).map_err(|_| JournalError::PayloadTooLarge {
        len: 4_294_967_295,
        max,
    })?;
    if payload_len > max {
        return Err(JournalError::PayloadTooLarge {
            len: payload_len,
            max,
        });
    }
    Ok(payload_len)
}

pub(crate) fn encode_record_payload(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    payload_len: u32,
) -> Result<Vec<u8>, JournalError> {
    let capacity =
        RECORD_HEADER_BYTES
            .checked_add(payload.len())
            .ok_or(JournalError::PayloadTooLarge {
                len: payload_len,
                max: 4_294_967_295,
            })?;
    let header = build_record_header(magic, kind, sequence, payload, payload_len)?;

    let mut encoded = Vec::with_capacity(capacity);
    encoded.extend_from_slice(&header);
    encoded.extend_from_slice(payload);
    Ok(encoded)
}

fn envelope_from_header(header: &RecordHeader) -> RecordEnvelope {
    RecordEnvelope {
        magic: header.magic,
        schema_version: header.schema_version,
        record_kind: header.record_kind,
        sequence: header.sequence,
    }
}

pub(crate) fn decode_record_payload(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, &[u8]), JournalError> {
    let header = decode_record_header(bytes, expected_magic, max_payload_len)?;
    let payload_start =
        usize::try_from(header.header_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_len_usize =
        usize::try_from(header.payload_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_end = payload_start
        .checked_add(payload_len_usize)
        .ok_or(JournalError::UnexpectedEof)?;
    let payload = bytes
        .get(payload_start..payload_end)
        .ok_or(JournalError::UnexpectedEof)?;
    verify_digest_match(payload, header.payload_digest)?;
    reject_trailing_bytes(payload_end, bytes.len())?;
    Ok((envelope_from_header(&header), payload))
}

pub(crate) fn reject_trailing_bytes(
    declared_end: usize,
    actual_len: usize,
) -> Result<(), JournalError> {
    match trailing_byte_bounds(declared_end, actual_len) {
        None => Ok(()),
        Some((declared_end, actual_len)) => Err(JournalError::UnexpectedTrailingBytes {
            declared_end,
            actual_len,
        }),
    }
}

pub(crate) const fn trailing_byte_bounds(
    declared_end: usize,
    actual_len: usize,
) -> Option<(usize, usize)> {
    if actual_len > declared_end {
        Some((declared_end, actual_len))
    } else {
        None
    }
}
