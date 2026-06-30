#![forbid(unsafe_code)]
//! IPC frame encoding and decoding utilities.

use crate::{IPC_HEADER_LEN, IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes};

/// Encodes a complete IPC frame (header + payload) into a byte vector.
pub fn encode_frame(
    command: IpcCommand,
    flags: u16,
    correlation: u64,
    payload: &[u8],
) -> Result<Vec<u8>, IpcError> {
    let header =
        IpcFrameHeader::new(command, flags, correlation, payload_len_u32(payload.len())?);
    let header_bytes = header.encode()?;
    let mut frame = Vec::with_capacity(
        IPC_HEADER_LEN
            .checked_add(payload.len())
            .ok_or(IpcError::PayloadTooLarge {
                actual: payload.len(),
                limit: usize::MAX,
            })?,
    );
    frame.extend_from_slice(&header_bytes);
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Decodes only the fixed header from a byte slice.
pub fn decode_frame_header(bytes: &[u8; IPC_HEADER_LEN]) -> Result<IpcFrameHeader, IpcError> {
    IpcFrameHeader::decode(bytes, MaxPayloadBytes::DEFAULT)
}

/// Decodes the payload from postcard bytes after the header has been validated.
pub fn decode_frame_payload(
    header: &IpcFrameHeader,
    payload: &[u8],
) -> Result<crate::IpcPayload, IpcError> {
    let Ok(expected_len) = usize::try_from(header.payload_len) else {
        return Err(IpcError::PayloadLengthOutOfRange {
            actual: header.payload_len,
        });
    };
    if payload.len() != expected_len {
        return Err(IpcError::PayloadLengthMismatch {
            header: expected_len,
            actual: payload.len(),
        });
    }
    postcard::from_bytes(payload).map_err(|_| IpcError::PayloadDecodeFailed)
}

fn payload_len_u32(len: usize) -> Result<u32, IpcError> {
    u32::try_from(len).map_err(|_| IpcError::PayloadTooLarge {
        actual: len,
        limit: usize::MAX,
    })
}