//! Postcard binary envelope encoding/decoding.

extern crate alloc;

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::emitter::error::EmitterError;
use crate::envelope::EnvelopeKind;

pub(crate) const CLI_MAGIC: u32 = 0x5642_4C49;
pub(crate) const CLI_HEADER_LEN: u32 = 52;
pub(crate) const CLI_HEADER_BYTES: usize = 52;
pub(crate) const CLI_CRC_OFFSET: usize = 48;
pub(crate) const DIGEST_BYTES: usize = 32;
#[allow(dead_code)]
pub(crate) const MAX_CLI_PAYLOAD_BYTES: u32 = 16_777_216;
pub(crate) const BINARY_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CliHeader {
    pub magic: u32,
    pub schema_version: u16,
    pub kind: u16,
    pub header_len: u32,
    pub payload_len: u32,
    pub payload_digest: [u8; DIGEST_BYTES],
    #[allow(dead_code)]
    pub header_checksum: u32,
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, EmitterError> {
    let slice = bytes.get(offset..).ok_or(EmitterError::UnexpectedEof)?;
    if slice.len() < 2 {
        return Err(EmitterError::UnexpectedEof);
    }
    let arr: [u8; 2] = slice
        .get(..2)
        .ok_or(EmitterError::UnexpectedEof)?
        .try_into()
        .map_err(|_| EmitterError::UnexpectedEof)?;
    Ok(u16::from_le_bytes(arr))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, EmitterError> {
    let slice = bytes.get(offset..).ok_or(EmitterError::UnexpectedEof)?;
    if slice.len() < 4 {
        return Err(EmitterError::UnexpectedEof);
    }
    let arr: [u8; 4] = slice
        .get(..4)
        .ok_or(EmitterError::UnexpectedEof)?
        .try_into()
        .map_err(|_| EmitterError::UnexpectedEof)?;
    Ok(u32::from_le_bytes(arr))
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), EmitterError> {
    let slice = bytes
        .get_mut(offset..offset.saturating_add(2))
        .ok_or(EmitterError::UnexpectedEof)?;
    slice.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), EmitterError> {
    let slice = bytes
        .get_mut(offset..offset.saturating_add(4))
        .ok_or(EmitterError::UnexpectedEof)?;
    slice.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

pub(crate) fn build_cli_header(
    kind: EnvelopeKind,
    payload_len: u32,
    payload_bytes: &[u8],
) -> Result<[u8; CLI_HEADER_BYTES], EmitterError> {
    let mut header = [0u8; CLI_HEADER_BYTES];

    write_u32(&mut header, 0, CLI_MAGIC)?;
    write_u16(&mut header, 4, BINARY_SCHEMA_VERSION)?;

    let kind_u16 = kind.to_u16();
    write_u16(&mut header, 6, kind_u16)?;

    write_u32(&mut header, 8, CLI_HEADER_LEN)?;
    write_u32(&mut header, 12, payload_len)?;

    let digest = blake3::hash(payload_bytes);
    let digest_bytes = header.get_mut(16..48).ok_or(EmitterError::UnexpectedEof)?;
    digest_bytes.copy_from_slice(digest.as_bytes());

    let checksum = crc32c::crc32c(&header[..CLI_CRC_OFFSET]);
    write_u32(&mut header, CLI_CRC_OFFSET, checksum)?;

    Ok(header)
}

pub(crate) fn decode_cli_header(bytes: &[u8]) -> Result<CliHeader, EmitterError> {
    let magic = read_u32(bytes, 0)?;
    let schema_version = read_u16(bytes, 4)?;
    let kind = read_u16(bytes, 6)?;
    let header_len = read_u32(bytes, 8)?;
    let payload_len = read_u32(bytes, 12)?;

    let payload_digest = bytes
        .get(16..48)
        .ok_or(EmitterError::UnexpectedEof)?
        .try_into()
        .map_err(|_| EmitterError::UnexpectedEof)?;

    let header_checksum = read_u32(bytes, CLI_CRC_OFFSET)?;

    let crc_slice = bytes
        .get(..CLI_CRC_OFFSET)
        .ok_or(EmitterError::UnexpectedEof)?;
    let computed_crc = crc32c::crc32c(crc_slice);
    if computed_crc != header_checksum {
        return Err(EmitterError::HeaderChecksumMismatch);
    }

    Ok(CliHeader {
        magic,
        schema_version,
        kind,
        header_len,
        payload_len,
        payload_digest,
        header_checksum,
    })
}

pub fn encode_postcard<T: Serialize + core::fmt::Debug>(
    payload: &T,
    kind: EnvelopeKind,
    max_payload_len: u32,
) -> Result<Vec<u8>, EmitterError> {
    let payload_bytes =
        postcard::to_allocvec(payload).map_err(|_| EmitterError::PostcardEncodeFailed)?;

    let payload_len =
        u32::try_from(payload_bytes.len()).map_err(|_| EmitterError::PayloadLengthOverflow {
            len: u32::try_from(payload_bytes.len()).unwrap_or(u32::MAX),
        })?;

    if payload_len > max_payload_len {
        return Err(EmitterError::PayloadTooLarge {
            len: payload_len,
            max: max_payload_len,
        });
    }

    let capacity = CLI_HEADER_BYTES
        .checked_add(payload_bytes.len())
        .ok_or(EmitterError::LengthOverflow)?;

    let header = build_cli_header(kind, payload_len, &payload_bytes)?;

    let mut encoded = Vec::with_capacity(capacity);
    encoded.extend_from_slice(&header);
    encoded.extend_from_slice(&payload_bytes);
    Ok(encoded)
}

pub fn decode_postcard<'a, T: Deserialize<'a> + core::fmt::Debug>(
    bytes: &'a [u8],
    expected_kind: EnvelopeKind,
    max_payload_len: u32,
) -> Result<T, EmitterError> {
    if bytes.len() < CLI_HEADER_BYTES {
        return Err(EmitterError::UnexpectedEof);
    }

    let header = decode_cli_header(bytes)?;

    if header.magic != CLI_MAGIC {
        return Err(EmitterError::BadMagic {
            found: header.magic,
        });
    }

    if header.schema_version < BINARY_SCHEMA_VERSION {
        return Err(EmitterError::MigrationRequired {
            from: header.schema_version,
            to: BINARY_SCHEMA_VERSION,
        });
    }
    if header.schema_version > BINARY_SCHEMA_VERSION {
        return Err(EmitterError::UnsupportedSchemaVersion {
            version: header.schema_version,
        });
    }

    let kind_val = header.kind;
    let expected_u16 = expected_kind.to_u16();
    if kind_val != expected_u16 {
        return Err(EmitterError::UnknownKind { kind: kind_val });
    }

    if header.header_len != CLI_HEADER_LEN {
        return Err(EmitterError::HeaderLengthMismatch {
            found: header.header_len,
        });
    }

    if header.payload_len > max_payload_len {
        return Err(EmitterError::PayloadTooLarge {
            len: header.payload_len,
            max: max_payload_len,
        });
    }

    let payload_start = CLI_HEADER_BYTES;
    let payload_len_usize =
        usize::try_from(header.payload_len).map_err(|_| EmitterError::PayloadLengthOverflow {
            len: header.payload_len,
        })?;
    let payload_end = payload_start.checked_add(payload_len_usize).ok_or(
        EmitterError::PayloadLengthOverflow {
            len: header.payload_len,
        },
    )?;

    if bytes.len() < payload_end {
        return Err(EmitterError::UnexpectedEof);
    }

    let payload_bytes = bytes
        .get(payload_start..payload_end)
        .ok_or(EmitterError::UnexpectedEof)?;

    let computed_digest = blake3::hash(payload_bytes);
    if computed_digest.as_bytes() != &header.payload_digest {
        return Err(EmitterError::PayloadDigestMismatch);
    }

    postcard::from_bytes(payload_bytes).map_err(|_| EmitterError::PostcardDecodeFailed)
}

#[cfg(test)]
mod tests;
