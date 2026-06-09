//! CLI Postcard Error-Path Tests
//!
//! vb-k8ut.5: malformed-header rejection, CRC/digest mismatches,
//! version-too-old/new, wrong-kind rejection, payload-too-large,
//! truncated-header, and garbage-bytes rejection. Each test exercises
//! one negative path of the typed `decode_postcard` / `decode_cli_payload`
//! surface and asserts the matching `PostcardError` variant.

use super::super::*;
use super::{encode_test_postcard, write_test_bytes, write_test_header_prefix};

#[test]
fn test_decode_invalid_magic() {
    let mut data = vec![0u8; HEADER_SIZE + 100];
    write_test_bytes(&mut data, 0..4, &[0x00, 0x00, 0x00, 0x00]);
    write_test_bytes(&mut data, 12..16, &(100u32).to_le_bytes());

    let result = decode_postcard(&data);
    assert_eq!(result, Err(PostcardError::InvalidMagic));
}

#[test]
fn test_decode_payload_too_large() {
    let mut data = vec![0u8; HEADER_SIZE + 100];
    write_test_header_prefix(&mut data, MAX_PAYLOAD_U32.saturating_add(1));

    let result = decode_postcard(&data);
    assert_eq!(result, Err(PostcardError::PayloadTooLarge));
}

#[test]
fn test_decode_invalid_header_length() {
    let mut data = vec![0u8; HEADER_SIZE + 100];
    write_test_bytes(&mut data, 0..4, &CLI_MAGIC);
    write_test_bytes(&mut data, 4..6, &CLI_SCHEMA_VERSION.to_le_bytes());
    write_test_bytes(&mut data, 6..8, &CLI_POSTCARD_KIND.to_le_bytes());
    write_test_bytes(
        &mut data,
        8..12,
        &HEADER_SIZE_U32.saturating_add(1).to_le_bytes(),
    );
    write_test_bytes(&mut data, 12..16, &(100u32).to_le_bytes());

    let result = decode_postcard(&data);
    assert_eq!(result, Err(PostcardError::InvalidHeaderLength));
}

#[test]
fn test_decode_data_too_short() {
    let data = vec![0u8; 10];
    let result = decode_postcard(&data);
    assert_eq!(result, Err(PostcardError::DecodeFailed));
}

#[test]
fn decode_rejects_corrupted_crc_before_exposure() {
    let mut encoded = encode_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload")
        .expect("test postcard encodes");
    assert!(encoded.get(48).is_some());
    if let Some(byte) = encoded.get_mut(48) {
        *byte ^= 0x01;
    }
    assert_eq!(decode_postcard(&encoded), Err(PostcardError::CrcMismatch));
}

#[test]
fn decode_rejects_corrupted_digest_before_exposure() {
    let mut encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
    assert!(encoded.get(16).is_some());
    if let Some(byte) = encoded.get_mut(16) {
        *byte ^= 0x01;
    }
    let crc = encoded.get(0..48).map_or(0, crc32fast::hash);
    write_test_bytes(&mut encoded, 48..52, &crc.to_le_bytes());
    assert_eq!(
        decode_postcard(&encoded),
        Err(PostcardError::DigestMismatch)
    );
}

#[test]
fn decode_rejects_wrong_kind() {
    let encoded = encode_postcard(
        CLI_SCHEMA_VERSION,
        CLI_POSTCARD_KIND.saturating_add(1),
        b"payload",
    )
    .expect("wrong-kind postcard encodes");
    assert_eq!(decode_postcard(&encoded), Err(PostcardError::WrongKind));
}

#[test]
fn decode_rejects_max_plus_one_payload_before_exposure() {
    let mut encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
    write_test_bytes(
        &mut encoded,
        12..16,
        &MAX_PAYLOAD_U32.saturating_add(1).to_le_bytes(),
    );
    let crc = encoded.get(0..48).map_or(0, crc32fast::hash);
    write_test_bytes(&mut encoded, 48..52, &crc.to_le_bytes());
    assert_eq!(
        decode_postcard(&encoded),
        Err(PostcardError::PayloadTooLarge)
    );
}

#[test]
fn decode_rejects_truncated_header() {
    let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
    let truncated = encoded
        .get(0..HEADER_SIZE.saturating_sub(1))
        .map_or(&[][..], |slice| slice);
    assert_eq!(decode_postcard(truncated), Err(PostcardError::DecodeFailed));
}

#[test]
fn decode_cli_payload_rejects_garbage_bytes_as_typed_envelope() {
    let garbage = [0xFFu8; 24];
    let result = decode_cli_payload(&garbage);
    assert_eq!(result, Err(PostcardError::DecodeFailed));
}
