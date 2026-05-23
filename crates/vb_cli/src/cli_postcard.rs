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

use serde::{Deserialize, Serialize};

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
pub(crate) const CLI_SCHEMA_VERSION: u16 = 1;
pub(crate) const CLI_POSTCARD_KIND: u16 = 2;

/// Machine-readable CLI payload content carried inside the postcard frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CliPostcardContentType {
    JsonUtf8,
}

/// Versioned CLI payload carried by the outer postcard frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CliPostcardPayload {
    pub(crate) schema_version: u16,
    pub(crate) kind: u16,
    pub(crate) content_type: CliPostcardContentType,
    pub(crate) json_utf8: Vec<u8>,
}

impl CliPostcardPayload {
    pub(crate) fn from_json_utf8(json_utf8: Vec<u8>) -> Result<Self, PostcardError> {
        if json_utf8.len() > MAX_PAYLOAD {
            return Err(PostcardError::PayloadTooLarge);
        }
        Ok(Self {
            schema_version: CLI_SCHEMA_VERSION,
            kind: CLI_POSTCARD_KIND,
            content_type: CliPostcardContentType::JsonUtf8,
            json_utf8,
        })
    }
}

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
    /// Schema version is older than the supported contract.
    VersionTooOld,
    /// Schema version is newer than the supported contract.
    VersionTooNew,
    /// Message kind is not the supported CLI postcard payload kind.
    WrongKind,
    /// Payload digest check failed.
    DigestMismatch,
    /// CRC check of header failed.
    CrcMismatch,
    /// The decoded payload metadata does not match the supported CLI contract.
    PayloadMetadataMismatch,
    /// The decoded CLI payload body is not valid UTF-8 JSON.
    JsonPayloadDecodeFailed,
    /// Data too short to contain valid header.
    DecodeFailed,
}

impl std::fmt::Display for PostcardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid magic bytes in postcard header"),
            Self::InvalidHeaderLength => write!(f, "invalid header length in postcard"),
            Self::PayloadTooLarge => write!(f, "payload length exceeds maximum"),
            Self::VersionTooOld => write!(f, "postcard schema version is too old"),
            Self::VersionTooNew => write!(f, "postcard schema version is too new"),
            Self::WrongKind => write!(f, "postcard kind is not supported"),
            Self::DigestMismatch => write!(f, "payload digest mismatch"),
            Self::CrcMismatch => write!(f, "header CRC mismatch"),
            Self::PayloadMetadataMismatch => write!(f, "postcard payload metadata mismatch"),
            Self::JsonPayloadDecodeFailed => write!(f, "postcard JSON payload decode failed"),
            Self::DecodeFailed => write!(f, "postcard decode failed: data too short"),
        }
    }
}

impl std::error::Error for PostcardError {}

fn validate_cli_payload(payload: &CliPostcardPayload) -> Result<(), PostcardError> {
    if payload.schema_version != CLI_SCHEMA_VERSION {
        return Err(PostcardError::PayloadMetadataMismatch);
    }
    if payload.kind != CLI_POSTCARD_KIND {
        return Err(PostcardError::PayloadMetadataMismatch);
    }
    if payload.content_type != CliPostcardContentType::JsonUtf8 {
        return Err(PostcardError::PayloadMetadataMismatch);
    }
    Ok(())
}

pub(crate) fn decode_cli_payload(payload: &[u8]) -> Result<CliPostcardPayload, PostcardError> {
    postcard::from_bytes::<CliPostcardPayload>(payload).map_err(|_| PostcardError::DecodeFailed)
}

pub(crate) fn decode_postcard_json(
    data: &[u8],
) -> Result<(PostcardHeader, serde_json::Value), PostcardError> {
    let (header_bytes, payload_bytes) = decode_postcard(data)?;
    let header = PostcardHeader::from_bytes(header_bytes)?;
    let payload = decode_cli_payload(payload_bytes)?;
    validate_cli_payload(&payload)?;
    let value = serde_json::from_slice::<serde_json::Value>(&payload.json_utf8)
        .map_err(|_| PostcardError::JsonPayloadDecodeFailed)?;
    Ok((header, value))
}

fn payload_digest(payload: &[u8]) -> [u8; 32] {
    let digest = blake3::hash(payload);
    let mut out = [0u8; 32];
    out.copy_from_slice(digest.as_bytes());
    out
}

fn validate_header_crc(header_bytes: &[u8]) -> Result<(), PostcardError> {
    let crc_input = header_bytes.get(0..48).ok_or(PostcardError::DecodeFailed)?;
    let expected_bytes = header_bytes
        .get(48..52)
        .ok_or(PostcardError::DecodeFailed)?;
    let expected = u32::from_le_bytes(
        <[u8; 4]>::try_from(expected_bytes).map_err(|_| PostcardError::DecodeFailed)?,
    );
    let actual = crc32fast::hash(crc_input);
    if actual == expected {
        Ok(())
    } else {
        Err(PostcardError::CrcMismatch)
    }
}

fn validate_version_and_kind(header: &PostcardHeader) -> Result<(), PostcardError> {
    if header.schema_version == 0 {
        return Err(PostcardError::VersionTooOld);
    }
    if header.schema_version > CLI_SCHEMA_VERSION {
        return Err(PostcardError::VersionTooNew);
    }
    if header.kind != CLI_POSTCARD_KIND {
        return Err(PostcardError::WrongKind);
    }
    Ok(())
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
    validate_version_and_kind(&header)?;

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
    validate_header_crc(header_bytes)?;
    if payload_digest(payload) != header.payload_digest {
        return Err(PostcardError::DigestMismatch);
    }
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

    result.extend_from_slice(&payload_digest(payload));

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
        let data = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, &[0u8; 100]);

        let header = PostcardHeader::from_bytes(&data).expect("test header decodes");
        assert_eq!(header.magic, CLI_MAGIC);
        assert_eq!(header.schema_version, CLI_SCHEMA_VERSION);
        assert_eq!(header.kind, CLI_POSTCARD_KIND);
        assert_eq!(header.header_len, HEADER_SIZE_U32);
        assert_eq!(header.payload_len, 100);
    }

    #[test]
    fn test_decode_valid_postcard() {
        let data = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, &[0u8; 100]);

        let (header, payload) = decode_postcard(&data).expect("valid postcard decodes");
        assert_eq!(header.len(), HEADER_SIZE);
        assert_eq!(payload.len(), 100);
    }

    #[test]
    fn test_decode_invalid_magic() {
        let mut data = vec![0u8; HEADER_SIZE + 100];
        write_test_bytes(&mut data, 0..4, &[0x00, 0x00, 0x00, 0x00]);
        write_test_bytes(&mut data, 12..16, &(100u32).to_le_bytes());

        let result = decode_postcard(&data);
        assert_eq!(result, Err(PostcardError::InvalidMagic));
    }

    #[test]
    fn test_decode_payload_too_large() {
        let mut data = vec![0u8; HEADER_SIZE + 100];
        write_test_header_prefix(&mut data, MAX_PAYLOAD_U32.saturating_add(1));

        let result = decode_postcard(&data);
        assert_eq!(result, Err(PostcardError::PayloadTooLarge));
    }

    #[test]
    fn test_decode_invalid_header_length() {
        let mut data = vec![0u8; HEADER_SIZE + 100];
        write_test_bytes(&mut data, 0..4, &CLI_MAGIC);
        write_test_bytes(&mut data, 4..6, &CLI_SCHEMA_VERSION.to_le_bytes());
        write_test_bytes(&mut data, 6..8, &CLI_POSTCARD_KIND.to_le_bytes());
        write_test_bytes(
            &mut data,
            8..12,
            &HEADER_SIZE_U32.saturating_add(1).to_le_bytes(),
        );
        write_test_bytes(&mut data, 12..16, &(100u32).to_le_bytes());

        let result = decode_postcard(&data);
        assert_eq!(result, Err(PostcardError::InvalidHeaderLength));
    }

    #[test]
    fn test_decode_data_too_short() {
        let data = vec![0u8; 10];
        let result = decode_postcard(&data);
        assert_eq!(result, Err(PostcardError::DecodeFailed));
    }

    #[test]
    fn test_encode_postcard() {
        let payload = b"test payload";
        let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, payload);

        assert_eq!(encoded.get(0..4), Some(CLI_MAGIC.as_slice()));
        assert_eq!(encoded.len(), HEADER_SIZE + payload.len());
    }

    #[test]
    fn test_roundtrip() {
        let payload = b"Hello, Postcard!";
        let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, payload);

        let (header, extracted_payload) = decode_postcard(&encoded).expect("roundtrip decodes");
        assert_eq!(header.len(), HEADER_SIZE);
        assert_eq!(extracted_payload, payload);
    }

    #[test]
    fn decode_rejects_corrupted_crc_before_exposure() {
        let mut encoded = encode_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload")
            .expect("test postcard encodes");
        assert!(encoded.get(48).is_some());
        if let Some(byte) = encoded.get_mut(48) {
            *byte ^= 0x01;
        }
        assert_eq!(decode_postcard(&encoded), Err(PostcardError::CrcMismatch));
    }

    #[test]
    fn decode_rejects_corrupted_digest_before_exposure() {
        let mut encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
        assert!(encoded.get(16).is_some());
        if let Some(byte) = encoded.get_mut(16) {
            *byte ^= 0x01;
        }
        let crc = encoded.get(0..48).map_or(0, crc32fast::hash);
        write_test_bytes(&mut encoded, 48..52, &crc.to_le_bytes());
        assert_eq!(
            decode_postcard(&encoded),
            Err(PostcardError::DigestMismatch)
        );
    }

    #[test]
    fn decode_rejects_old_and_future_versions() {
        let old = encode_test_postcard(0, CLI_POSTCARD_KIND, b"payload");
        let future = encode_postcard(
            CLI_SCHEMA_VERSION.saturating_add(1),
            CLI_POSTCARD_KIND,
            b"payload",
        )
        .expect("future-version postcard encodes");
        assert_eq!(decode_postcard(&old), Err(PostcardError::VersionTooOld));
        assert_eq!(decode_postcard(&future), Err(PostcardError::VersionTooNew));
    }

    #[test]
    fn decode_rejects_wrong_kind() {
        let encoded = encode_postcard(
            CLI_SCHEMA_VERSION,
            CLI_POSTCARD_KIND.saturating_add(1),
            b"payload",
        )
        .expect("wrong-kind postcard encodes");
        assert_eq!(decode_postcard(&encoded), Err(PostcardError::WrongKind));
    }

    #[test]
    fn decode_rejects_max_plus_one_payload_before_exposure() {
        let mut encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
        write_test_bytes(
            &mut encoded,
            12..16,
            &MAX_PAYLOAD_U32.saturating_add(1).to_le_bytes(),
        );
        let crc = encoded.get(0..48).map_or(0, crc32fast::hash);
        write_test_bytes(&mut encoded, 48..52, &crc.to_le_bytes());
        assert_eq!(
            decode_postcard(&encoded),
            Err(PostcardError::PayloadTooLarge)
        );
    }

    #[test]
    fn decode_rejects_truncated_header() {
        let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
        let truncated = encoded
            .get(0..HEADER_SIZE.saturating_sub(1))
            .map_or(&[][..], |slice| slice);
        assert_eq!(decode_postcard(truncated), Err(PostcardError::DecodeFailed));
    }

    fn encode_test_postcard(schema_version: u16, kind: u16, payload: &[u8]) -> Vec<u8> {
        encode_postcard(schema_version, kind, payload).expect("test postcard encodes")
    }

    fn write_test_header_prefix(data: &mut [u8], payload_len: u32) {
        write_test_bytes(data, 0..4, &CLI_MAGIC);
        write_test_bytes(data, 4..6, &CLI_SCHEMA_VERSION.to_le_bytes());
        write_test_bytes(data, 6..8, &CLI_POSTCARD_KIND.to_le_bytes());
        write_test_bytes(data, 8..12, &HEADER_SIZE_U32.to_le_bytes());
        write_test_bytes(data, 12..16, &payload_len.to_le_bytes());
    }

    fn write_test_bytes(data: &mut [u8], range: std::ops::Range<usize>, bytes: &[u8]) {
        assert_eq!(range.len(), bytes.len());
        assert!(data.get_mut(range.clone()).is_some());
        if let Some(target) = data.get_mut(range) {
            target.copy_from_slice(bytes);
        }
    }
}
