//! IPC frame validation functions.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

use crate::{IpcError, MaxPayloadBytes};

use super::frame_codec::decode_frame_header;
use super::frame_types::{IpcFrameHeader, IPC_HEADER_LEN, IPC_MAGIC};

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
pub fn validate_frame_bounds(header: &IpcFrameHeader, max_payload: MaxPayloadBytes) -> Result<(), IpcError> {
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

/// Reads a frame header from a Read trait object.
pub fn read_frame_header<R: Read>(reader: &mut R) -> Result<IpcFrameHeader, IpcError> {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
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
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
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
