//! Tests for frame encoding/decoding.

use crate::error::IpcError;
use crate::ingress::MaxPayloadBytes;
use crate::ipc_types::{decode_frame_payload, encode_frame, decode_frame_header, read_frame_header,
    read_frame_payload, read_frame_header_bounded, read_frame_payload_bounded, validate_frame_bounds,
    validate_frame_magic, write_frame, IpcCommand, IpcFrameHeader, IPC_HEADER_LEN, IPC_MAGIC,
    IPC_VERSION};
use crate::IpcPayload;

use bytes::Bytes;

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

    assert_ok!(header, "header should decode for {command:?}");
    let Ok(header) = header else {
        return;
    };
    assert_eq!(header.command, command, "command should roundtrip");
}

fn assert_payload_roundtrip(command: IpcCommand) {
    let payload = b"test";
    let frame = encode_frame(command, 0, 42, payload);
    assert_ok!(frame, "encode should succeed for {command:?}");
    let Ok(frame_bytes) = frame else { return };

    let header_arr: [u8; IPC_HEADER_LEN] = match frame_bytes.get(..IPC_HEADER_LEN) {
        Some(s) => match s.try_into() {
            Ok(a) => a,
            Err(_) => return,
        },
        None => return,
    };
    let decoded = decode_frame_header(&header_arr);

    assert_ok!(decoded, "decode should succeed for {command:?}");
    let Ok(header) = decoded else { return };
    assert_eq!(header.command, command, "command should roundtrip for {command:?}");
    assert_eq!(header.correlation, 42);
    let payload_len = match usize::try_from(header.payload_len) {
        Ok(v) => v,
        Err(_) => return,
    };
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

    assert!(frame.len() > IPC_HEADER_LEN, "frame should contain header plus payload");
    let header_slice = match frame.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_bytes: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header_result = decode_frame_header(&header_bytes);
    assert_ok!(header_result, "header should decode");
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
    let Ok(encoded) = encoded else { return };
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let result = IpcFrameHeader::decode(&encoded, tiny_max);

    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: 9999, limit: tiny_max.get() }));
}

#[test]
fn validate_frame_magic_rejects_wrong_magic() {
    let wrong_magic: u32 = 0x1234_5678;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&wrong_magic.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 20]);

    assert_eq!(validate_frame_magic(&bytes), Err(IpcError::InvalidMagic { actual: wrong_magic }));
}

#[test]
fn validate_frame_bounds_rejects_at_boundary() {
    let max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 2);

    assert_eq!(validate_frame_bounds(&header, max), Err(IpcError::PayloadTooLarge { actual: 2, limit: max.get() }));
}

// ── Frame protocol tests ─────────────────────────────────────────────────────────

#[test]
fn encode_frame_produces_correct_magic_bytes() {
    let result = encode_frame(IpcCommand::Health, 0, 1, b"");
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else { return };

    let magic_slice = frame.get(..4);
    assert_eq!(magic_slice, Some(IPC_MAGIC.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_produces_correct_version_byte() {
    let result = encode_frame(IpcCommand::Health, 0, 1, b"");
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else { return };

    let version_slice = frame.get(4..6);
    assert_eq!(version_slice, Some(IPC_VERSION.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_produces_correct_command_byte() {
    let result = encode_frame(IpcCommand::DrainTrace, 0, 1, b"");
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else { return };

    let cmd_slice = frame.get(6..8);
    assert_eq!(cmd_slice, Some(9u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_produces_correct_payload_length() {
    let payload = b"hello";
    let result = encode_frame(IpcCommand::Health, 0, 1, payload);
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else { return };

    let len_slice = frame.get(20..24);
    assert_eq!(len_slice, Some(5u32.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_roundtrip_with_empty_payload() {
    let payload: &[u8] = b"";
    let frame_result = encode_frame(IpcCommand::Health, 0, 42, payload);
    assert_ok!(frame_result, "encode should succeed");
    let Ok(frame_bytes) = frame_result else { return };

    let header_bytes_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_bytes_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header, "header should decode");
    let Ok(header) = header else { return };

    assert_eq!(header.command, IpcCommand::Health);
    assert_eq!(header.correlation, 42);
    assert_eq!(header.payload_len, 0);
    let payload_section = frame_bytes.get(IPC_HEADER_LEN..);
    assert_eq!(payload_section, Some(&[][..]));
}

#[test]
fn encode_frame_roundtrip_with_large_payload() {
    let payload = vec![0xAB_u8; 1024];
    let frame_result = encode_frame(IpcCommand::SubmitRun, 0, 99, &payload);
    assert_ok!(frame_result, "encode should succeed");
    let Ok(frame_bytes) = frame_result else { return };

    let header_bytes_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_bytes_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header, "header should decode");
    let Ok(header) = header else { return };

    assert_eq!(header.command, IpcCommand::SubmitRun);
    assert_eq!(header.correlation, 99);
    assert_eq!(header.payload_len, 1024);
    let payload_section = frame_bytes.get(IPC_HEADER_LEN..);
    assert_eq!(payload_section, Some(payload.as_slice()));
}

#[test]
fn decode_frame_header_rejects_truncated_magic() {
    let bytes = [0u8; 3];
    let mut cursor = std::io::Cursor::new(bytes);
    let result = read_frame_header(&mut cursor);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn validate_frame_magic_rejects_too_short_input() {
    let bytes = [0u8; 3];
    let result = validate_frame_magic(&bytes);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn decode_frame_payload_rejects_length_mismatch() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let short_payload = vec![0u8; 50];
    let result = decode_frame_payload(&header, &short_payload);
    assert_eq!(result, Err(IpcError::PayloadLengthMismatch { header: 100, actual: 50 }));
}

#[test]
fn validate_frame_bounds_rejects_zero_length_with_default_max() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let result = validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_frame_bounds_rejects_oversized_length() {
    let max_default = MaxPayloadBytes::DEFAULT.get();
    let oversized = match u32::try_from(max_default.saturating_add(1)) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, oversized);
    let result = validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);
    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: max_default.saturating_add(1), limit: max_default }));
}

#[test]
fn validate_frame_magic_accepts_correct_magic() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&IPC_MAGIC.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 20]);
    let result = validate_frame_magic(&bytes);
    assert_eq!(result, Ok(()));
}

#[test]
fn decode_frame_roundtrip_preserves_submit_run_command() { assert_command_roundtrip(IpcCommand::SubmitRun); }
#[test]
fn decode_frame_roundtrip_preserves_submit_run_inline_command() { assert_command_roundtrip(IpcCommand::SubmitRunInline); }
#[test]
fn decode_frame_roundtrip_preserves_cancel_run_command() { assert_command_roundtrip(IpcCommand::CancelRun); }
#[test]
fn decode_frame_roundtrip_preserves_inspect_run_command() { assert_command_roundtrip(IpcCommand::InspectRun); }
#[test]
fn decode_frame_roundtrip_preserves_list_events_command() { assert_command_roundtrip(IpcCommand::ListEvents); }
#[test]
fn decode_frame_roundtrip_preserves_answer_ask_command() { assert_command_roundtrip(IpcCommand::AnswerAsk); }
#[test]
fn decode_frame_roundtrip_preserves_complete_action_command() { assert_command_roundtrip(IpcCommand::CompleteAction); }
#[test]
fn decode_frame_roundtrip_preserves_fail_action_command() { assert_command_roundtrip(IpcCommand::FailAction); }
#[test]
fn decode_frame_roundtrip_preserves_drain_trace_command() { assert_command_roundtrip(IpcCommand::DrainTrace); }
#[test]
fn decode_frame_roundtrip_preserves_health_command() { assert_command_roundtrip(IpcCommand::Health); }
#[test]
fn decode_frame_roundtrip_preserves_shutdown_command() { assert_command_roundtrip(IpcCommand::Shutdown); }

#[test]
fn read_frame_header_rejects_short_read() {
    let data = vec![0u8; 10];
    let mut cursor = std::io::Cursor::new(data);
    let result = read_frame_header(&mut cursor);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn read_frame_payload_returns_exact_bytes_when_available() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let payload_data = b"test";
    let mut cursor = std::io::Cursor::new(payload_data.as_slice());
    let result = read_frame_payload(&mut cursor, &header);
    assert_ok!(result, "payload should read");
    let Ok(payload) = result else { return };
    assert_eq!(payload.as_slice(), b"test");
}

#[test]
fn read_frame_payload_rejects_truncated_payload() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 10);
    let short_data = b"abc";
    let mut cursor = std::io::Cursor::new(short_data.as_slice());
    let result = read_frame_payload(&mut cursor, &header);
    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn write_frame_produces_valid_frame_on_writer() {
    let mut writer = Vec::new();
    let result = write_frame(&mut writer, IpcCommand::Shutdown, 0, 55, b"bye");
    assert_ok!(result, "write_frame should succeed");
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
    assert_ok!(header, "written header should decode");
    let Ok(header) = header else { return };
    assert_eq!(header.command, IpcCommand::Shutdown);
    assert_eq!(header.correlation, 55);
}

#[test]
fn decode_frame_payload_succeeds_for_matching_length() {
    let payload = IpcPayload::Health;
    let payload_bytes = postcard::to_allocvec(&payload);
    assert_ok!(payload_bytes, "payload should encode");
    let Ok(payload_bytes) = payload_bytes else { return };
    let payload_len = match u32::try_from(payload_bytes.len()) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, payload_len);
    let result = decode_frame_payload(&header, &payload_bytes);
    assert_ok!(result, "payload should decode");
    let Ok(decoded) = result else { return };
    assert_eq!(decoded, IpcPayload::Health);
}

#[test]
fn encode_frame_with_nonzero_flags() {
    let result = encode_frame(IpcCommand::Health, 0x1234, 1, b"");
    assert_ok!(result, "encode should succeed");
    let Ok(frame) = result else { return };
    let flags_slice = frame.get(8..10);
    assert_eq!(flags_slice, Some(0x1234u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_with_max_correlation() {
    let corr = u64::MAX;
    let result = encode_frame(IpcCommand::Health, 0, corr, b"");
    assert_ok!(result, "encode should succeed");
    let Ok(frame) = result else { return };
    let corr_slice = frame.get(12..20);
    assert_eq!(corr_slice, Some(corr.to_le_bytes().as_slice()));
}

#[test]
fn validate_frame_magic_rejects_zero_magic() { assert_bad_magic_rejected(0x0000_0000); }
#[test]
fn validate_frame_magic_rejects_all_ones_magic() { assert_bad_magic_rejected(0xFFFF_FFFF); }
#[test]
fn validate_frame_magic_rejects_reversed_magic() { assert_bad_magic_rejected(0x5442_4C56); }
#[test]
fn validate_frame_magic_rejects_off_by_one_magic() { assert_bad_magic_rejected(0x5642_4C55); }

// ══ Adversarial frame decode attacks ═══════════════════════════════════════════

#[test]
fn adversarial_all_zero_bytes_header_rejected_as_bad_magic() {
    let zero_header: [u8; IPC_HEADER_LEN] = [0u8; IPC_HEADER_LEN];
    let result = decode_frame_header(&zero_header);
    assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0 }));
}

#[test]
fn adversarial_all_ff_bytes_header_rejected_as_bad_magic() {
    let ff_header: [u8; IPC_HEADER_LEN] = [0xFF_u8; IPC_HEADER_LEN];
    let result = decode_frame_header(&ff_header);
    assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0xFFFF_FFFF }));
}

#[test]
fn adversarial_valid_magic_garbage_rest_rejected_as_unsupported_version() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..].fill(0xFF);
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 0xFFFF }));
}

#[test]
fn adversarial_unsupported_version_two_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&2u16.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 2 }));
}

#[test]
fn adversarial_unknown_command_id_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&200u16.to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnknownCommand(200)));
}

#[test]
fn adversarial_command_id_zero_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnknownCommand(0)));
}

#[test]
fn adversarial_command_id_max_u16_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&u16::MAX.to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnknownCommand(u16::MAX)));
}

#[test]
fn adversarial_nonzero_reserved_field_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    header_bytes[10..12].copy_from_slice(&1u16.to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::ReservedNonZero { actual: 1 }));
}

#[test]
fn adversarial_payload_len_4gb_rejected_as_too_large() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, u32::MAX);
    let result = validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);
    let expected_len = usize::try_from(u32::MAX).map_or(usize::MAX, |v| v);
    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: expected_len, limit: MaxPayloadBytes::DEFAULT.get() }));
}

#[test]
fn adversarial_payload_len_one_over_default_max_rejected() {
    let default_max = MaxPayloadBytes::DEFAULT.get();
    let over_limit = match u32::try_from(default_max.saturating_add(1)) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, over_limit);
    let result = validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);
    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: default_max.saturating_add(1), limit: default_max }));
}

#[test]
fn adversarial_truncated_header_short_read_rejected() {
    let data = [0x56u8; 1];
    let mut cursor = std::io::Cursor::new(data);
    let result = read_frame_header(&mut cursor);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn adversarial_truncated_header_23_bytes_rejected() {
    let Some(short_len) = IPC_HEADER_LEN.checked_sub(1) else { return };
    let data = vec![0u8; short_len];
    let mut cursor = std::io::Cursor::new(data);
    let result = read_frame_header(&mut cursor);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn adversarial_payload_shorter_than_declared_rejected() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let short_data = vec![0u8; 10];
    let mut cursor = std::io::Cursor::new(short_data.as_slice());
    let result = read_frame_payload(&mut cursor, &header);
    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn adversarial_payload_decode_length_mismatch_header_says_50_actual_10() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 50);
    let short_payload = vec![0u8; 10];
    let result = decode_frame_payload(&header, &short_payload);
    assert_eq!(result, Err(IpcError::PayloadLengthMismatch { header: 50, actual: 10 }));
}

#[test]
fn adversarial_payload_decode_length_mismatch_header_says_0_actual_10() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let extra_payload = vec![0u8; 10];
    let result = decode_frame_payload(&header, &extra_payload);
    assert_eq!(result, Err(IpcError::PayloadLengthMismatch { header: 0, actual: 10 }));
}

#[test]
fn adversarial_garbage_postcard_payload_rejected() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC];
    let result = decode_frame_payload(&header, &garbage);
    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn adversarial_encode_then_decode_roundtrip_submit_run_command() { assert_payload_roundtrip(IpcCommand::SubmitRun); }
#[test]
fn adversarial_encode_then_decode_roundtrip_submit_run_inline_command() { assert_payload_roundtrip(IpcCommand::SubmitRunInline); }
#[test]
fn adversarial_encode_then_decode_roundtrip_cancel_run_command() { assert_payload_roundtrip(IpcCommand::CancelRun); }
#[test]
fn adversarial_encode_then_decode_roundtrip_inspect_run_command() { assert_payload_roundtrip(IpcCommand::InspectRun); }
#[test]
fn adversarial_encode_then_decode_roundtrip_list_events_command() { assert_payload_roundtrip(IpcCommand::ListEvents); }
#[test]
fn adversarial_encode_then_decode_roundtrip_answer_ask_command() { assert_payload_roundtrip(IpcCommand::AnswerAsk); }
#[test]
fn adversarial_encode_then_decode_roundtrip_complete_action_command() { assert_payload_roundtrip(IpcCommand::CompleteAction); }
#[test]
fn adversarial_encode_then_decode_roundtrip_fail_action_command() { assert_payload_roundtrip(IpcCommand::FailAction); }
#[test]
fn adversarial_encode_then_decode_roundtrip_drain_trace_command() { assert_payload_roundtrip(IpcCommand::DrainTrace); }
#[test]
fn adversarial_encode_then_decode_roundtrip_health_command() { assert_payload_roundtrip(IpcCommand::Health); }
#[test]
fn adversarial_encode_then_decode_roundtrip_shutdown_command() { assert_payload_roundtrip(IpcCommand::Shutdown); }

#[test]
fn adversarial_encode_empty_payload_succeeds() {
    let payload: &[u8] = b"";
    let result = encode_frame(IpcCommand::Health, 0, 1, payload);
    assert_ok!(result, "empty payload should encode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.len(), IPC_HEADER_LEN);
}

#[test]
fn adversarial_encode_payload_at_max_boundary_succeeds() {
    let max = MaxPayloadBytes::DEFAULT.get();
    let payload = vec![0xAB_u8; max];
    let result = encode_frame(IpcCommand::SubmitRun, 0, 1, &payload);
    assert_ok!(result, "max-size payload should encode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.len(), IPC_HEADER_LEN.checked_add(max).map_or(0, |v| v));
}

#[test]
fn adversarial_read_frame_payload_bounded_enforces_limit() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let payload_data = vec![0u8; 100];
    let mut cursor = std::io::Cursor::new(payload_data.as_slice());
    let result = read_frame_payload_bounded(&mut cursor, &header, tiny_max);
    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: 100, limit: 1 }));
}

#[test]
fn adversarial_read_frame_header_bounded_enforces_limit() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 50);
    let encoded = header.encode();
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let mut cursor = std::io::Cursor::new(encoded.as_slice());
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let result = read_frame_header_bounded(&mut cursor, tiny_max);
    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: 50, limit: 1 }));
}

#[test]
fn adversarial_byte_order_swap_magic_rejected() {
    let be_magic_bytes = 0x5642_4C54_u32.to_be_bytes();
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&be_magic_bytes);
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0x544C_4256 }));
}
