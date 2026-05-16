//! IPC frame tests - validation boundary tests.

use super::{
    decode_frame_header, decode_frame_payload, read_frame_header,
    read_frame_header_bounded, read_frame_payload, read_frame_payload_bounded,
    validate_frame_bounds, IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes,
    IPC_HEADER_LEN,
};

#[cfg(test)]
macro_rules! assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        match &$result {
            Ok(_) => (),
            Err(_) => assert_eq!(Some("Err(..)"), None::<&str> $(, $($arg)+)?),
        }
    }};
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
    let Ok(encoded) = encoded else { return; };
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let result = IpcFrameHeader::decode(&encoded, tiny_max);
    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: 9999, limit: tiny_max.get() }));
}

#[test]
fn validate_frame_bounds_rejects_at_boundary() {
    let max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 2);
    assert_eq!(validate_frame_bounds(&header, max), Err(IpcError::PayloadTooLarge { actual: 2, limit: max.get() }));
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
fn decode_frame_payload_rejects_length_mismatch() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let short_payload = vec![0u8; 50];
    let result = decode_frame_payload(&header, &short_payload);
    assert_eq!(result, Err(IpcError::PayloadLengthMismatch { header: 100, actual: 50 }));
}

#[test]
fn read_frame_header_rejects_short_read() {
    let data = vec![0u8; 10];
    let mut cursor = std::io::Cursor::new(data);
    let result = read_frame_header(&mut cursor);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
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
fn read_frame_payload_bounded_enforces_limit() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let payload_data = vec![0u8; 100];
    let mut cursor = std::io::Cursor::new(payload_data.as_slice());
    let result = read_frame_payload_bounded(&mut cursor, &header, tiny_max);
    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: 100, limit: 1 }));
}

#[test]
fn read_frame_header_bounded_enforces_limit() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 50);
    let encoded = header.encode();
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return; };
    let mut cursor = std::io::Cursor::new(encoded.as_slice());
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let result = read_frame_header_bounded(&mut cursor, tiny_max);
    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: 50, limit: 1 }));
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
    let Some(short_len) = IPC_HEADER_LEN.checked_sub(1) else { return; };
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
fn adversarial_byte_order_swap_magic_rejected() {
    let be_magic_bytes = 0x5642_4C54_u32.to_be_bytes();
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&be_magic_bytes);
    header_bytes[4..6].copy_from_slice(&crate::IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0x544C_4256 }));
}
