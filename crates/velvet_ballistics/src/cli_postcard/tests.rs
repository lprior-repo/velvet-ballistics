//! CLI Postcard Tests

use super::*;

#[test]
fn test_valid_magic() {
    assert_eq!(CLI_MAGIC, [0x56, 0x43, 0x4C, 0x41]);
    assert_eq!(CLI_MAGIC, *b"VCLA");
}

#[test]
fn test_max_payload() {
    assert_eq!(MAX_PAYLOAD, 65536);
}

#[test]
fn test_header_size() {
    assert_eq!(HEADER_SIZE, 52);
}

#[test]
fn test_postcard_header_from_bytes() {
    let data = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, &[0u8; 100]);

    let header = PostcardHeader::from_bytes(&data).expect("test header decodes");
    assert_eq!(header.magic, CLI_MAGIC);
    assert_eq!(header.schema_version, CLI_SCHEMA_VERSION);
    assert_eq!(header.kind, CLI_POSTCARD_KIND);
    assert_eq!(header.header_len, HEADER_SIZE_U32);
    assert_eq!(header.payload_len, 100);
}

#[test]
fn test_decode_valid_postcard() {
    let data = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, &[0u8; 100]);

    let (header, payload) = decode_postcard(&data).expect("valid postcard decodes");
    assert_eq!(header.len(), HEADER_SIZE);
    assert_eq!(payload.len(), 100);
}

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
fn test_encode_postcard() {
    let payload = b"test payload";
    let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, payload);

    assert_eq!(encoded.get(0..4), Some(CLI_MAGIC.as_slice()));
    assert_eq!(encoded.len(), HEADER_SIZE + payload.len());
}

#[test]
fn test_roundtrip() {
    let payload = b"Hello, Postcard!";
    let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, payload);

    let (header, extracted_payload) = decode_postcard(&encoded).expect("roundtrip decodes");
    assert_eq!(header.len(), HEADER_SIZE);
    assert_eq!(extracted_payload, payload);
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
fn decode_rejects_old_and_future_versions() {
    let old = encode_test_postcard(0, CLI_POSTCARD_KIND, b"payload");
    let future = encode_postcard(
        CLI_SCHEMA_VERSION.saturating_add(1),
        CLI_POSTCARD_KIND,
        b"payload",
    )
    .expect("future-version postcard encodes");
    assert_eq!(decode_postcard(&old), Err(PostcardError::VersionTooOld));
    assert_eq!(decode_postcard(&future), Err(PostcardError::VersionTooNew));
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

fn encode_test_postcard(schema_version: u16, kind: u16, payload: &[u8]) -> Vec<u8> {
    encode_postcard(schema_version, kind, payload).expect("test postcard encodes")
}

fn write_test_header_prefix(data: &mut [u8], payload_len: u32) {
    write_test_bytes(data, 0..4, &CLI_MAGIC);
    write_test_bytes(data, 4..6, &CLI_SCHEMA_VERSION.to_le_bytes());
    write_test_bytes(data, 6..8, &CLI_POSTCARD_KIND.to_le_bytes());
    write_test_bytes(data, 8..12, &HEADER_SIZE_U32.to_le_bytes());
    write_test_bytes(data, 12..16, &payload_len.to_le_bytes());
}

fn write_test_bytes(data: &mut [u8], range: std::ops::Range<usize>, bytes: &[u8]) {
    assert_eq!(range.len(), bytes.len());
    assert!(data.get_mut(range.clone()).is_some());
    if let Some(target) = data.get_mut(range) {
        target.copy_from_slice(bytes);
    }
}
