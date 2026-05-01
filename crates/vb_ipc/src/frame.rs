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

/// Decodes the payload from postcard bytes after the header has been validated.
pub fn decode_frame_payload(
    header: &IpcFrameHeader,
    payload: &[u8],
) -> Result<crate::IpcPayload, IpcError> {
    let expected_len = match usize::try_from(header.payload_len) {
        Ok(len) => len,
        Err(_) => {
            return Err(IpcError::PayloadLengthOutOfRange {
                actual: header.payload_len,
            });
        }
    };
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
    let payload_len = match usize::try_from(header.payload_len) {
        Ok(len) => len,
        Err(_) => {
            return Err(IpcError::PayloadLengthOutOfRange {
                actual: header.payload_len,
            });
        }
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

/// Reads the payload bytes following a validated header from a Read trait object.
pub fn read_frame_payload<R: Read>(
    reader: &mut R,
    header: &IpcFrameHeader,
) -> Result<Vec<u8>, IpcError> {
    let payload_len = match usize::try_from(header.payload_len) {
        Ok(len) => len,
        Err(_) => {
            return Err(IpcError::PayloadLengthOutOfRange {
                actual: header.payload_len,
            });
        }
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_frame_produces_valid_header_and_payload() {
        let payload = b"test-data";
        let result = encode_frame(IpcCommand::Health, 0, 99, payload);
        assert!(result.is_ok(), "encode_frame should succeed");
        let Ok(frame) = result else {
            return;
        };

        assert!(
            frame.len() > IPC_HEADER_LEN,
            "frame should contain header plus payload"
        );
        let header_slice = match frame.get(..IPC_HEADER_LEN) {
            Some(s) => s,
            None => return,
        };
        let header_bytes: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
            Ok(h) => h,
            Err(_) => return,
        };
        let header_result = decode_frame_header(&header_bytes);
        assert!(header_result.is_ok(), "header should decode");
        let Ok(header) = header_result else {
            return;
        };
        assert_eq!(header.command, IpcCommand::Health);
        assert_eq!(header.correlation, 99);
        let payload_len = match usize::try_from(header.payload_len) {
            Ok(v) => v,
            Err(_) => return,
        };
        assert_eq!(payload_len, payload.len());
        assert_eq!(frame.get(IPC_HEADER_LEN..), Some(payload.as_slice()));
    }

    #[test]
    fn fuzz_decode_frame_rejects_short_input() {
        let short: [u8; IPC_HEADER_LEN] = [0u8; IPC_HEADER_LEN];
        let result = decode_frame_header(&short);
        assert!(result.is_err(), "zero-filled header should fail validation");
    }

    #[test]
    fn fuzz_decode_frame_rejects_bad_magic() {
        let bad_magic: u32 = 0xDEAD_BEEF;
        let mut header_bytes = [0u8; IPC_HEADER_LEN];
        header_bytes[..4].copy_from_slice(&bad_magic.to_le_bytes());
        let result = decode_frame_header(&header_bytes);

        assert_eq!(result, Err(IpcError::InvalidMagic { actual: bad_magic }));
    }

    #[test]
    fn fuzz_decode_frame_rejects_oversized_payload() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 9999);
        let encoded = header.encode();
        assert!(encoded.is_ok(), "header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
        let result = IpcFrameHeader::decode(&encoded, tiny_max);

        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: 9999,
                limit: tiny_max.get(),
            })
        );
    }

    #[test]
    fn validate_frame_magic_rejects_wrong_magic() {
        let wrong_magic: u32 = 0x1234_5678;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&wrong_magic.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 20]);

        assert_eq!(
            validate_frame_magic(&bytes),
            Err(IpcError::InvalidMagic {
                actual: wrong_magic,
            })
        );
    }

    #[test]
    fn validate_frame_bounds_rejects_at_boundary() {
        let max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 2);

        assert_eq!(
            validate_frame_bounds(&header, max),
            Err(IpcError::PayloadTooLarge {
                actual: 2,
                limit: max.get(),
            })
        );
    }
}
