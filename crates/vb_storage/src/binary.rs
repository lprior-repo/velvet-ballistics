//! Binary serialization helpers for record encoding/decoding.
//!
//! Provides low-level byte manipulation for fixed-width record headers.

use crate::{
    constants::{CRC_OFFSET, DIGEST_BYTES},
    error::JournalError,
};

/// Reads a little-endian u16 from bytes at offset.
pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, JournalError> {
    let end = offset.checked_add(2).ok_or(JournalError::UnexpectedEof)?;
    let slice = bytes.get(offset..end).ok_or(JournalError::UnexpectedEof)?;
    let raw = <[u8; 2]>::try_from(slice).map_err(|_| JournalError::UnexpectedEof)?;
    Ok(u16::from_le_bytes(raw))
}

/// Reads a little-endian u32 from bytes at offset.
pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, JournalError> {
    let end = offset.checked_add(4).ok_or(JournalError::UnexpectedEof)?;
    let slice = bytes.get(offset..end).ok_or(JournalError::UnexpectedEof)?;
    let raw = <[u8; 4]>::try_from(slice).map_err(|_| JournalError::UnexpectedEof)?;
    Ok(u32::from_le_bytes(raw))
}

/// Reads a little-endian u64 from bytes at offset.
pub(crate) fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, JournalError> {
    let end = offset.checked_add(8).ok_or(JournalError::UnexpectedEof)?;
    let slice = bytes.get(offset..end).ok_or(JournalError::UnexpectedEof)?;
    let raw = <[u8; 8]>::try_from(slice).map_err(|_| JournalError::UnexpectedEof)?;
    Ok(u64::from_le_bytes(raw))
}

/// Writes a little-endian u16 to bytes at offset.
pub(crate) fn write_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), JournalError> {
    let end = offset.checked_add(2).ok_or(JournalError::UnexpectedEof)?;
    let target = bytes
        .get_mut(offset..end)
        .ok_or(JournalError::UnexpectedEof)?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

/// Writes a little-endian u32 to bytes at offset.
pub(crate) fn write_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), JournalError> {
    let end = offset.checked_add(4).ok_or(JournalError::UnexpectedEof)?;
    let target = bytes
        .get_mut(offset..end)
        .ok_or(JournalError::UnexpectedEof)?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

/// Writes a little-endian u64 to bytes at offset.
pub(crate) fn write_u64(bytes: &mut [u8], offset: usize, value: u64) -> Result<(), JournalError> {
    let end = offset.checked_add(8).ok_or(JournalError::UnexpectedEof)?;
    let target = bytes
        .get_mut(offset..end)
        .ok_or(JournalError::UnexpectedEof)?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

/// Writes a 32-byte digest to bytes at offset 24..CRC_OFFSET.
pub(crate) fn write_digest(
    bytes: &mut [u8],
    digest: &[u8; DIGEST_BYTES],
) -> Result<(), JournalError> {
    let target = bytes
        .get_mut(24..CRC_OFFSET)
        .ok_or(JournalError::UnexpectedEof)?;
    target.copy_from_slice(digest);
    Ok(())
}
