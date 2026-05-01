//! IPC frame encoding and decoding utilities.

use std::io::{Cursor, Read, Write};

use crate::{IPC_HEADER_LEN, IPC_MAGIC, IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes};

/// Encodes a complete IPC frame (header + payload) into a byte vector.
pub fn encode_frame(
    command: IpcCommand,
    flags: u16,
    correlation: u64,
    payload: &[u8],
) -> Result<Vec<u8>, IpcError> {
    let header = IpcFrameHeader::new(command, flags, correlation, payload_len_u32(payload.len())?);
    let header_bytes = header.encode()?;
    let mut frame = Vec::with_capacity(IPC_HEADER_LEN.checked_add(payload.len()).ok_or(
        IpcError::PayloadTooLarge {
            actual: payload.len(),
            limit: usize::MAX,
        },
    )?);
    frame.extend_from_slice(&header_bytes);
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Decodes only the fixed header from a byte slice.
pub fn decode_frame_header(bytes: &[u8; IPC_HEADER_LEN]) -> Result<IpcFrameHeader, IpcError> {
    IpcFrameHeader::decode(bytes, MaxPayloadBytes::DEFAULT)
}

/// Decodes a header from arbitrary bytes for fuzzing and socket ingress.
pub fn fuzz_decode_frame(bytes: &[u8], max_payload: MaxPayloadBytes) -> Result<(), IpcError> {
    validate_frame_magic(bytes)?;
    if bytes.len() < IPC_HEADER_LEN {
        return Err(IpcError::HeaderDecodeFailed);
    }
    let header_bytes = fixed_header_from_slice(bytes)?;
    let header = IpcFrameHeader::decode(&header_bytes, max_payload)?;
    validate_frame_bounds(&header, max_payload)?;
    let payload_len = payload_len_usize(header.payload_len)?;
    let total_len = IPC_HEADER_LEN
        .checked_add(payload_len)
        .ok_or(IpcError::PayloadTooLarge {
            actual: payload_len,
            limit: max_payload.get(),
        })?;
    if bytes.len() < total_len {
        return Err(IpcError::PayloadLengthMismatch {
            header: total_len,
            actual: bytes.len(),
        });
    }
    let payload = bytes
        .get(IPC_HEADER_LEN..total_len)
        .ok_or(IpcError::HeaderDecodeFailed)?;
    decode_frame_payload(&header, payload).map(|_| ())
}

/// Decodes the payload from postcard bytes after the header has been validated.
pub fn decode_frame_payload(
    header: &IpcFrameHeader,
    payload: &[u8],
) -> Result<crate::IpcPayload, IpcError> {
    let expected_len = payload_len_usize(header.payload_len)?;
    if payload.len() != expected_len {
        return Err(IpcError::PayloadLengthMismatch {
            header: expected_len,
            actual: payload.len(),
        });
    }
    postcard::from_bytes(payload).map_err(|_| IpcError::PayloadDecodeFailed)
}

/// Validates that the magic bytes are correct before any allocation occurs.
pub fn validate_frame_magic(bytes: &[u8]) -> Result<(), IpcError> {
    if bytes.len() < 4 {
        return Err(IpcError::HeaderDecodeFailed);
    }
    let mut cursor = Cursor::new(bytes);
    let mut magic_bytes = [0u8; 4];
    cursor
        .read_exact(&mut magic_bytes)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    let magic = u32::from_le_bytes(magic_bytes);
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
    let payload_len = payload_len_usize(header.payload_len)?;
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
    read_frame_header_bounded(reader, MaxPayloadBytes::DEFAULT)
}

/// Reads a frame header with an explicit payload bound.
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
    read_frame_payload_bounded(reader, header, MaxPayloadBytes::DEFAULT)
}

/// Reads payload bytes only after the declared length is checked against a bound.
pub fn read_frame_payload_bounded<R: Read>(
    reader: &mut R,
    header: &IpcFrameHeader,
    max_payload: MaxPayloadBytes,
) -> Result<Vec<u8>, IpcError> {
    validate_frame_bounds(header, max_payload)?;
    let payload_len = payload_len_usize(header.payload_len)?;
    let mut payload = vec![0u8; payload_len];
    reader
        .read_exact(&mut payload)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    Ok(payload)
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

fn payload_len_u32(len: usize) -> Result<u32, IpcError> {
    u32::try_from(len).map_err(|_| IpcError::PayloadTooLarge {
        actual: len,
        limit: usize::MAX,
    })
}

fn payload_len_usize(len: u32) -> Result<usize, IpcError> {
    usize::try_from(len).map_err(|_| IpcError::PayloadLengthOutOfRange { actual: len })
}

fn fixed_header_from_slice(bytes: &[u8]) -> Result<[u8; IPC_HEADER_LEN], IpcError> {
    let header = bytes
        .get(..IPC_HEADER_LEN)
        .ok_or(IpcError::HeaderDecodeFailed)?;
    <[u8; IPC_HEADER_LEN]>::try_from(header).map_err(|_| IpcError::HeaderDecodeFailed)
}
