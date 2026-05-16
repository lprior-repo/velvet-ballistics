#![forbid(unsafe_code)]
//! IPC frame validation utilities.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

use crate::{IpcError, IpcFrameHeader, MaxPayloadBytes, IPC_MAGIC};

/// Validates that the magic bytes are correct before any allocation occurs.
pub fn validate_frame_magic(bytes: &[u8]) -> Result<(), IpcError> {
    if bytes.len() < 4 {
        return Err(IpcError::HeaderDecodeFailed);
    }
    let mut cursor = Cursor::new(bytes);
    let magic = cursor
        .read_u32::<LittleEndian>()
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    if magic != IPC_MAGIC {
        return Err(IpcError::InvalidMagic { actual: magic });
    }
    Ok(())
}

/// Validates that the payload length does not exceed the maximum before reading.
pub fn validate_frame_bounds(
    header: &IpcFrameHeader,
    max_payload: MaxPayloadBytes,
) -> Result<(), IpcError> {
    let Ok(payload_len) = usize::try_from(header.payload_len) else {
        return Err(IpcError::PayloadLengthOutOfRange {
            actual: header.payload_len,
        });
    };
    if payload_len > max_payload.get() {
        return Err(IpcError::PayloadTooLarge {
            actual: payload_len,
            limit: max_payload.get(),
        });
    }
    Ok(())
}