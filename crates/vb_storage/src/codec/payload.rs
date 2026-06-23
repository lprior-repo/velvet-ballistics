use crate::codec::header::{build_record_header, decode_record_header};
use crate::{
    constants::{DIGEST_BYTES, RECORD_HEADER_BYTES},
    error::JournalError,
    records::RecordKind,
    types::{RecordEnvelope, RecordHeader},
};

pub(crate) enum PayloadLenDecision {
    Accepted(u32),
    TooLarge { len: u32, max: u32 },
    LenOverflow { len: u64 },
}

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
    match classify_payload_len(len, max) {
        PayloadLenDecision::Accepted(payload_len) => Ok(payload_len),
        PayloadLenDecision::TooLarge { len, max } => {
            Err(JournalError::PayloadTooLarge { len, max })
        }
        PayloadLenDecision::LenOverflow { len } => Err(JournalError::PayloadLenOverflow { len }),
    }
}

pub(crate) fn classify_payload_len(len: usize, max: u32) -> PayloadLenDecision {
    match u32::try_from(len) {
        Ok(payload_len) if payload_len > max => PayloadLenDecision::TooLarge {
            len: payload_len,
            max,
        },
        Ok(payload_len) => PayloadLenDecision::Accepted(payload_len),
        Err(_) => PayloadLenDecision::LenOverflow {
            len: observed_len_u64(len),
        },
    }
}

fn observed_len_u64(len: usize) -> u64 {
    u64::try_from(len).map_or(u64::MAX, core::convert::identity)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn first_len_above_u32_max() -> Option<(usize, u64)> {
        let Ok(max_u32_as_usize) = usize::try_from(u32::MAX) else {
            return None;
        };
        let overflow_len = max_u32_as_usize.checked_add(1)?;
        let expected = u64::from(u32::MAX).checked_add(1)?;
        Some((overflow_len, expected))
    }

    #[test]
    fn payload_len_u32_preserves_in_range_oversize_len() {
        let result = payload_len_u32(5, 4);

        assert!(
            matches!(
                result,
                Err(JournalError::PayloadTooLarge { len: 5, max: 4 })
            ),
            "in-range overage must preserve exact payload length, got {result:?}"
        );
    }

    #[test]
    fn payload_len_u32_rejects_usize_above_u32_without_saturation() {
        let Some((overflow_len, expected_len)) = first_len_above_u32_max() else {
            return;
        };

        let result = payload_len_u32(overflow_len, 1);

        assert!(
            matches!(result, Err(JournalError::PayloadLenOverflow { len }) if len == expected_len),
            "u32 overflow must report exact overflow length, got {result:?}"
        );
    }
}
