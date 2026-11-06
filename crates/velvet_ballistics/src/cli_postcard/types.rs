//! CLI Postcard Types
//!
//! Core types for CLI Postcard binary format.
//! This CLI-output format has its own 52-byte little-endian header; it is
//! distinct from the 60-byte storage record envelope and the 24-byte IPC header.

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
/// - payload_digest: 32 bytes (BLAKE3)
/// - header_crc: 4 bytes
pub(crate) const HEADER_SIZE: usize = 52;
pub(crate) const HEADER_SIZE_U32: u32 = 52;
pub(crate) const MAX_PAYLOAD_U32: u32 = 64 * 1024;
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
    pub(crate) fn from_json_utf8(json_utf8: Vec<u8>) -> Result<Self, super::PostcardError> {
        if json_utf8.len() > MAX_PAYLOAD {
            return Err(super::PostcardError::PayloadTooLarge);
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
    /// Schema version as a little-endian u16.
    pub(crate) schema_version: u16,
    /// Kind enum as u16.
    pub(crate) kind: u16,
    /// Length of header in bytes.
    pub(crate) header_len: u32,
    /// Length of payload in bytes.
    pub(crate) payload_len: u32,
    /// BLAKE3 digest of payload (32 bytes).
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
    pub(crate) fn validate(&self) -> Result<(), super::PostcardError> {
        if self.magic != CLI_MAGIC {
            return Err(super::PostcardError::InvalidMagic);
        }
        if self.header_len != HEADER_SIZE_U32 {
            return Err(super::PostcardError::InvalidHeaderLength);
        }
        if self.payload_len > MAX_PAYLOAD_U32 {
            return Err(super::PostcardError::PayloadTooLarge);
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
    pub(crate) fn from_bytes(data: &[u8]) -> Result<Self, super::PostcardError> {
        if data.len() < HEADER_SIZE {
            return Err(super::PostcardError::DecodeFailed);
        }

        let magic = super::read_array::<4>(data, 0)?;
        let schema_version = u16::from_le_bytes(super::read_array::<2>(data, 4)?);
        let kind = u16::from_le_bytes(super::read_array::<2>(data, 6)?);
        let header_len = u32::from_le_bytes(super::read_array::<4>(data, 8)?);
        let payload_len = u32::from_le_bytes(super::read_array::<4>(data, 12)?);
        let payload_digest = super::read_array::<32>(data, 16)?;
        let header_crc = u32::from_le_bytes(super::read_array::<4>(data, 48)?);

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
