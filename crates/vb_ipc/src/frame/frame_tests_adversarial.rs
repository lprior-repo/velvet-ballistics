//! IPC frame tests - adversarial attack tests.

use super::{
    decode_frame_header, decode_frame_payload, encode_frame,
    IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes, IPC_HEADER_LEN, IPC_VERSION,
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
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..].fill(0xFF);
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 0xFFFF }));
}

#[test]
fn adversarial_unsupported_version_two_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&2u16.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 2 }));
}

#[test]
fn adversarial_unknown_command_id_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&200u16.to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnknownCommand(200)));
}

#[test]
fn adversarial_command_id_zero_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnknownCommand(0)));
}

#[test]
fn adversarial_command_id_max_u16_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&u16::MAX.to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::UnknownCommand(u16::MAX)));
}

#[test]
fn adversarial_nonzero_reserved_field_rejected() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&crate::IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    header_bytes[8..10].copy_from_slice(&0u16.to_le_bytes());
    header_bytes[10..12].copy_from_slice(&1u16.to_le_bytes());
    let result = decode_frame_header(&header_bytes);
    assert_eq!(result, Err(IpcError::ReservedNonZero { actual: 1 }));
}

#[test]
fn adversarial_payload_len_4gb_rejected_as_too_large() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, u32::MAX);
    let result = super::validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);
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
    let result = super::validate_frame_bounds(&header, MaxPayloadBytes::DEFAULT);
    assert_eq!(result, Err(IpcError::PayloadTooLarge { actual: default_max.saturating_add(1), limit: default_max }));
}
