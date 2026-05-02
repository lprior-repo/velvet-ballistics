//! IPC frame tests - protocol format tests.

use super::{
    decode_frame_header, encode_frame, validate_frame_magic, IpcCommand, IpcError,
    IpcFrameHeader, MaxPayloadBytes, IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION,
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
fn encode_frame_produces_correct_magic_bytes() {
    let result = encode_frame(IpcCommand::Health, 0, 1, b"");
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else { return; };
    let magic_slice = frame.get(..4);
    assert_eq!(magic_slice, Some(IPC_MAGIC.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_produces_correct_version_byte() {
    let result = encode_frame(IpcCommand::Health, 0, 1, b"");
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else { return; };
    let version_slice = frame.get(4..6);
    assert_eq!(version_slice, Some(IPC_VERSION.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_produces_correct_command_byte() {
    let result = encode_frame(IpcCommand::DrainTrace, 0, 1, b"");
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else { return; };
    let cmd_slice = frame.get(6..8);
    assert_eq!(cmd_slice, Some(9u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_produces_correct_payload_length() {
    let payload = b"hello";
    let result = encode_frame(IpcCommand::Health, 0, 1, payload);
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else { return; };
    let len_slice = frame.get(20..24);
    assert_eq!(len_slice, Some(5u32.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_with_nonzero_flags() {
    let result = encode_frame(IpcCommand::Health, 0x1234, 1, b"");
    assert_ok!(result, "encode should succeed");
    let Ok(frame) = result else { return; };
    let flags_slice = frame.get(8..10);
    assert_eq!(flags_slice, Some(0x1234u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_frame_with_max_correlation() {
    let corr = u64::MAX;
    let result = encode_frame(IpcCommand::Health, 0, corr, b"");
    assert_ok!(result, "encode should succeed");
    let Ok(frame) = result else { return; };
    let corr_slice = frame.get(12..20);
    assert_eq!(corr_slice, Some(corr.to_le_bytes().as_slice()));
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
fn validate_frame_magic_rejects_zero_magic() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 20]);
    assert_eq!(validate_frame_magic(&bytes), Err(IpcError::InvalidMagic { actual: 0 }));
}

#[test]
fn validate_frame_magic_rejects_all_ones_magic() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 20]);
    assert_eq!(validate_frame_magic(&bytes), Err(IpcError::InvalidMagic { actual: 0xFFFF_FFFF }));
}

#[test]
fn validate_frame_magic_rejects_reversed_magic() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x5442_4C56_u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 20]);
    assert_eq!(validate_frame_magic(&bytes), Err(IpcError::InvalidMagic { actual: 0x5442_4C56 }));
}

#[test]
fn validate_frame_magic_rejects_off_by_one_magic() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x5642_4C55_u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 20]);
    assert_eq!(validate_frame_magic(&bytes), Err(IpcError::InvalidMagic { actual: 0x5642_4C55 }));
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
fn validate_frame_magic_rejects_too_short_input() {
    let bytes = [0u8; 3];
    let result = validate_frame_magic(&bytes);
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}
