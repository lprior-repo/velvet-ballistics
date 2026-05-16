#![forbid(unsafe_code)]
//! IPC frame I/O utilities for reading and writing frames from streams.

use std::io::{Read, Write};

use crate::{IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes};

use super::codec::{decode_frame_header, encode_frame};
use super::validate::validate_frame_bounds;

/// Reads a frame header from a Read trait object.
pub fn read_frame_header<R: Read>(reader: &mut R) -> Result<IpcFrameHeader, IpcError> {
    let mut header_bytes = [0u8; crate::IPC_HEADER_LEN];
    reader
        .read_exact(&mut header_bytes)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    decode_frame_header(&header_bytes)
}

/// Reads a frame header and validates it against an explicit payload limit.
pub fn read_frame_header_bounded<R: Read>(
    reader: &mut R,
    max_payload: MaxPayloadBytes,
) -> Result<IpcFrameHeader, IpcError> {
    let mut header_bytes = [0u8; crate::IPC_HEADER_LEN];
    reader
        .read_exact(&mut header_bytes)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    IpcFrameHeader::decode(&header_bytes, max_payload)
}

/// Reads the payload bytes following a validated header from a Read trait object.
pub fn read_frame_payload<R: Read>(
    reader: &mut R,
    header: &IpcFrameHeader,
) -> Result<Vec<u8>, IpcError> {
    let Ok(payload_len) = usize::try_from(header.payload_len) else {
        return Err(IpcError::PayloadLengthOutOfRange {
            actual: header.payload_len,
        });
    };
    let mut payload = vec![0u8; payload_len];
    reader
        .read_exact(&mut payload)
        .map_err(|_| IpcError::PayloadDecodeFailed)?;
    Ok(payload)
}

/// Reads payload bytes after enforcing an explicit payload limit.
pub fn read_frame_payload_bounded<R: Read>(
    reader: &mut R,
    header: &IpcFrameHeader,
    max_payload: MaxPayloadBytes,
) -> Result<Vec<u8>, IpcError> {
    validate_frame_bounds(header, max_payload)?;
    read_frame_payload(reader, header)
}

/// Writes a complete frame to a Write trait object.
pub fn write_frame<W: Write>(
    writer: &mut W,
    command: IpcCommand,
    flags: u16,
    correlation: u64,
    payload: &[u8],
) -> Result<(), IpcError> {
    let frame = encode_frame(command, flags, correlation, payload)?;
    writer
        .write_all(&frame)
        .map_err(|_| IpcError::HeaderEncodeFailed)?;
    Ok(())
}
