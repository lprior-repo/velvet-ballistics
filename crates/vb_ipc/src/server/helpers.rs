//! Buffer and frame helper functions.

use super::IpcResponse;
use super::error::IpcServerError;
use crate::IpcError;
use crate::IpcFrameHeader;
use crate::{IPC_HEADER_LEN, MaxPayloadBytes};
use mio::Registry;
use mio::Token;
use mio::net::UnixStream;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IpcCommand;

    // ── append_read_bytes tests ──

    #[test]
    fn append_read_bytes_appends_data_to_empty_buffer() {
        let mut read_buffer = Vec::new();
        let temp_buf = [0xAB_u8; 4096];
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 10);
        assert!(result.is_ok(), "appending 10 bytes should succeed");
        assert_eq!(read_buffer.len(), 10);
        assert_eq!(read_buffer.as_slice(), &[0xAB; 10]);
    }

    #[test]
    fn append_read_bytes_appends_to_existing_buffer() {
        let mut read_buffer = vec![1, 2, 3];
        let temp_buf = [4u8; 4096];
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 3);
        assert!(result.is_ok());
        assert_eq!(read_buffer.len(), 6);
        assert_eq!(read_buffer.as_slice(), &[1, 2, 3, 4, 4, 4]);
    }

    #[test]
    fn append_read_bytes_with_zero_bytes_read() {
        let mut read_buffer = Vec::new();
        let temp_buf = [0u8; 4096];
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 0);
        assert!(result.is_ok(), "zero bytes should succeed");
        assert!(read_buffer.is_empty());
    }

    #[test]
    fn append_read_bytes_rejects_bytes_read_exceeding_temp_buf() {
        let mut read_buffer = Vec::new();
        let temp_buf = [0u8; 4096];
        // bytes_read > 4096 is impossible in practice but tests the guard
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 5000);
        assert!(result.is_err(), "bytes_read > temp_buf size should fail");
    }

    // ── read_buffer_header tests ──

    #[test]
    fn read_buffer_header_returns_incomplete_frame_for_short_buffer() {
        let short_buf = vec![0u8; 10];
        let result = read_buffer_header(&short_buf);
        assert!(result.is_err(), "short buffer should fail");
        let Err(err) = result else { return };
        let msg = err.to_string();
        assert!(
            msg.contains("incomplete"),
            "expected 'incomplete' in '{msg}'"
        );
    }

    #[test]
    fn read_buffer_header_returns_incomplete_frame_for_empty_buffer() {
        let empty_buf: Vec<u8> = Vec::new();
        let result = read_buffer_header(&empty_buf);
        assert!(result.is_err());
    }

    #[test]
    fn read_buffer_header_succeeds_with_exact_header_length() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let encoded = header.encode();
        assert!(encoded.is_ok(), "header should encode");
        let Ok(encoded) = encoded else { return };
        let buf = encoded.to_vec();
        let result = read_buffer_header(&buf);
        assert!(result.is_ok(), "exact header length should succeed");
    }

    #[test]
    fn read_buffer_header_succeeds_with_extra_bytes() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let encoded = header.encode();
        assert!(encoded.is_ok());
        let Ok(encoded) = encoded else { return };
        let mut buf = encoded.to_vec();
        buf.extend_from_slice(&[0xFF; 100]); // extra payload bytes
        let result = read_buffer_header(&buf);
        assert!(result.is_ok(), "extra bytes after header should still succeed");
    }

    // ── frame_total_len tests ──

    #[test]
    fn frame_total_len_header_only_zero_payload() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let result = frame_total_len(&header);
        assert!(result.is_ok());
        let Ok(val) = result else { return };
        assert_eq!(val, IPC_HEADER_LEN);
    }

    #[test]
    fn frame_total_len_with_payload() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
        let result = frame_total_len(&header);
        assert!(result.is_ok());
        let Ok(val) = result else { return };
        assert_eq!(val, IPC_HEADER_LEN + 100);
    }

    #[test]
    fn frame_total_len_with_max_reasonable_payload() {
        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, 1000);
        let result = frame_total_len(&header);
        assert!(result.is_ok());
        let Ok(val) = result else { return };
        assert_eq!(val, IPC_HEADER_LEN + 1000);
    }

    // ── extract_payload tests ──

    #[test]
    fn extract_payload_returns_incomplete_when_buffer_too_short() {
        let mut read_buffer = vec![0u8; 10];
        let result = extract_payload(&mut read_buffer, 50);
        assert!(result.is_err());
    }

    #[test]
    fn extract_payload_extracts_header_plus_payload() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let encoded = header.encode();
        assert!(encoded.is_ok());
        let Ok(encoded) = encoded else { return };
        let mut read_buffer = encoded.to_vec();
        read_buffer.extend_from_slice(b"test");
        let total_len = IPC_HEADER_LEN + 4;

        let result = extract_payload(&mut read_buffer, total_len);
        assert!(result.is_ok(), "extract should succeed");
        let Ok(payload) = result else { return };
        assert_eq!(payload.as_slice(), b"test");
    }

    #[test]
    fn extract_payload_preserves_remaining_bytes_in_buffer() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let encoded = header.encode();
        assert!(encoded.is_ok());
        let Ok(encoded) = encoded else { return };
        let mut read_buffer = encoded.to_vec();
        read_buffer.extend_from_slice(b"test");
        read_buffer.extend_from_slice(b"extra");
        let total_len = IPC_HEADER_LEN + 4;

        let result = extract_payload(&mut read_buffer, total_len);
        assert!(result.is_ok());
        assert_eq!(read_buffer.as_slice(), b"extra", "remaining bytes should stay in buffer");
    }

    #[test]
    fn extract_payload_returns_empty_for_zero_payload_len() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let encoded = header.encode();
        assert!(encoded.is_ok());
        let Ok(encoded) = encoded else { return };
        let mut read_buffer = encoded.to_vec();
        let result = extract_payload(&mut read_buffer, IPC_HEADER_LEN);
        assert!(result.is_ok());
        let Ok(payload) = result else { return };
        assert!(payload.is_empty());
    }

    // ── frame_error_response tests ──

    #[test]
    fn frame_error_response_produces_frame_error_variant() {
        let err = IpcError::InvalidMagic { actual: 0xDEAD };
        let response = frame_error_response(err);
        let IpcResponse::FrameError { message } = response else {
            return;
        };
        assert!(message.contains("magic"), "expected 'magic' in '{message}'");
    }

    #[test]
    fn frame_error_response_includes_display_of_source() {
        let err = IpcError::PayloadTooLarge {
            actual: 999,
            limit: 10,
        };
        let response = frame_error_response(err);
        let IpcResponse::FrameError { message } = response else {
            return;
        };
        assert!(message.contains("999"), "expected actual value in '{message}'");
        assert!(message.contains("10"), "expected limit value in '{message}'");
    }

    // ── borrow_workflow_resolver tests ──

    #[test]
    fn borrow_workflow_resolver_returns_none_for_none_outer() {
        let mut outer: Option<&mut dyn crate::server::WorkflowResolver> = None;
        let result = borrow_workflow_resolver(&mut outer);
        assert!(result.is_none());
    }

    #[test]
    fn borrow_workflow_resolver_returns_some_for_some_outer() {
        struct DummyResolver;
        impl crate::server::WorkflowResolver for DummyResolver {
            fn resolve_workflow(
                &mut self,
                _digest: vb_core::WorkflowDigest,
            ) -> Result<vb_core::workflow::CompiledWorkflow, crate::server::WorkflowResolutionError>
            {
                Err(crate::server::WorkflowResolutionError::NotFound)
            }
        }
        let mut resolver = DummyResolver;
        let mut outer: Option<&mut dyn crate::server::WorkflowResolver> = Some(&mut resolver);
        let result = borrow_workflow_resolver(&mut outer);
        assert!(result.is_some());
    }
}
