//! Read buffer management for IPC frame parsing.
//!
//! Functions for appending read bytes, reading frame headers,
//! computing total frame length, and extracting payloads.

use crate::IpcError;
use crate::IpcFrameHeader;
use crate::{IPC_HEADER_LEN, MaxPayloadBytes};
use super::super::error::IpcServerError as ServerError;

/// Appends read bytes into the read buffer with bounds checking.
pub(crate) fn append_read_bytes(
    read_buffer: &mut Vec<u8>,
    temp_buf: &[u8; 4096],
    bytes_read: usize,
) -> Result<(), ServerError> {
    let read_slice = temp_buf
        .get(..bytes_read)
        .ok_or(ServerError::FrameInvalid {
            source: IpcError::PayloadLengthMismatch {
                header: 4096,
                actual: bytes_read,
            },
        })?;
    let next_len = read_buffer
        .len()
        .checked_add(read_slice.len())
        .ok_or(ServerError::ReadBufferTooLarge)?;
    let max_buffer = IPC_HEADER_LEN
        .checked_add(MaxPayloadBytes::DEFAULT.get())
        .ok_or(ServerError::ReadBufferTooLarge)?;
    if next_len > max_buffer {
        return Err(ServerError::ReadBufferTooLarge);
    }
    read_buffer.extend_from_slice(read_slice);
    Ok(())
}

/// Reads the frame header from the read buffer.
pub(crate) fn read_buffer_header(
    read_buffer: &[u8],
) -> Result<[u8; IPC_HEADER_LEN], ServerError> {
    let header_slice = read_buffer
        .get(..IPC_HEADER_LEN)
        .ok_or(ServerError::IncompleteFrame)?;
    <[u8; IPC_HEADER_LEN]>::try_from(header_slice).map_err(|_| ServerError::IncompleteFrame)
}

/// Computes total frame length from header.
pub(crate) fn frame_total_len(header: &IpcFrameHeader) -> Result<usize, ServerError> {
    let payload_len =
        usize::try_from(header.payload_len).map_err(|_| ServerError::FrameInvalid {
            source: IpcError::PayloadLengthOutOfRange {
                actual: header.payload_len,
            },
        })?;
    let total_len = IPC_HEADER_LEN
        .checked_add(payload_len)
        .ok_or(ServerError::ReadBufferTooLarge)?;
    Ok(total_len)
}

/// Extracts payload bytes from the read buffer.
pub(crate) fn extract_payload(
    read_buffer: &mut Vec<u8>,
    total_len: usize,
) -> Result<Vec<u8>, ServerError> {
    if read_buffer.len() < total_len {
        return Err(ServerError::IncompleteFrame);
    }
    // Keep unread bytes in the connection buffer and return only the consumed payload.
    let remaining = read_buffer.split_off(total_len);
    let mut frame = std::mem::replace(read_buffer, remaining);
    Ok(frame.split_off(IPC_HEADER_LEN))
}

/// Re-export `IpcServerError` for test assertions in helpers/mod.rs.
pub(crate) use super::super::error::IpcServerError;

/// Test-only helper: checked_add shim for `append_read_bytes` overflow tests.
#[cfg(test)]
pub(crate) fn append_read_bytes_checked_add(
    a: usize,
    b: usize,
) -> Result<usize, ServerError> {
    a.checked_add(b).ok_or(ServerError::ReadBufferTooLarge)
}

/// Test-only helper: checked_add shim for `frame_total_len` overflow tests.
#[cfg(test)]
pub(crate) fn frame_total_len_checked_add(
    header_len: usize,
    payload_len: usize,
) -> Result<usize, ServerError> {
    header_len
        .checked_add(payload_len)
        .ok_or(ServerError::ReadBufferTooLarge)
}
