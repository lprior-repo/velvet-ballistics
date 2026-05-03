//! Buffer and frame helper functions.

use super::error::IpcServerError;
use crate::IpcFrameHeader;
use crate::IpcError;
use crate::{MaxPayloadBytes, IPC_HEADER_LEN};
use super::IpcResponse;
use mio::net::UnixStream;
use mio::Registry;
use mio::Token;
use std::io::Write;

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

/// Sends a response frame to the client.
pub fn send_response(
    stream: &mut UnixStream,
    write_buffer: &mut Vec<u8>,
    registry: &Registry,
    token: Token,
    request_header: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<(), IpcServerError> {
    let payload_bytes =
        postcard::to_allocvec(response).map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    let payload_len =
        u32::try_from(payload_bytes.len()).map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    let header = IpcFrameHeader::new(
        request_header.command,
        0,
        request_header.correlation,
        payload_len,
    );
    let header_bytes = header
        .encode()
        .map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    write_buffer.clear();
    write_buffer.extend_from_slice(&header_bytes);
    write_buffer.extend_from_slice(&payload_bytes);

    let written = match stream.write(write_buffer) {
        Ok(count) => count,
        Err(ref source) if source.kind() == std::io::ErrorKind::WouldBlock => 0,
        Err(source) => return Err(IpcServerError::ResponseWriteFailed { source }),
    };
    if written > 0 {
        drop(write_buffer.drain(..written));
    }
    if write_buffer.is_empty() {
        match stream.flush() {
            Ok(()) => {}
            Err(ref source) if source.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(source) => return Err(IpcServerError::ResponseWriteFailed { source }),
        }
    } else {
        registry
            .reregister(
                stream,
                token,
                mio::Interest::READABLE | mio::Interest::WRITABLE,
            )
            .map_err(|source| IpcServerError::PollFailed { source })?;
    }

    Ok(())
}
