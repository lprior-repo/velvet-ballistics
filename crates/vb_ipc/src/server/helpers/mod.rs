#![forbid(unsafe_code)]
//! Buffer and frame helper functions.

use super::IpcResponse;
use super::error::IpcServerError;
use crate::IpcError;
use crate::IpcFrameHeader;
use crate::{IPC_HEADER_LEN, IPC_MAGIC, MaxPayloadBytes};
use mio::Registry;
use mio::Token;
use mio::net::UnixStream;
#[cfg(test)]
use std::io::Write;

/// Maximum bytes to read while still in AwaitingMagic state.
pub const AWAITING_MAGIC_MAX_BYTES: usize = 4;

/// Magic validation state for IPC frame header parsing.
///
/// Tracks whether we have validated the magic bytes at the start of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicValidationState {
    /// Have not yet validated magic bytes.
    AwaitingMagic,
    /// Magic bytes validated successfully.
    MagicValidated,
}

/// Validates magic bytes early, before allocating a large read buffer.
///
/// Returns `MagicValidated` if the magic matches `IPC_MAGIC`.
/// Returns `AwaitingMagic` if not enough bytes have been collected.
/// Returns an `IpcError::InvalidMagic` if the magic bytes are present but do not match.
pub fn validate_magic_early(bytes: &[u8]) -> Result<MagicValidationState, IpcError> {
    let Some(prefix) = bytes.get(..AWAITING_MAGIC_MAX_BYTES) else {
        return Ok(MagicValidationState::AwaitingMagic);
    };
    let magic_bytes = <[u8; AWAITING_MAGIC_MAX_BYTES]>::try_from(prefix)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    let magic = u32::from_le_bytes(magic_bytes);
    if magic == IPC_MAGIC {
        Ok(MagicValidationState::MagicValidated)
    } else {
        Err(IpcError::InvalidMagic { actual: magic })
    }
}

/// Appends read bytes into the read buffer with bounds checking.
pub fn append_read_bytes(
    read_buffer: &mut Vec<u8>,
    temp_buf: &[u8; 4096],
    bytes_read: usize,
) -> Result<(), IpcServerError> {
    let read_slice = temp_buf
        .get(..bytes_read)
        .ok_or(IpcServerError::FrameInvalid {
            source: IpcError::PayloadLengthMismatch {
                header: 4096,
                actual: bytes_read,
            },
        })?;
    let next_len = read_buffer
        .len()
        .checked_add(read_slice.len())
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    let max_buffer = IPC_HEADER_LEN
        .checked_add(MaxPayloadBytes::DEFAULT.get())
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    if next_len > max_buffer {
        return Err(IpcServerError::ReadBufferTooLarge);
    }
    read_buffer
        .try_reserve(read_slice.len())
        .map_err(|_| IpcServerError::ReadBufferTooLarge)?;
    read_buffer.extend_from_slice(read_slice);
    Ok(())
}

/// Reads the frame header from the read buffer.
pub fn read_buffer_header(read_buffer: &[u8]) -> Result<[u8; IPC_HEADER_LEN], IpcServerError> {
    let header_slice = read_buffer
        .get(..IPC_HEADER_LEN)
        .ok_or(IpcServerError::IncompleteFrame)?;
    <[u8; IPC_HEADER_LEN]>::try_from(header_slice).map_err(|_| IpcServerError::IncompleteFrame)
}

/// Computes total frame length from header.
pub fn frame_total_len(header: &IpcFrameHeader) -> Result<usize, IpcServerError> {
    let payload_len =
        usize::try_from(header.payload_len).map_err(|_| IpcServerError::FrameInvalid {
            source: IpcError::PayloadLengthOutOfRange {
                actual: header.payload_len,
            },
        })?;
    let total_len = IPC_HEADER_LEN
        .checked_add(payload_len)
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    Ok(total_len)
}

/// Extracts payload bytes from the read buffer.
pub fn extract_payload(
    read_buffer: &mut Vec<u8>,
    total_len: usize,
) -> Result<Vec<u8>, IpcServerError> {
    if read_buffer.len() < total_len {
        return Err(IpcServerError::IncompleteFrame);
    }
    // Keep unread bytes in the connection buffer and return only the consumed payload.
    let remaining = read_buffer.split_off(total_len);
    let mut frame = std::mem::replace(read_buffer, remaining);
    Ok(frame.split_off(IPC_HEADER_LEN))
}

/// Creates a frame error response.
pub fn frame_error_response(error: IpcError) -> IpcResponse {
    IpcResponse::FrameError {
        message: error.to_string(),
    }
}

/// Borrows the workflow resolver from an optional mutable reference.
pub fn borrow_workflow_resolver<'a>(
    resolver: &'a mut Option<&mut dyn crate::server::WorkflowResolver>,
) -> Option<&'a mut dyn crate::server::WorkflowResolver> {
    match resolver {
        Some(inner) => Some(&mut **inner),
        None => None,
    }
}

#[cfg(test)]
pub(crate) mod test_hooks {
    use std::cell::Cell;
    thread_local! {
        pub(crate) static FORCE_POSTCARD_FAIL: Cell<bool> = const { Cell::new(false) };
        pub(crate) static FORCE_HEADER_ENCODE_FAIL: Cell<bool> = const { Cell::new(false) };
        pub(crate) static FORCE_FLUSH_FAIL: Cell<bool> = const { Cell::new(false) };
    }
}

/// Sends a response frame to the client.
pub fn send_response(
    stream: &mut UnixStream,
    write_buffer: &mut Vec<u8>,
    registry: &Registry,
    token: Token,
    request_header: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<(), IpcServerError> {
    if !write_buffer.is_empty() {
        return Err(IpcServerError::ResponsePending);
    }
    super::response_io::prepare_response(write_buffer, request_header, response)?;
    super::response_io::write_response_once(stream, write_buffer)?;
    super::response_io::finish_write(stream, write_buffer, registry, token)
}

#[cfg(test)]
pub(crate) fn append_read_bytes_checked_add(a: usize, b: usize) -> Result<usize, IpcServerError> {
    a.checked_add(b).ok_or(IpcServerError::ReadBufferTooLarge)
}

#[cfg(test)]
pub(crate) fn frame_total_len_checked_add(
    header_len: usize,
    payload_len: usize,
) -> Result<usize, IpcServerError> {
    header_len
        .checked_add(payload_len)
        .ok_or(IpcServerError::ReadBufferTooLarge)
}

#[cfg(test)]
mod tests;
