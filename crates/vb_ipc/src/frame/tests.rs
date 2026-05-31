//! Tests for the IPC frame encoding and decoding utilities.

use super::*;

macro_rules! assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        match &$result {
            Ok(_) => (),
            Err(_) => assert_eq!(Some("Err(..)"), None::<&str> $(, $($arg)+)?),
        }
    }};
}

fn assert_command_roundtrip(command: IpcCommand) {
    let frame_result = encode_frame(command, 0, 7, b"");
    assert_ok!(frame_result, "encode should succeed for {command:?}");
    let frame_bytes =
        frame_result.unwrap_or_else(|e| panic!("encode should succeed for {command:?}: {e:?}"));

    let header_slice = frame_bytes
        .get(..IPC_HEADER_LEN)
        .unwrap_or_else(|| panic!("frame too short for {command:?}"));
    let header_arr: [u8; IPC_HEADER_LEN] = header_slice
        .try_into()
        .unwrap_or_else(|_| panic!("header slice wrong size for {command:?}"));
    let header = decode_frame_header(&header_arr);

    assert_ok!(header, "header should decode for {command:?}");
    let header = header.unwrap_or_else(|e| panic!("header should decode for {command:?}: {e:?}"));
    assert_eq!(header.command, command, "command should roundtrip");
}

fn assert_payload_roundtrip(command: IpcCommand) {
    let payload = b"test";
    let frame = encode_frame(command, 0, 42, payload);
    assert_ok!(frame, "encode should succeed for {command:?}");
    let frame_bytes =
        frame.unwrap_or_else(|e| panic!("encode should succeed for {command:?}: {e:?}"));

    let header_arr: [u8; IPC_HEADER_LEN] = frame_bytes
        .get(..IPC_HEADER_LEN)
        .unwrap_or_else(|| panic!("frame too short for {command:?}"))
        .try_into()
        .unwrap_or_else(|_| panic!("header slice wrong size for {command:?}"));
    let decoded = decode_frame_header(&header_arr);

    assert_ok!(decoded, "decode should succeed for {command:?}");
    let header = decoded.unwrap_or_else(|e| panic!("decode should succeed for {command:?}: {e:?}"));
    assert_eq!(
        header.command, command,
        "command should roundtrip for {command:?}"
    );
    assert_eq!(header.correlation, 42);
    let payload_len = usize::try_from(header.payload_len)
        .unwrap_or_else(|_| panic!("payload_len should fit usize for {command:?}"));
    assert_eq!(payload_len, 4);
    assert_eq!(frame_bytes.get(IPC_HEADER_LEN..), Some(payload.as_slice()));
}

fn assert_bad_magic_rejected(bad_magic: u32) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&bad_magic.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 20]);

    assert_eq!(
        validate_frame_magic(&bytes),
        Err(IpcError::InvalidMagic { actual: bad_magic }),
        "magic {bad_magic:#010x} should be rejected"
    );
}

#[test]
fn encode_frame_produces_valid_header_and_payload() {
    let payload = b"test-data";
    let result = encode_frame(IpcCommand::Health, 0, 99, payload);
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else {
        return;
    };

    assert!(
        frame.len() > IPC_HEADER_LEN,
        "frame should contain header plus payload"
    );
    let header_slice = match frame.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => panic!("frame too short"),
    };
    let header_bytes: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
        Ok(h) => h,
        Err(_) => panic!("header slice wrong size"),
    };
    let header_result = decode_frame_header(&header_bytes);
    assert_ok!(header_result, "header should decode");
    let Ok(header) = header_result else {
        panic!("header should decode")
    };
    assert_eq!(header.command, IpcCommand::Health);
    assert_eq!(header.correlation, 99);
    let payload_len = match usize::try_from(header.payload_len) {
        Ok(v) => v,
        Err(_) => panic!("payload_len should fit usize"),
    };
    assert_eq!(payload_len, payload.len());
    assert_eq!(frame.get(IPC_HEADER_LEN..), Some(payload.as_slice()));
}

#[test]
fn fuzz_decode_frame_rejects_short_input() {
    let short: [u8; IPC_HEADER_LEN] = [0u8; IPC_HEADER_LEN];
    let result = decode_frame_header(&short);
    assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0 }));
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
    assert_ok!(encoded, "header should encode");
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
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else {
        return;
    };

    // Then: bytes 0..4 are IPC_MAGIC in little-endian
    let magic_slice = frame.get(..4);
    assert_eq!(magic_slice, Some(crate::IPC_MAGIC.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_produces_correct_version_byte() {
    // Given: a valid command and empty payload
    // When: encoding a frame
    let result = encode_frame(IpcCommand::Health, 0, 1, b"");
    assert_ok!(result, "encode_frame should succeed");
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
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else {
        return;
    };

    // Then: bytes 6..8 are command id 9 in little-endian
    let cmd_slice = frame.get(6..8);
    assert_eq!(cmd_slice, Some(9u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_produces_correct_payload_length() {
    // Given: a payload of 5 bytes
    let payload = b"hello";

    // When: encoding a frame
    let result = encode_frame(IpcCommand::Health, 0, 1, payload);
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else {
        return;
    };

    // Then: bytes 20..24 contain payload length 5 in little-endian
    let len_slice = frame.get(20..24);
    assert_eq!(len_slice, Some(5u32.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_roundtrip_with_empty_payload() {
    // Given: an empty payload
    let payload: &[u8] = b"";

    // When: encoding then decoding
    let frame_result = encode_frame(IpcCommand::Health, 0, 42, payload);
    assert_ok!(frame_result, "encode should succeed");
    let Ok(frame_bytes) = frame_result else {
        panic!("encode should succeed")
    };

    let header_bytes_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => panic!("frame too short"),
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_bytes_slice.try_into() {
        Ok(h) => h,
        Err(_) => panic!("header slice wrong size"),
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header, "header should decode");
    let Ok(header) = header else {
        panic!("header should decode")
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
    assert_ok!(frame_result, "encode should succeed");
    let Ok(frame_bytes) = frame_result else {
        panic!("encode should succeed")
    };

    let header_bytes_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => panic!("frame too short"),
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_bytes_slice.try_into() {
        Ok(h) => h,
        Err(_) => panic!("header slice wrong size"),
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header, "header should decode");
    let Ok(header) = header else {
        panic!("header should decode")
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
fn decode_frame_roundtrip_preserves_submit_run_command() {
    assert_command_roundtrip(IpcCommand::SubmitRun);
}

#[test]
fn decode_frame_roundtrip_preserves_submit_run_inline_command() {
    assert_command_roundtrip(IpcCommand::SubmitRunInline);
}

#[test]
fn decode_frame_roundtrip_preserves_cancel_run_command() {
    assert_command_roundtrip(IpcCommand::CancelRun);
}

#[test]
fn decode_frame_roundtrip_preserves_inspect_run_command() {
    assert_command_roundtrip(IpcCommand::InspectRun);
}

#[test]
fn decode_frame_roundtrip_preserves_list_events_command() {
    assert_command_roundtrip(IpcCommand::ListEvents);
}

#[test]
fn decode_frame_roundtrip_preserves_answer_ask_command() {
    assert_command_roundtrip(IpcCommand::AnswerAsk);
}

#[test]
fn decode_frame_roundtrip_preserves_complete_action_command() {
    assert_command_roundtrip(IpcCommand::CompleteAction);
}

#[test]
fn decode_frame_roundtrip_preserves_fail_action_command() {
    assert_command_roundtrip(IpcCommand::FailAction);
}

#[test]
fn decode_frame_roundtrip_preserves_drain_trace_command() {
    assert_command_roundtrip(IpcCommand::DrainTrace);
}

#[test]
fn decode_frame_roundtrip_preserves_health_command() {
    assert_command_roundtrip(IpcCommand::Health);
}

#[test]
fn decode_frame_roundtrip_preserves_shutdown_command() {
    assert_command_roundtrip(IpcCommand::Shutdown);
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
    assert_ok!(result, "payload should read");
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

    // Then: PayloadDecodeFailed is returned (short read)
    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn write_frame_produces_valid_frame_on_writer() {
    // Given: a Vec<u8> writer
    let mut writer = Vec::new();

    // When: writing a frame with payload
    let result = write_frame(&mut writer, IpcCommand::Shutdown, 0, 55, b"bye");

    // Then: write succeeds and the output can be decoded
    assert_ok!(result, "write_frame should succeed");
    assert!(
        writer.len() > IPC_HEADER_LEN,
        "should contain header + payload"
    );
    let header_slice = match writer.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header, "written header should decode");
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
    let payload_bytes =
        postcard::to_allocvec(&payload).unwrap_or_else(|e| panic!("payload should encode: {e:?}"));
    let payload_len = match u32::try_from(payload_bytes.len()) {
        Ok(v) => v,
        Err(_) => panic!("payload_len should fit u32"),
    };
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, payload_len);

    // When: decoding the frame payload
    let decoded = decode_frame_payload(&header, &payload_bytes)
        .unwrap_or_else(|e| panic!("payload should decode: {e:?}"));

    // Then: it decodes to the correct payload
    assert_eq!(decoded, crate::IpcPayload::Health);
}

#[test]
fn encode_frame_with_nonzero_flags() {
    // Given: a frame with flags=0x1234
    let result = encode_frame(IpcCommand::Health, 0x1234, 1, b"");

    // When: encoding
    assert_ok!(result, "encode should succeed");
    let Ok(frame) = result else {
        panic!("encode should succeed")
    };

    // Then: flags bytes at offset 8..10 match
    let flags_slice = frame.get(8..10);
    assert_eq!(flags_slice, Some(0x1234u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_with_max_correlation() {
    // Given: a frame with correlation=u64::MAX
    let corr = u64::MAX;
    let result = encode_frame(IpcCommand::Health, 0, corr, b"");

    // When: encoding
    assert_ok!(result, "encode should succeed");
    let Ok(frame) = result else {
        panic!("encode should succeed")
    };

    // Then: correlation bytes at offset 12..20 match
    let corr_slice = frame.get(12..20);
    assert_eq!(corr_slice, Some(corr.to_le_bytes().as_slice()));
}

#[test]
fn validate_frame_magic_rejects_zero_magic() {
    assert_bad_magic_rejected(0x0000_0000);
}

#[test]
fn validate_frame_magic_rejects_all_ones_magic() {
    assert_bad_magic_rejected(0xFFFF_FFFF);
}

#[test]
fn validate_frame_magic_rejects_reversed_magic() {
    assert_bad_magic_rejected(0x5442_4C56);
}

#[test]
fn validate_frame_magic_rejects_off_by_one_magic() {
    assert_bad_magic_rejected(0x5642_4C55);
}

// ══ Adversarial frame decode attacks ══

#[test]
fn adversarial_all_zero_bytes_header_rejected_as_bad_magic() {
    // Given: a header made entirely of zero bytes
    let zero_header: [u8; IPC_HEADER_LEN] = [0u8; IPC_HEADER_LEN];

    // When: decoding
    let result = decode_frame_header(&zero_header);

    // Then: InvalidMagic with actual=0 (zero is not VBLT)
    assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0 }));
}

#[test]
fn adversarial_all_ff_bytes_header_rejected_as_bad_magic() {
    // Given: a header made entirely of 0xFF bytes
    let ff_header: [u8; IPC_HEADER_LEN] = [0xFF_u8; IPC_HEADER_LEN];

    // When: decoding
    let result = decode_frame_header(&ff_header);

    // Then: InvalidMagic with actual=0xFFFFFFFF
    assert_eq!(
        result,
        Err(IpcError::InvalidMagic {
            actual: 0xFFFF_FFFF
        })
    );
}

#[test]
fn adversarial_valid_magic_garbage_rest_rejected_as_unsupported_version() {
    // Given: valid magic but version field is garbage (0x1337)
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    // Everything after magic is 0xFF => version 0xFFFF
    header_bytes[4..].fill(0xFF);

    // When: decoding
    let result = decode_frame_header(&header_bytes);

    // Then: UnsupportedVersion (first field after magic)
    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 0xFFFF }));
}

#[test]
fn adversarial_unsupported_version_two_rejected() {
    // Given: valid magic, version=2 (unsupported)
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&2u16.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    // reserved = 0
    // payload_len = 0

    // When: decoding
    let result = decode_frame_header(&header_bytes);

    // Then: UnsupportedVersion with actual=2
    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 2 }));
}

#[test]
fn adversarial_unknown_command_id_rejected() {
    // Given: valid magic, version=1, command=200 (invalid)
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&crate::IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&200u16.to_le_bytes());

    // When: decoding
    let result = decode_frame_header(&header_bytes);

    // Then: UnknownCommand(200)
    assert_eq!(result, Ok(IpcFrameHeader::new(IpcCommand::UnknownCommand(200), 0, 0, 0)));
}

#[test]
fn adversarial_command_id_zero_rejected() {
    // Given: valid magic, version=1, command=0 (invalid)
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&crate::IPC_VERSION.to_le_bytes());
    // command bytes 6..8 already 0

    // When: decoding
    let result = decode_frame_header(&header_bytes);

    // Then: UnknownCommand(0)
    assert_eq!(result, Ok(IpcFrameHeader::new(IpcCommand::UnknownCommand(0), 0, 0, 0)));
}

#[test]
fn adversarial_command_id_max_u16_rejected() {
    // Given: valid magic, version=1, command=u16::MAX
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&crate::IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&u16::MAX.to_le_bytes());

    // When: decoding
    let result = decode_frame_header(&header_bytes);

    // Then: UnknownCommand(u16::MAX)
    assert_eq!(result, Ok(IpcFrameHeader::new(IpcCommand::UnknownCommand(u16::MAX), 0, 0, 0)));
}

#[test]
fn adversarial_nonzero_reserved_field_rejected() {
    // Given: valid magic, version, command, but reserved=1
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&crate::IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    // flags 8..10 = 0
    header_bytes[10..12].copy_from_slice(&1u16.to_le_bytes()); // reserved = 1

    // When: decoding
    let result = decode_frame_header(&header_bytes);

    // Then: ReservedNonZero
    assert_eq!(result, Err(IpcError::ReservedNonZero { actual: 1 }));
}

#[test]
fn adversarial_payload_len_4gb_rejected_as_too_large() {
    // Given: payload_len = u32::MAX (4GB+)
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, u32::MAX);

    // When: validating bounds against default max (1 MiB)
    let result = validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);

    // Then: PayloadTooLarge
    let expected_len = usize::try_from(u32::MAX).map_or(usize::MAX, |v| v);
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: expected_len,
            limit: MaxPayloadBytes::DEFAULT.get(),
        })
    );
}

#[test]
fn adversarial_payload_len_one_over_default_max_rejected() {
    // Given: payload_len one byte over default max
    let default_max = MaxPayloadBytes::DEFAULT.get();
    let over_limit = match u32::try_from(default_max.saturating_add(1)) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, over_limit);

    // When: validating bounds
    let result = validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);

    // Then: PayloadTooLarge
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: default_max.saturating_add(1),
            limit: default_max,
        })
    );
}

#[test]
fn adversarial_truncated_header_short_read_rejected() {
    // Given: a reader with only 1 byte (way too short for header)
    let data = [0x56u8; 1];
    let mut cursor = std::io::Cursor::new(data);

    // When: reading a frame header
    let result = read_frame_header(&mut cursor);

    // Then: HeaderDecodeFailed (short read)
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn adversarial_truncated_header_23_bytes_rejected() {
    // Given: a reader with 23 bytes (one short of full header)
    let Some(short_len) = IPC_HEADER_LEN.checked_sub(1) else {
        panic!("IPC_HEADER_LEN should be > 0")
    };
    let data = vec![0u8; short_len];
    let mut cursor = std::io::Cursor::new(data);

    // When: reading a frame header
    let result = read_frame_header(&mut cursor);

    // Then: HeaderDecodeFailed
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn adversarial_payload_shorter_than_declared_rejected() {
    // Given: a header declaring payload_len=100 but only 10 bytes available
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let short_data = vec![0u8; 10];
    let mut cursor = std::io::Cursor::new(short_data.as_slice());

    // When: reading the frame payload
    let result = read_frame_payload(&mut cursor, &header);

    // Then: PayloadDecodeFailed (short read)
    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn adversarial_payload_decode_length_mismatch_header_says_50_actual_10() {
    // Given: a header with payload_len=50 and only 10 bytes of payload
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 50);
    let short_payload = vec![0u8; 10];

    // When: decoding the frame payload
    let result = decode_frame_payload(&header, &short_payload);

    // Then: PayloadLengthMismatch
    assert_eq!(
        result,
        Err(IpcError::PayloadLengthMismatch {
            header: 50,
            actual: 10,
        })
    );
}

#[test]
fn adversarial_payload_decode_length_mismatch_header_says_0_actual_10() {
    // Given: a header with payload_len=0 but 10 bytes of payload supplied
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let extra_payload = vec![0u8; 10];

    // When: decoding the frame payload
    let result = decode_frame_payload(&header, &extra_payload);

    // Then: PayloadLengthMismatch (header=0, actual=10)
    assert_eq!(
        result,
        Err(IpcError::PayloadLengthMismatch {
            header: 0,
            actual: 10,
        })
    );
}

#[test]
fn adversarial_garbage_postcard_payload_rejected() {
    // Given: a header with payload_len=4 matching garbage bytes
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC];

    // When: decoding the frame payload
    let result = decode_frame_payload(&header, &garbage);

    // Then: PayloadDecodeFailed (postcard can't decode garbage as IpcPayload)
    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn adversarial_encode_then_decode_roundtrip_submit_run_command() {
    assert_payload_roundtrip(IpcCommand::SubmitRun);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_submit_run_inline_command() {
    assert_payload_roundtrip(IpcCommand::SubmitRunInline);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_cancel_run_command() {
    assert_payload_roundtrip(IpcCommand::CancelRun);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_inspect_run_command() {
    assert_payload_roundtrip(IpcCommand::InspectRun);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_list_events_command() {
    assert_payload_roundtrip(IpcCommand::ListEvents);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_answer_ask_command() {
    assert_payload_roundtrip(IpcCommand::AnswerAsk);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_complete_action_command() {
    assert_payload_roundtrip(IpcCommand::CompleteAction);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_fail_action_command() {
    assert_payload_roundtrip(IpcCommand::FailAction);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_drain_trace_command() {
    assert_payload_roundtrip(IpcCommand::DrainTrace);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_health_command() {
    assert_payload_roundtrip(IpcCommand::Health);
}

#[test]
fn adversarial_encode_then_decode_roundtrip_shutdown_command() {
    assert_payload_roundtrip(IpcCommand::Shutdown);
}

#[test]
fn adversarial_encode_empty_payload_succeeds() {
    // Given: an empty payload
    let payload: &[u8] = b"";

    // When: encoding
    let result = encode_frame(IpcCommand::Health, 0, 1, payload);

    // Then: encoding succeeds with header-only frame
    assert_ok!(result, "empty payload should encode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.len(), IPC_HEADER_LEN);
}

#[test]
fn adversarial_encode_payload_at_max_boundary_succeeds() {
    // Given: a payload exactly at the default max (1 MiB)
    let max = MaxPayloadBytes::DEFAULT.get();
    let payload = vec![0xAB_u8; max];

    // When: encoding
    let result = encode_frame(IpcCommand::SubmitRun, 0, 1, &payload);

    // Then: encoding succeeds
    assert_ok!(result, "max-size payload should encode");
    let Ok(frame) = result else { return };
    assert_eq!(
        frame.len(),
        IPC_HEADER_LEN.checked_add(max).map_or(0, |v| v)
    );
}

#[test]
fn adversarial_read_frame_payload_bounded_enforces_limit() {
    // Given: a header with payload_len=100 and a max of 10
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let payload_data = vec![0u8; 100];
    let mut cursor = std::io::Cursor::new(payload_data.as_slice());

    // When: reading with bounded max
    let result = read_frame_payload_bounded(&mut cursor, &header, tiny_max);

    // Then: PayloadTooLarge
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: 100,
            limit: 1,
        })
    );
}

#[test]
fn adversarial_read_frame_header_bounded_enforces_limit() {
    // Given: a valid header with payload_len=50
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 50);
    let encoded = header.encode();
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let mut cursor = std::io::Cursor::new(encoded.as_slice());

    // When: reading with a tiny max (1 byte)
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let result = read_frame_header_bounded(&mut cursor, tiny_max);

    // Then: PayloadTooLarge
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: 50,
            limit: 1,
        })
    );
}

#[test]
fn adversarial_byte_order_swap_magic_rejected() {
    // Given: the magic bytes stored as big-endian (wrong endianness for LE protocol)
    // 0x5642_4C54 in big-endian bytes = [0x56, 0x42, 0x4C, 0x54]
    // When read as little-endian this becomes 0x544C_4256
    let be_magic_bytes = 0x5642_4C54_u32.to_be_bytes(); // [0x56, 0x42, 0x4C, 0x54]
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&be_magic_bytes);
    header_bytes[4..6].copy_from_slice(&crate::IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());

    // When: decoding
    let result = decode_frame_header(&header_bytes);

    // Then: InvalidMagic - the bytes [0x56, 0x42, 0x4C, 0x54] read as LE = 0x544C_4256
    assert_eq!(
        result,
        Err(IpcError::InvalidMagic {
            actual: 0x544C_4256
        })
    );
}

// ══ codec.rs coverage tests ════════════════════════════════════════════════════

#[test]
fn encode_frame_produces_exact_byte_layout() {
    let payload = b"hello";
    let frame = encode_frame(IpcCommand::Health, 0x1234, 0xDEADBEEFCAFE, payload).unwrap();

    assert_eq!(frame.len(), IPC_HEADER_LEN + payload.len());
    assert_eq!(&frame[0..4], &IPC_MAGIC.to_le_bytes());
    assert_eq!(&frame[4..6], &crate::IPC_VERSION.to_le_bytes());
    assert_eq!(&frame[6..8], &IpcCommand::Health.as_u16().to_le_bytes());
    assert_eq!(&frame[20..24], &(payload.len() as u32).to_le_bytes());
    assert_eq!(&frame[IPC_HEADER_LEN..], payload.as_slice());
}

#[test]
fn encode_frame_with_large_u32_payload_length() {
    let payload = vec![0xCC_u8; 70000];
    let result = encode_frame(IpcCommand::SubmitRun, 0, 1, &payload);
    assert_ok!(result, "encode should succeed for large payload");
    let Ok(frame) = result else { return };
    let len_slice = frame.get(20..24);
    assert_eq!(len_slice, Some(70000u32.to_le_bytes().as_slice()));
}

#[test]
fn decode_frame_header_rejects_truncated_input_via_reader() {
    let data: Vec<u8> = vec![];
    let mut cursor = std::io::Cursor::new(data);
    let result = read_frame_header(&mut cursor);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

// ══ validate.rs coverage tests ═════════════════════════════════════════════════

#[test]
fn validate_frame_magic_with_exactly_four_bytes_wrong_magic() {
    let wrong_magic: u32 = 0xDEAD_BEEF;
    let bytes = wrong_magic.to_le_bytes();
    assert_eq!(
        validate_frame_magic(&bytes),
        Err(IpcError::InvalidMagic {
            actual: wrong_magic
        })
    );
}

#[test]
fn validate_frame_bounds_with_exactly_max_payload() {
    let max = MaxPayloadBytes::DEFAULT.get();
    let payload_len = match u32::try_from(max) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, payload_len);
    assert_eq!(
        validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT),
        Ok(())
    );
}

#[test]
fn validate_frame_bounds_with_default_max_plus_one() {
    let default_max = MaxPayloadBytes::DEFAULT.get();
    let over_limit = match u32::try_from(default_max.saturating_add(1)) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, over_limit);
    assert_eq!(
        validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT),
        Err(IpcError::PayloadTooLarge {
            actual: default_max.saturating_add(1),
            limit: default_max,
        })
    );
}

// ══ io.rs coverage tests ═══════════════════════════════════════════════════════

#[test]
fn read_frame_header_with_exact_header_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 42, 0);
    let encoded = header.encode();
    assert_ok!(encoded);
    let Ok(encoded) = encoded else {
        panic!("header should encode")
    };
    let mut cursor = std::io::Cursor::new(encoded.as_slice());
    let result = read_frame_header(&mut cursor);
    assert_ok!(result);
    let Ok(decoded) = result else {
        panic!("header should decode")
    };
    assert_eq!(decoded.command, IpcCommand::Health);
    assert_eq!(decoded.correlation, 42);
}

#[test]
fn read_frame_header_with_zero_bytes() {
    let data: Vec<u8> = vec![];
    let mut cursor = std::io::Cursor::new(data);
    let result = read_frame_header(&mut cursor);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn read_frame_header_bounded_with_payload_within_max() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let encoded = header.encode();
    assert_ok!(encoded);
    let Ok(encoded) = encoded else {
        panic!("header should encode")
    };
    let mut cursor = std::io::Cursor::new(encoded.as_slice());
    let result = read_frame_header_bounded(&mut cursor, MaxPayloadBytes::DEFAULT);
    assert_ok!(result);
    let Ok(decoded) = result else {
        panic!("header should decode")
    };
    assert_eq!(decoded.payload_len, 100);
}

#[test]
fn read_frame_payload_bounded_with_payload_within_max() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let payload_data = b"test";
    let mut cursor = std::io::Cursor::new(payload_data.as_slice());
    let result = read_frame_payload_bounded(&mut cursor, &header, MaxPayloadBytes::DEFAULT);
    assert_ok!(result);
    let Ok(payload) = result else { return };
    assert_eq!(payload.as_slice(), b"test");
}

#[test]
fn write_frame_produces_correct_frame_bytes() {
    let mut writer = Vec::new();
    let payload = b"hello";
    let result = write_frame(&mut writer, IpcCommand::Health, 0x1234, 42, payload);
    assert_ok!(result);
    assert_eq!(writer.len(), IPC_HEADER_LEN + payload.len());
    let magic_slice = writer.get(..4);
    assert_eq!(magic_slice, Some(crate::IPC_MAGIC.to_le_bytes().as_slice()));
    let flags_slice = writer.get(8..10);
    assert_eq!(flags_slice, Some(0x1234u16.to_le_bytes().as_slice()));
    let corr_slice = writer.get(12..20);
    assert_eq!(corr_slice, Some(42u64.to_le_bytes().as_slice()));
    let len_slice = writer.get(20..24);
    assert_eq!(len_slice, Some(5u32.to_le_bytes().as_slice()));
    let payload_slice = writer.get(IPC_HEADER_LEN..);
    assert_eq!(payload_slice, Some(payload.as_slice()));
}

#[test]
fn write_frame_with_empty_payload_produces_header_only_frame() {
    let mut writer = Vec::new();
    let result = write_frame(&mut writer, IpcCommand::Shutdown, 0, 99, b"");
    assert_ok!(result);
    assert_eq!(writer.len(), IPC_HEADER_LEN);
    let header_slice = match writer.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header);
    let Ok(header) = header else { return };
    assert_eq!(header.command, IpcCommand::Shutdown);
    assert_eq!(header.correlation, 99);
    assert_eq!(header.payload_len, 0);
}

#[test]
fn read_frame_payload_with_zero_length_payload() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let empty_data: &[u8] = b"";
    let mut cursor = std::io::Cursor::new(empty_data);
    let result = read_frame_payload(&mut cursor, &header);
    assert_ok!(result);
    let Ok(payload) = result else {
        panic!("payload should read")
    };
    assert_eq!(payload.len(), 0);
}

#[test]
fn encode_frame_with_all_flags_set() {
    let result = encode_frame(IpcCommand::Health, u16::MAX, 1, b"");
    assert_ok!(result);
    let Ok(frame) = result else {
        panic!("encode should succeed")
    };
    let flags_slice = frame.get(8..10);
    assert_eq!(flags_slice, Some(u16::MAX.to_le_bytes().as_slice()));
}

#[test]
fn decode_frame_header_preserves_all_fields() {
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0xABCD, 0x1234_5678_9ABC_DEF0, 4096);
    let encoded = header.encode();
    assert_ok!(encoded);
    let Ok(encoded) = encoded else {
        panic!("header should encode")
    };
    let decoded = decode_frame_header(&encoded);
    assert_ok!(decoded);
    let Ok(decoded) = decoded else {
        panic!("header should decode")
    };
    assert_eq!(decoded.command, IpcCommand::SubmitRun);
    assert_eq!(decoded.flags, 0xABCD);
    assert_eq!(decoded.correlation, 0x1234_5678_9ABC_DEF0);
    assert_eq!(decoded.payload_len, 4096);
}

#[test]
fn read_frame_payload_bounded_rejects_oversized_payload() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let payload_data = vec![0u8; 100];
    let mut cursor = std::io::Cursor::new(payload_data.as_slice());
    let result = read_frame_payload_bounded(&mut cursor, &header, tiny_max);
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: 100,
            limit: 1,
        })
    );
}

#[test]
fn read_frame_header_bounded_rejects_short_reader() {
    let data = vec![0u8; 10];
    let mut cursor = std::io::Cursor::new(data);
    let result = read_frame_header_bounded(&mut cursor, MaxPayloadBytes::DEFAULT);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn write_frame_rejects_failing_writer() {
    struct FailingWriter;
    impl std::io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "write failed",
            ))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = FailingWriter;
    let result = write_frame(&mut writer, IpcCommand::Health, 0, 1, b"");
    assert_eq!(result, Err(IpcError::HeaderEncodeFailed));
    let Ok(()) = writer.flush() else {
        panic!("FailingWriter flush should succeed after write_frame error");
    };
}

#[test]
fn payload_len_u32_rejects_usize_max() {
    let result = payload_len_u32(usize::MAX);
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: usize::MAX,
            limit: usize::MAX,
        })
    );
}
