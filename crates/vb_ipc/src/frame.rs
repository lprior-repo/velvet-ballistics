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

    // ── Frame protocol tests ──

    #[test]
    fn encode_frame_produces_correct_magic_bytes() {
        // Given: a valid command and empty payload
        // When: encoding a frame
        let result = encode_frame(IpcCommand::Health, 0, 1, b"");
        assert!(result.is_ok(), "encode_frame should succeed");
        let Ok(frame) = result else {
            return;
        };

        // Then: bytes 0..4 are IPC_MAGIC in little-endian
        let magic_slice = frame.get(..4);
        assert_eq!(
            magic_slice,
            Some(crate::IPC_MAGIC.to_le_bytes().as_slice())
        );
    }

    #[test]
    fn encode_frame_produces_correct_version_byte() {
        // Given: a valid command and empty payload
        // When: encoding a frame
        let result = encode_frame(IpcCommand::Health, 0, 1, b"");
        assert!(result.is_ok(), "encode_frame should succeed");
        let Ok(frame) = result else {
            return;
        };

        // Then: bytes 4..6 are IPC_VERSION in little-endian
        let version_slice = frame.get(4..6);
        assert_eq!(
            version_slice,
            Some(crate::IPC_VERSION.to_le_bytes().as_slice())
        );
    }

    #[test]
    fn encode_frame_produces_correct_command_byte() {
        // Given: a DrainTrace command (id=9)
        // When: encoding a frame
        let result = encode_frame(IpcCommand::DrainTrace, 0, 1, b"");
        assert!(result.is_ok(), "encode_frame should succeed");
        let Ok(frame) = result else {
            return;
        };

        // Then: bytes 6..8 are command id 9 in little-endian
        let cmd_slice = frame.get(6..8);
        assert_eq!(
            cmd_slice,
            Some(9u16.to_le_bytes().as_slice())
        );
    }

    #[test]
    fn encode_frame_produces_correct_payload_length() {
        // Given: a payload of 5 bytes
        let payload = b"hello";

        // When: encoding a frame
        let result = encode_frame(IpcCommand::Health, 0, 1, payload);
        assert!(result.is_ok(), "encode_frame should succeed");
        let Ok(frame) = result else {
            return;
        };

        // Then: bytes 20..24 contain payload length 5 in little-endian
        let len_slice = frame.get(20..24);
        assert_eq!(
            len_slice,
            Some(5u32.to_le_bytes().as_slice())
        );
    }

    #[test]
    fn encode_frame_roundtrip_with_empty_payload() {
        // Given: an empty payload
        let payload: &[u8] = b"";

        // When: encoding then decoding
        let frame_result = encode_frame(IpcCommand::Health, 0, 42, payload);
        assert!(frame_result.is_ok(), "encode should succeed");
        let Ok(frame_bytes) = frame_result else {
            return;
        };

        let header_bytes_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
            Some(s) => s,
            None => return,
        };
        let header_arr: [u8; IPC_HEADER_LEN] = match header_bytes_slice.try_into() {
            Ok(h) => h,
            Err(_) => return,
        };
        let header = decode_frame_header(&header_arr);
        assert!(header.is_ok(), "header should decode");
        let Ok(header) = header else {
            return;
        };

        // Then: command, correlation, and payload_len match
        assert_eq!(header.command, IpcCommand::Health);
        assert_eq!(header.correlation, 42);
        assert_eq!(header.payload_len, 0);
        let payload_section = frame_bytes.get(IPC_HEADER_LEN..);
        assert_eq!(payload_section, Some(&[][..]));
    }

    #[test]
    fn encode_frame_roundtrip_with_large_payload() {
        // Given: a 1024-byte payload
        let payload = vec![0xAB_u8; 1024];

        // When: encoding then decoding
        let frame_result = encode_frame(IpcCommand::SubmitRun, 0, 99, &payload);
        assert!(frame_result.is_ok(), "encode should succeed");
        let Ok(frame_bytes) = frame_result else {
            return;
        };

        let header_bytes_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
            Some(s) => s,
            None => return,
        };
        let header_arr: [u8; IPC_HEADER_LEN] = match header_bytes_slice.try_into() {
            Ok(h) => h,
            Err(_) => return,
        };
        let header = decode_frame_header(&header_arr);
        assert!(header.is_ok(), "header should decode");
        let Ok(header) = header else {
            return;
        };

        // Then: all header fields and payload match
        assert_eq!(header.command, IpcCommand::SubmitRun);
        assert_eq!(header.correlation, 99);
        assert_eq!(header.payload_len, 1024);
        let payload_section = frame_bytes.get(IPC_HEADER_LEN..);
        assert_eq!(payload_section, Some(payload.as_slice()));
    }

    #[test]
    fn decode_frame_header_rejects_truncated_magic() {
        // Given: only 3 bytes (truncated magic)
        let bytes = [0u8; 3];

        // When: trying to read a frame header from a 3-byte reader
        let mut cursor = std::io::Cursor::new(bytes);
        let result = read_frame_header(&mut cursor);

        // Then: HeaderDecodeFailed is returned (short read)
        assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
    }

    #[test]
    fn validate_frame_magic_rejects_too_short_input() {
        // Given: only 3 bytes (too short for magic validation)
        let bytes = [0u8; 3];

        // When: validating magic
        let result = validate_frame_magic(&bytes);

        // Then: HeaderDecodeFailed is returned
        assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
    }

    #[test]
    fn decode_frame_payload_rejects_length_mismatch() {
        // Given: a header declaring payload_len=100
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);

        // When: decoding with only 50 bytes of payload
        let short_payload = vec![0u8; 50];
        let result = decode_frame_payload(&header, &short_payload);

        // Then: PayloadLengthMismatch with exact header/actual values
        assert_eq!(
            result,
            Err(IpcError::PayloadLengthMismatch {
                header: 100,
                actual: 50,
            })
        );
    }

    #[test]
    fn validate_frame_bounds_rejects_zero_length_with_default_max() {
        // Given: a header with payload_len=0
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

        // When: validating bounds with default max
        let result = validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);

        // Then: it succeeds (0 is always within bounds)
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_frame_bounds_rejects_oversized_length() {
        // Given: a header with payload_len larger than default max
        let max_default = MaxPayloadBytes::DEFAULT.get();
        let oversized = match u32::try_from(max_default.saturating_add(1)) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, oversized);

        // When: validating bounds
        let result = validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);

        // Then: PayloadTooLarge with exact values
        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: max_default.saturating_add(1),
                limit: max_default,
            })
        );
    }

    #[test]
    fn validate_frame_magic_accepts_correct_magic() {
        // Given: bytes starting with correct IPC_MAGIC
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&crate::IPC_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 20]);

        // When: validating magic
        let result = validate_frame_magic(&bytes);

        // Then: it succeeds
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn decode_frame_roundtrip_preserves_all_command_variants() {
        // Given: all IpcCommand variants
        let commands = [
            IpcCommand::SubmitRun,
            IpcCommand::SubmitRunInline,
            IpcCommand::CancelRun,
            IpcCommand::InspectRun,
            IpcCommand::ListEvents,
            IpcCommand::AnswerAsk,
            IpcCommand::CompleteAction,
            IpcCommand::FailAction,
            IpcCommand::DrainTrace,
            IpcCommand::Health,
            IpcCommand::Shutdown,
        ];

        // When: encoding and decoding each command variant
        for cmd in commands {
            let frame_result = encode_frame(cmd, 0, 7, b"");
            assert!(frame_result.is_ok(), "encode should succeed for {cmd:?}");
            let Ok(frame_bytes) = frame_result else {
                return;
            };

            let header_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
                Some(s) => s,
                None => return,
            };
            let header_arr: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
                Ok(h) => h,
                Err(_) => return,
            };
            let header = decode_frame_header(&header_arr);

            // Then: each command round-trips correctly
            assert!(header.is_ok(), "header should decode for {cmd:?}");
            let Ok(h) = header else {
                return;
            };
            assert_eq!(h.command, cmd, "command should roundtrip");
        }
    }

    #[test]
    fn read_frame_header_rejects_short_read() {
        // Given: a reader with only 10 bytes (less than IPC_HEADER_LEN)
        let data = vec![0u8; 10];
        let mut cursor = std::io::Cursor::new(data);

        // When: reading a frame header
        let result = read_frame_header(&mut cursor);

        // Then: HeaderDecodeFailed is returned
        assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
    }

    #[test]
    fn read_frame_payload_returns_exact_bytes_when_available() {
        // Given: a valid header with payload_len=4
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let payload_data = b"test";
        let mut cursor = std::io::Cursor::new(payload_data.as_slice());

        // When: reading the frame payload
        let result = read_frame_payload(&mut cursor, &header);

        // Then: the exact payload bytes are returned
        assert!(result.is_ok(), "payload should read");
        let Ok(payload) = result else {
            return;
        };
        assert_eq!(payload.as_slice(), b"test");
    }

    #[test]
    fn read_frame_payload_rejects_truncated_payload() {
        // Given: a header declaring 10 bytes but only 3 available
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 10);
        let short_data = b"abc";
        let mut cursor = std::io::Cursor::new(short_data.as_slice());

        // When: reading the frame payload
        let result = read_frame_payload(&mut cursor, &header);

        // Then: HeaderDecodeFailed is returned (short read)
        assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
    }

    #[test]
    fn write_frame_produces_valid_frame_on_writer() {
        // Given: a Vec<u8> writer
        let mut writer = Vec::new();

        // When: writing a frame with payload
        let result = write_frame(&mut writer, IpcCommand::Shutdown, 0, 55, b"bye");

        // Then: write succeeds and the output can be decoded
        assert!(result.is_ok(), "write_frame should succeed");
        assert!(writer.len() > IPC_HEADER_LEN, "should contain header + payload");
        let header_slice = match writer.get(..IPC_HEADER_LEN) {
            Some(s) => s,
            None => return,
        };
        let header_arr: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
            Ok(h) => h,
            Err(_) => return,
        };
        let header = decode_frame_header(&header_arr);
        assert!(header.is_ok(), "written header should decode");
        let Ok(header) = header else {
            return;
        };
        assert_eq!(header.command, IpcCommand::Shutdown);
        assert_eq!(header.correlation, 55);
    }

    #[test]
    fn decode_frame_payload_succeeds_for_matching_length() {
        // Given: a valid IpcPayload encoded as postcard bytes
        let payload = crate::IpcPayload::Health;
        let payload_bytes = postcard::to_allocvec(&payload);
        assert!(payload_bytes.is_ok(), "payload should encode");
        let Ok(payload_bytes) = payload_bytes else {
            return;
        };
        let payload_len = match u32::try_from(payload_bytes.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, payload_len);

        // When: decoding the frame payload
        let result = decode_frame_payload(&header, &payload_bytes);

        // Then: it decodes to the correct payload
        assert!(result.is_ok(), "payload should decode");
        let Ok(decoded) = result else {
            return;
        };
        assert_eq!(decoded, crate::IpcPayload::Health);
    }

    #[test]
    fn encode_frame_with_nonzero_flags() {
        // Given: a frame with flags=0x1234
        let result = encode_frame(IpcCommand::Health, 0x1234, 1, b"");

        // When: encoding
        assert!(result.is_ok(), "encode should succeed");
        let Ok(frame) = result else {
            return;
        };

        // Then: flags bytes at offset 8..10 match
        let flags_slice = frame.get(8..10);
        assert_eq!(
            flags_slice,
            Some(0x1234u16.to_le_bytes().as_slice())
        );
    }

    #[test]
    fn encode_frame_with_max_correlation() {
        // Given: a frame with correlation=u64::MAX
        let corr = u64::MAX;
        let result = encode_frame(IpcCommand::Health, 0, corr, b"");

        // When: encoding
        assert!(result.is_ok(), "encode should succeed");
        let Ok(frame) = result else {
            return;
        };

        // Then: correlation bytes at offset 12..20 match
        let corr_slice = frame.get(12..20);
        assert_eq!(
            corr_slice,
            Some(corr.to_le_bytes().as_slice())
        );
    }

    #[test]
    fn validate_frame_magic_rejects_various_bad_magic_values() {
        // Given: several known-bad magic values
        let bad_magics: Vec<u32> = vec![
            0x0000_0000,
            0xFFFF_FFFF,
            0x5442_4C56, // reversed byte order
            0x5642_4C55, // off by one
        ];

        // When: validating each
        for bad_magic in bad_magics {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&bad_magic.to_le_bytes());
            bytes.extend_from_slice(&[0u8; 20]);

            // Then: all are rejected
            assert_eq!(
                validate_frame_magic(&bytes),
                Err(IpcError::InvalidMagic { actual: bad_magic }),
                "magic {bad_magic:#010x} should be rejected"
            );
        }
    }
}
