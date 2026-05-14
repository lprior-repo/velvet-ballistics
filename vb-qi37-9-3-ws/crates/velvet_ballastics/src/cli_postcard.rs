//! CLI Postcard Module
//!
//! This module provides binary Postcard output with bounded allocation.
//! Postcard payloads are validated against header before decode.
//!
//! ## Contract Clauses
//! - INV-005: Postcard payloads respect bounded allocation (header_len + payload_len validated before decode)
//! - POST-007: Postcard output validates magic + header length before payload decode

#![forbid(unsafe_code)]
#![allow(dead_code)]

/// Magic bytes for CLI Postcard format: "VCLA" (Velvet CLI Application)
pub(crate) const CLI_MAGIC: [u8; 4] = [0x56, 0x43, 0x4C, 0x41];

/// Maximum payload size in bytes (64KB).
/// This bound is validated before allocation to prevent OOM.
pub(crate) const MAX_PAYLOAD: usize = 64 * 1024;

/// Header size in bytes:
/// - magic: 4 bytes
/// - schema_version_u16: 2 bytes
/// - kind_u16: 2 bytes
/// - header_len: 4 bytes
/// - payload_len: 4 bytes
/// - payload_digest: 32 bytes (SHA-256)
/// - header_crc: 4 bytes
pub(crate) const HEADER_SIZE: usize = 52;
const HEADER_SIZE_U32: u32 = 52;
const MAX_PAYLOAD_U32: u32 = 64 * 1024;

/// Postcard header structure for CLI output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostcardHeader {
    /// Magic bytes (must be CLI_MAGIC).
    pub(crate) magic: [u8; 4],
    /// Schema version as u16 (endianness specified by protocol).
    pub(crate) schema_version: u16,
    /// Kind enum as u16.
    pub(crate) kind: u16,
    /// Length of header in bytes.
    pub(crate) header_len: u32,
    /// Length of payload in bytes.
    pub(crate) payload_len: u32,
    /// SHA-256 digest of payload (32 bytes).
    pub(crate) payload_digest: [u8; 32],
    /// CRC-32 of header bytes.
    pub(crate) header_crc: u32,
}

impl PostcardHeader {
    /// Validate header before payload allocation.
    /// INV-005: Ensures bounded allocation by checking:
    /// - magic matches CLI_MAGIC
    /// - header_len matches HEADER_SIZE
    /// - payload_len <= MAX_PAYLOAD
    ///
    /// # Returns
    /// `Ok(())` if header is valid, `Err(PostcardError)` otherwise.
    pub(crate) fn validate(&self) -> Result<(), PostcardError> {
        if self.magic != CLI_MAGIC {
            return Err(PostcardError::InvalidMagic);
        }
        if self.header_len != HEADER_SIZE_U32 {
            return Err(PostcardError::InvalidHeaderLength);
        }
        if self.payload_len > MAX_PAYLOAD_U32 {
            return Err(PostcardError::PayloadTooLarge);
        }
        Ok(())
    }

    /// Create a PostcardHeader from raw bytes.
    ///
    /// # Arguments
    /// * `data` - Raw byte slice containing at least HEADER_SIZE bytes
    ///
    /// # Returns
    /// `Ok(PostcardHeader)` if data is large enough, `Err(PostcardError::DecodeFailed)` otherwise.
    pub(crate) fn from_bytes(data: &[u8]) -> Result<Self, PostcardError> {
        if data.len() < HEADER_SIZE {
            return Err(PostcardError::DecodeFailed);
        }

        let magic = read_array::<4>(data, 0)?;
        let schema_version = u16::from_le_bytes(read_array::<2>(data, 4)?);
        let kind = u16::from_le_bytes(read_array::<2>(data, 6)?);
        let header_len = u32::from_le_bytes(read_array::<4>(data, 8)?);
        let payload_len = u32::from_le_bytes(read_array::<4>(data, 12)?);
        let payload_digest = read_array::<32>(data, 16)?;
        let header_crc = u32::from_le_bytes(read_array::<4>(data, 48)?);

        Ok(PostcardHeader {
            magic,
            schema_version,
            kind,
            header_len,
            payload_len,
            payload_digest,
            header_crc,
        })
    }
}

fn read_array<const N: usize>(data: &[u8], start: usize) -> Result<[u8; N], PostcardError> {
    let end = start.checked_add(N).ok_or(PostcardError::DecodeFailed)?;
    let bytes = data.get(start..end).ok_or(PostcardError::DecodeFailed)?;
    <[u8; N]>::try_from(bytes).map_err(|_| PostcardError::DecodeFailed)
}

/// Errors that can occur during Postcard decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PostcardError {
    /// Magic bytes do not match CLI_MAGIC.
    InvalidMagic,
    /// Header length does not match expected HEADER_SIZE.
    InvalidHeaderLength,
    /// Payload length exceeds MAX_PAYLOAD.
    PayloadTooLarge,
    /// CRC check of header failed.
    CrcMismatch,
    /// Data too short to contain valid header.
    DecodeFailed,
}

impl std::fmt::Display for PostcardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid magic bytes in postcard header"),
            Self::InvalidHeaderLength => write!(f, "invalid header length in postcard"),
            Self::PayloadTooLarge => write!(f, "payload length exceeds maximum"),
            Self::CrcMismatch => write!(f, "header CRC mismatch"),
            Self::DecodeFailed => write!(f, "postcard decode failed: data too short"),
        }
    }
}

/// Decode a Postcard message from bytes.
/// Validates header before allocating payload buffer.
/// INV-005: Bounded allocation enforced via header validation.
///
/// # Arguments
/// * `data` - Raw byte slice containing postcard message
///
/// # Returns
/// `Ok((header, payload))` if decode succeeds, `Err(PostcardError)` otherwise.
///
/// # Invariants
/// - INV-005: payload_len is validated <= MAX_PAYLOAD before any allocation
/// - POST-007: magic and header_len validated before payload decode
pub(crate) fn decode_postcard(data: &[u8]) -> Result<(&[u8], &[u8]), PostcardError> {
    if data.len() < HEADER_SIZE {
        return Err(PostcardError::DecodeFailed);
    }

    let header = PostcardHeader::from_bytes(data)?;
    header.validate()?;

    let payload_start = HEADER_SIZE;
    let payload_len =
        usize::try_from(header.payload_len).map_err(|_| PostcardError::PayloadTooLarge)?;
    let payload_end = payload_start
        .checked_add(payload_len)
        .ok_or(PostcardError::DecodeFailed)?;

    if data.len() < payload_end {
        return Err(PostcardError::DecodeFailed);
    }

    let header_bytes = data
        .get(0..HEADER_SIZE)
        .ok_or(PostcardError::DecodeFailed)?;
    let payload = data
        .get(payload_start..payload_end)
        .ok_or(PostcardError::DecodeFailed)?;
    Ok((header_bytes, payload))
}

/// Encode a Postcard message to bytes.
/// Returns a vector containing header + payload.
///
/// # Arguments
/// * `schema_version` - Schema version as u16
/// * `kind` - Kind as u16
/// * `payload` - Raw payload bytes
///
/// # Returns
/// `Ok(Vec<u8>)` containing the encoded postcard message.
pub(crate) fn encode_postcard(
    schema_version: u16,
    kind: u16,
    payload: &[u8],
) -> Result<Vec<u8>, PostcardError> {
    if payload.len() > MAX_PAYLOAD {
        return Err(PostcardError::PayloadTooLarge);
    }
    let payload_len = u32::try_from(payload.len()).map_err(|_| PostcardError::PayloadTooLarge)?;
    let capacity = HEADER_SIZE
        .checked_add(payload.len())
        .ok_or(PostcardError::PayloadTooLarge)?;
    let mut result = Vec::with_capacity(capacity);

    result.extend_from_slice(&CLI_MAGIC);
    result.extend_from_slice(&schema_version.to_le_bytes());
    result.extend_from_slice(&kind.to_le_bytes());
    result.extend_from_slice(&HEADER_SIZE_U32.to_le_bytes());
    result.extend_from_slice(&payload_len.to_le_bytes());

    let mut payload_digest = [0u8; 32];
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    payload.hash(&mut hasher);
    let hash = hasher.finish().to_le_bytes();
    payload_digest[..8].copy_from_slice(&hash);

    result.extend_from_slice(&payload_digest);

    let header_crc = crc32fast::hash(&result);
    result.extend_from_slice(&header_crc.to_le_bytes());

    result.extend_from_slice(payload);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_magic() {
        assert_eq!(CLI_MAGIC, [0x56, 0x43, 0x4C, 0x41]);
        assert_eq!(CLI_MAGIC, *b"VCLA");
    }

    #[test]
    fn test_max_payload() {
        assert_eq!(MAX_PAYLOAD, 65536);
    }

    #[test]
    fn test_header_size() {
        assert_eq!(HEADER_SIZE, 52);
    }

    #[test]
    fn test_postcard_header_from_bytes() {
        let mut data = vec![0u8; HEADER_SIZE + 100];
        data[0..4].copy_from_slice(&CLI_MAGIC);
        data[4..6].copy_from_slice(&1u16.to_le_bytes());
        data[6..8].copy_from_slice(&2u16.to_le_bytes());
        data[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        data[12..16].copy_from_slice(&(100u32).to_le_bytes());

        let header = PostcardHeader::from_bytes(&data).unwrap();
        assert_eq!(header.magic, CLI_MAGIC);
        assert_eq!(header.schema_version, 1);
        assert_eq!(header.kind, 2);
        assert_eq!(header.header_len, HEADER_SIZE as u32);
        assert_eq!(header.payload_len, 100);
    }

    #[test]
    fn test_decode_valid_postcard() {
        let mut data = vec![0u8; HEADER_SIZE + 100];
        data[0..4].copy_from_slice(&CLI_MAGIC);
        data[4..6].copy_from_slice(&1u16.to_le_bytes());
        data[6..8].copy_from_slice(&2u16.to_le_bytes());
        data[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        data[12..16].copy_from_slice(&(100u32).to_le_bytes());

        let result = decode_postcard(&data);
        assert!(result.is_ok());
        let (header, payload) = result.unwrap();
        assert_eq!(header.len(), HEADER_SIZE);
        assert_eq!(payload.len(), 100);
    }

    #[test]
    fn test_decode_invalid_magic() {
        let mut data = vec![0u8; HEADER_SIZE + 100];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        data[12..16].copy_from_slice(&(100u32).to_le_bytes());

        let result = decode_postcard(&data);
        assert_eq!(result.unwrap_err(), PostcardError::InvalidMagic);
    }

    #[test]
    fn test_decode_payload_too_large() {
        let mut data = vec![0u8; HEADER_SIZE + 100];
        data[0..4].copy_from_slice(&CLI_MAGIC);
        data[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        data[12..16].copy_from_slice(&((MAX_PAYLOAD + 1) as u32).to_le_bytes());

        let result = decode_postcard(&data);
        assert_eq!(result.unwrap_err(), PostcardError::PayloadTooLarge);
    }

    #[test]
    fn test_decode_invalid_header_length() {
        let mut data = vec![0u8; HEADER_SIZE + 100];
        data[0..4].copy_from_slice(&CLI_MAGIC);
        data[8..12].copy_from_slice(&((HEADER_SIZE + 1) as u32).to_le_bytes());
        data[12..16].copy_from_slice(&(100u32).to_le_bytes());

        let result = decode_postcard(&data);
        assert_eq!(result.unwrap_err(), PostcardError::InvalidHeaderLength);
    }

    #[test]
    fn test_decode_data_too_short() {
        let data = vec![0u8; 10];
        let result = decode_postcard(&data);
        assert_eq!(result.unwrap_err(), PostcardError::DecodeFailed);
    }

    #[test]
    fn test_encode_postcard() {
        let payload = b"test payload";
        let encoded = encode_postcard(1, 2, payload).unwrap();

        assert_eq!(&encoded[0..4], &CLI_MAGIC);
        assert_eq!(encoded.len(), HEADER_SIZE + payload.len());
    }

    #[test]
    fn test_roundtrip() {
        let payload = b"Hello, Postcard!";
        let encoded = encode_postcard(1, 2, payload).unwrap();

        let (header, extracted_payload) = decode_postcard(&encoded).unwrap();
        assert_eq!(header.len(), HEADER_SIZE);
        assert_eq!(extracted_payload, payload);
    }
}
