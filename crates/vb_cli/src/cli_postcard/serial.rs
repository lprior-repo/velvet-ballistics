//! CLI Postcard Serialization Helpers
//!
//! Postcard header layout, wire parsing, and validation.
//!
//! vb-k8ut.5: the header is a fixed 52-byte wire structure with magic,
//! schema version, kind tag, header length, payload length, SHA-256
//! digest, and CRC32 checksum.

use super::constants::*;
use super::error::PostcardError;

/// Low-level helper: read a fixed-size byte slice from `data` at `start`.
///
/// Returns `DecodeFailed` on bounds errors.
pub(crate) fn read_array<const N: usize>(data: &[u8], start: usize) -> Result<[u8; N], PostcardError> {
    let end = start.checked_add(N).ok_or(PostcardError::DecodeFailed)?;
    let bytes = data.get(start..end).ok_or(PostcardError::DecodeFailed)?;
    <[u8; N]>::try_from(bytes).map_err(|_| PostcardError::DecodeFailed)
}

/// Postcard header structure for CLI output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostcardHeader {
    pub(crate) magic: [u8; 4],
    pub(crate) schema_version: u16,
    pub(crate) kind: u16,
    pub(crate) header_len: u32,
    pub(crate) payload_len: u32,
    pub(crate) payload_digest: [u8; 32],
    pub(crate) header_crc: u32,
}

impl PostcardHeader {
    /// INV-005: Bounded allocation gate.
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
