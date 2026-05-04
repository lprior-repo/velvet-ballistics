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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // read_u16 / write_u16 round-trips
    // =========================================================================

    #[test]
    fn u16_roundtrip_zero() -> Result<(), JournalError> {
        let mut buf = [0u8; 4];
        write_u16(&mut buf, 0, 0)?;
        assert_eq!(read_u16(&buf, 0)?, 0u16);
        Ok(())
    }

    #[test]
    fn u16_roundtrip_max() -> Result<(), JournalError> {
        let mut buf = [0u8; 4];
        write_u16(&mut buf, 0, u16::MAX)?;
        assert_eq!(read_u16(&buf, 0)?, u16::MAX);
        Ok(())
    }

    #[test]
    fn u16_roundtrip_at_end_of_buffer() -> Result<(), JournalError> {
        let mut buf = [0u8; 6];
        write_u16(&mut buf, 4, 0xABCD)?;
        assert_eq!(read_u16(&buf, 4)?, 0xABCDu16);
        Ok(())
    }

    #[test]
    fn u16_roundtrip_various_values() -> Result<(), JournalError> {
        let mut buf = [0u8; 8];
        for &val in &[1u16, 256, 0x1234, 0xFFFF, 0x0001] {
            buf.fill(0);
            write_u16(&mut buf, 0, val)?;
            assert_eq!(read_u16(&buf, 0)?, val, "roundtrip failed for {val:#06x}");
        }
        Ok(())
    }

    #[test]
    fn u16_read_rejects_offset_at_boundary() {
        let buf = [0u8; 2];
        let result = read_u16(&buf, 2);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "reading past buffer must yield UnexpectedEof, got {result:?}"
        );
    }

    #[test]
    fn u16_read_rejects_offset_one_from_end() {
        let buf = [0u8; 3];
        let result = read_u16(&buf, 2);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "reading with only 1 byte remaining must yield UnexpectedEof, got {result:?}"
        );
    }

    #[test]
    fn u16_write_rejects_offset_at_boundary() {
        let mut buf = [0u8; 2];
        let result = write_u16(&mut buf, 2, 0);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "writing past buffer must yield UnexpectedEof, got {result:?}"
        );
    }

    #[test]
    fn u16_read_rejects_empty_buffer() {
        let buf: [u8; 0] = [];
        let result = read_u16(&buf, 0);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "empty buffer must yield UnexpectedEof, got {result:?}"
        );
    }

    // =========================================================================
    // read_u32 / write_u32 round-trips
    // =========================================================================

    #[test]
    fn u32_roundtrip_zero() -> Result<(), JournalError> {
        let mut buf = [0u8; 8];
        write_u32(&mut buf, 0, 0)?;
        assert_eq!(read_u32(&buf, 0)?, 0u32);
        Ok(())
    }

    #[test]
    fn u32_roundtrip_max() -> Result<(), JournalError> {
        let mut buf = [0u8; 8];
        write_u32(&mut buf, 0, u32::MAX)?;
        assert_eq!(read_u32(&buf, 0)?, u32::MAX);
        Ok(())
    }

    #[test]
    fn u32_roundtrip_various_values() -> Result<(), JournalError> {
        let mut buf = [0u8; 8];
        for &val in &[1u32, 0x01020304, 0x80000000, 0x7FFFFFFF, 42] {
            buf.fill(0);
            write_u32(&mut buf, 0, val)?;
            assert_eq!(read_u32(&buf, 0)?, val, "roundtrip failed for {val:#010x}");
        }
        Ok(())
    }

    #[test]
    fn u32_roundtrip_at_nonzero_offset() -> Result<(), JournalError> {
        let mut buf = [0u8; 12];
        write_u32(&mut buf, 4, 0xDEADBEEF)?;
        assert_eq!(read_u32(&buf, 4)?, 0xDEADBEEFu32);
        // Bytes before and after should still be zero
        assert_eq!(&buf[0..4], &[0, 0, 0, 0]);
        assert_eq!(&buf[8..12], &[0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn u32_read_rejects_offset_three_from_end() {
        let buf = [0u8; 6];
        let result = read_u32(&buf, 3);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "reading with only 3 bytes remaining must yield UnexpectedEof, got {result:?}"
        );
    }

    #[test]
    fn u32_write_rejects_offset_at_boundary() {
        let mut buf = [0u8; 4];
        let result = write_u32(&mut buf, 4, 0);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "writing past buffer must yield UnexpectedEof, got {result:?}"
        );
    }

    // =========================================================================
    // read_u64 / write_u64 round-trips
    // =========================================================================

    #[test]
    fn u64_roundtrip_zero() -> Result<(), JournalError> {
        let mut buf = [0u8; 16];
        write_u64(&mut buf, 0, 0)?;
        assert_eq!(read_u64(&buf, 0)?, 0u64);
        Ok(())
    }

    #[test]
    fn u64_roundtrip_max() -> Result<(), JournalError> {
        let mut buf = [0u8; 16];
        write_u64(&mut buf, 0, u64::MAX)?;
        assert_eq!(read_u64(&buf, 0)?, u64::MAX);
        Ok(())
    }

    #[test]
    fn u64_roundtrip_various_values() -> Result<(), JournalError> {
        let mut buf = [0u8; 16];
        for &val in &[1u64, 256, 0x123456789ABCDEF0, u64::MAX / 2] {
            buf.fill(0);
            write_u64(&mut buf, 0, val)?;
            assert_eq!(read_u64(&buf, 0)?, val, "roundtrip failed for {val:#018x}");
        }
        Ok(())
    }

    #[test]
    fn u64_roundtrip_at_nonzero_offset() -> Result<(), JournalError> {
        let mut buf = [0u8; 24];
        write_u64(&mut buf, 8, 0xCAFEBABE_DEADBEEF)?;
        assert_eq!(read_u64(&buf, 8)?, 0xCAFEBABE_DEADBEEFu64);
        Ok(())
    }

    #[test]
    fn u64_read_rejects_offset_seven_from_end() {
        let buf = [0u8; 14];
        let result = read_u64(&buf, 7);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "reading with only 7 bytes remaining must yield UnexpectedEof, got {result:?}"
        );
    }

    #[test]
    fn u64_write_rejects_offset_near_boundary() {
        let mut buf = [0u8; 8];
        let result = write_u64(&mut buf, 1, 0);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "writing with only 7 bytes available must yield UnexpectedEof, got {result:?}"
        );
    }

    // =========================================================================
    // write_digest
    // =========================================================================

    #[test]
    fn write_digest_roundtrip_in_full_header() -> Result<(), JournalError> {
        let mut buf = [0u8; 60]; // CRC_OFFSET = 56, so 24..56 = 32 bytes
        let digest = [0xAB; DIGEST_BYTES];
        write_digest(&mut buf, &digest)?;
        // The 32 bytes at 24..56 should match
        let written = &buf[24..56];
        assert_eq!(written, &digest);
        // Bytes outside the digest range should be zero
        assert!(buf.iter().take(24).all(|&b| b == 0), "prefix must be zeroed");
        assert!(buf.iter().skip(56).all(|&b| b == 0), "suffix must be zeroed");
        Ok(())
    }

    #[test]
    fn write_digest_all_zero() -> Result<(), JournalError> {
        let mut buf = [0u8; 60];
        let digest = [0x00; DIGEST_BYTES];
        write_digest(&mut buf, &digest)?;
        assert!(buf.iter().all(|&b| b == 0), "all-zero digest into all-zero buffer");
        Ok(())
    }

    #[test]
    fn write_digest_rejects_buffer_too_small() {
        let mut buf = [0u8; 55]; // Less than CRC_OFFSET (56)
        let digest = [0xFF; DIGEST_BYTES];
        let result = write_digest(&mut buf, &digest);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "buffer smaller than 56 bytes must yield UnexpectedEof, got {result:?}"
        );
    }

    #[test]
    fn write_digest_rejects_empty_buffer() {
        let mut buf: [u8; 0] = [];
        let digest = [0xFF; DIGEST_BYTES];
        let result = write_digest(&mut buf, &digest);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "empty buffer must yield UnexpectedEof, got {result:?}"
        );
    }

    // =========================================================================
    // Little-endian byte order verification
    // =========================================================================

    #[test]
    fn u16_writes_little_endian() -> Result<(), JournalError> {
        let mut buf = [0u8; 2];
        write_u16(&mut buf, 0, 0x0102)?;
        // Little-endian: LSB first
        assert_eq!(buf[0], 0x02);
        assert_eq!(buf[1], 0x01);
        Ok(())
    }

    #[test]
    fn u32_writes_little_endian() -> Result<(), JournalError> {
        let mut buf = [0u8; 4];
        write_u32(&mut buf, 0, 0x01020304)?;
        assert_eq!(buf[0], 0x04);
        assert_eq!(buf[1], 0x03);
        assert_eq!(buf[2], 0x02);
        assert_eq!(buf[3], 0x01);
        Ok(())
    }

    #[test]
    fn u64_writes_little_endian() -> Result<(), JournalError> {
        let mut buf = [0u8; 8];
        write_u64(&mut buf, 0, 0x0102030405060708)?;
        assert_eq!(buf[0], 0x08);
        assert_eq!(buf[7], 0x01);
        Ok(())
    }

    // =========================================================================
    // Multiple writes to same buffer
    // =========================================================================

    #[test]
    fn multiple_writes_in_same_buffer() -> Result<(), JournalError> {
        let mut buf = [0u8; 14];
        write_u16(&mut buf, 0, 0x1234)?;
        write_u32(&mut buf, 2, 0xDEADBEEF)?;
        write_u64(&mut buf, 6, 0xCAFEBABE_DEADBEEF)?;
        assert_eq!(read_u16(&buf, 0)?, 0x1234u16);
        assert_eq!(read_u32(&buf, 2)?, 0xDEADBEEFu32);
        assert_eq!(read_u64(&buf, 6)?, 0xCAFEBABE_DEADBEEFu64);
        Ok(())
    }
}
