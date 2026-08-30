use crate::codec::validation::{
    validate_kind_family, validate_known_kind, validate_schema_version,
};
use crate::{
    binary::{read_u16, read_u32, read_u64, write_digest, write_u16, write_u32, write_u64},
    constants::{
        CRC_OFFSET, CURRENT_SCHEMA_VERSION, DIGEST_BYTES, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
    },
    error::JournalError,
    records::RecordKind,
    types::digests::PayloadDigest,
    types::RecordHeader,
};

pub fn encode_record_header(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    max_payload_len: u32,
) -> Result<[u8; RECORD_HEADER_BYTES], JournalError> {
    validate_kind_family(magic, kind.id())?;
    let payload_len = super::payload::payload_len_u32(payload.len(), max_payload_len)?;
    build_record_header(magic, kind, sequence, payload, payload_len)
}

pub fn decode_record_header(
    header: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<RecordHeader, JournalError> {
    let header = header
        .get(..RECORD_HEADER_BYTES)
        .ok_or(JournalError::UnexpectedEof)?;
    let decoded = decode_record_header_unchecked_len(header)?;
    if decoded.magic != expected_magic {
        return Err(JournalError::BadMagic {
            found: decoded.magic,
        });
    }
    validate_schema_version(decoded.schema_version)?;
    validate_known_kind(decoded.record_kind)?;
    validate_kind_family(decoded.magic, decoded.record_kind)?;
    if decoded.header_len != RECORD_HEADER_LEN {
        return Err(JournalError::HeaderLengthMismatch {
            found: decoded.header_len,
        });
    }
    if decoded.payload_len > max_payload_len {
        return Err(JournalError::PayloadTooLarge {
            len: decoded.payload_len,
            max: max_payload_len,
        });
    }
    if crc32c::crc32c(header_prefix_for_crc(header)?) != decoded.header_checksum {
        return Err(JournalError::HeaderChecksumMismatch);
    }
    Ok(decoded)
}

pub(crate) fn build_record_header(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    payload_len: u32,
) -> Result<[u8; RECORD_HEADER_BYTES], JournalError> {
    let mut header = [0_u8; RECORD_HEADER_BYTES];
    write_u32(&mut header, 0, magic)?;
    write_u16(&mut header, 4, CURRENT_SCHEMA_VERSION)?;
    write_u16(&mut header, 6, kind.id())?;
    write_u32(&mut header, 8, RECORD_HEADER_LEN)?;
    write_u32(&mut header, 12, payload_len)?;
    write_u64(&mut header, 16, sequence)?;
    write_digest(&mut header, blake3::hash(payload).as_bytes())?;
    let checksum = crc32c::crc32c(header_prefix_for_crc(&header)?);
    write_u32(&mut header, CRC_OFFSET, checksum)?;
    Ok(header)
}

pub(crate) fn decode_record_header_unchecked_len(
    header: &[u8],
) -> Result<RecordHeader, JournalError> {
    Ok(RecordHeader {
        magic: read_u32(header, 0)?,
        schema_version: read_u16(header, 4)?,
        record_kind: read_u16(header, 6)?,
        header_len: read_u32(header, 8)?,
        payload_len: read_u32(header, 12)?,
        sequence: read_u64(header, 16)?,
        payload_digest: PayloadDigest::from_bytes(digest_from_header(header)?),
        header_checksum: read_u32(header, CRC_OFFSET)?,
    })
}

pub(crate) fn header_prefix_for_crc(header: &[u8]) -> Result<&[u8], JournalError> {
    header.get(..CRC_OFFSET).ok_or(JournalError::UnexpectedEof)
}

pub(crate) fn digest_from_header(header: &[u8]) -> Result<[u8; DIGEST_BYTES], JournalError> {
    let digest = header
        .get(24..CRC_OFFSET)
        .ok_or(JournalError::UnexpectedEof)?;
    <[u8; DIGEST_BYTES]>::try_from(digest).map_err(|_| JournalError::UnexpectedEof)
}
