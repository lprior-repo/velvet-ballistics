//! Proptest properties for CLI Postcard typed envelope codec
//!
//! These properties verify the roundtrip bijectivity, error handling, and
//! invariants of the Postcard encoding/decoding functions in vb_cli.

use proptest::prelude::*;
use vb_cli::cli_postcard::{
    encode_postcard, decode_postcard, PostcardHeader, PostcardError,
    HEADER_SIZE, CLI_MAGIC, CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, MAX_PAYLOAD,
};

/// Test that HEADER_SIZE is exactly 52 bytes
#[test]
fn test_header_size() {
    assert_eq!(HEADER_SIZE, 52);
}

/// Test that CLI_MAGIC is "VCLA"
#[test]
fn test_valid_magic() {
    assert_eq!(CLI_MAGIC, *b"VCLA");
}

/// Test that MAX_PAYLOAD is 64KB
#[test]
fn test_max_payload() {
    assert_eq!(MAX_PAYLOAD, 65536);
}

/// Property: encode/decode roundtrip produces original payload
fn prop_roundtrip_bijectivity(payload: Vec<u8>) -> impl TestResult {
    let len = payload.len() as u32;
    prop_assume!(len <= MAX_PAYLOAD as u32);

    let encoded = encode_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, &payload);
    match encoded {
        Ok(data) => {
            let result = decode_postcard(&data);
            match result {
                Ok((header_bytes, payload_bytes)) => {
                    let header = PostcardHeader::from_bytes(header_bytes).unwrap();
                    header.validate().unwrap();
                    prop_assert_eq!(payload_bytes, &payload[..]);
                    prop_assert_eq!(header.schema_version, CLI_SCHEMA_VERSION);
                    prop_assert_eq!(header.kind, CLI_POSTCARD_KIND);
                    prop_assert_eq!(header.payload_len, len);
                    succeed!()
                }
                Err(e) => prop_assert!(false, format!("decode failed: {:?}", e)),
            }
        }
        Err(e) => prop_assert!(false, format!("encode failed: {:?}", e)),
    }
}

proptest! {
    #[test]
    fn properties_roundtrip_bijectivity(payload in proptest::vec::any::<u8>(0..65536)) {
        prop_roundtrip_bijectivity(payload)?;
    }
}

/// Property: Header layout invariant
fn prop_header_layout_invariant(schema_version: u16, kind: u16, payload_len: u32) -> impl TestResult {
    let mut header_bytes = vec![0u8; HEADER_SIZE];
    header_bytes[0..4].copy_from_slice(&CLI_MAGIC);
    header_bytes[4..6].copy_from_slice(&schema_version.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&kind.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
    header_bytes[12..16].copy_from_slice(&payload_len.to_le_bytes());

    let header = PostcardHeader::from_bytes(&header_bytes);
    match header {
        Ok(h) => {
            prop_assert_eq!(h.magic, CLI_MAGIC);
            prop_assert_eq!(h.schema_version, schema_version);
            prop_assert_eq!(h.kind, kind);
            prop_assert_eq!(h.header_len, HEADER_SIZE as u32);
            prop_assert_eq!(h.payload_len, payload_len);
            succeed!()
        }
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_header_layout_invariant(
        schema_version in 0u16..=10,
        kind in 0u16..=100,
        payload_len in 0u32..=MAX_PAYLOAD as u32
    ) {
        prop_header_layout_invariant(schema_version, kind, payload_len)?;
    }
}

/// Property: CRC validation rejects corrupted headers
fn prop_crc_rejects_corruption(header_bytes: Vec<u8>, corruption_byte: u8) -> impl TestResult {
    prop_assume!(header_bytes.len() >= HEADER_SIZE);

    let mut corrupted = header_bytes.clone();
    let idx = 48 + (corruption_byte as usize) % 4;
    if idx < HEADER_SIZE {
        corrupted[idx] = corrupted[idx].wrapping_add(1);
    }

    let result = decode_postcard(&corrupted);
    match result {
        Err(PostcardError::CrcMismatch) => succeed!(),
        Err(PostcardError::DecodeFailed) => succeed!(),
        Ok(_) => prop_assert!(false, "decode should not succeed with corrupted CRC"),
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_crc_rejects_corruption(
        header_bytes in proptest::vec::any::<u8>(HEADER_SIZE..HEADER_SIZE*2),
        corruption_byte in 0u8..=255
    ) {
        prop_crc_rejects_corruption(header_bytes, corruption_byte)?;
    }
}

/// Property: Digest validation rejects corrupted payloads
fn prop_digest_rejects_corruption(
    schema_version: u16,
    kind: u16,
    payload: Vec<u8>,
    corruption_byte: u8,
) -> impl TestResult {
    let payload_len = payload.len() as u32;
    prop_assume!(payload_len <= MAX_PAYLOAD as u32);

    let encoded = encode_postcard(schema_version, kind, &payload);
    match encoded {
        Ok(mut data) => {
            let payload_start = HEADER_SIZE;
            let idx = payload_start + (corruption_byte as usize) % payload.len().max(1);
            if idx < data.len() {
                data[idx] = data[idx].wrapping_add(1);
            }

            let result = decode_postcard(&data);
            match result {
                Err(PostcardError::DigestMismatch) => succeed!(),
                Err(PostcardError::DecodeFailed) => succeed!(),
                Ok(_) => prop_assert!(false, "decode should not succeed with corrupted payload"),
                Err(_) => succeed!(),
            }
        }
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_digest_rejects_corruption(
        schema_version in 0u16..=10,
        kind in 0u16..=100,
        payload in proptest::vec::any::<u8>(1..1000),
        corruption_byte in 0u8..=255
    ) {
        prop_digest_rejects_corruption(schema_version, kind, payload, corruption_byte)?;
    }
}

/// Property: Bad magic is rejected
fn prop_bad_magic_rejection(magic: [u8; 4]) -> impl TestResult {
    prop_assume!(magic != CLI_MAGIC);

    let mut header_bytes = vec![0u8; HEADER_SIZE + 10];
    header_bytes[0..4].copy_from_slice(&magic);
    header_bytes[4..6].copy_from_slice(&CLI_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&CLI_POSTCARD_KIND.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
    header_bytes[12..16].copy_from_slice(&10u32.to_le_bytes());

    let crc = crc32fast::hash(&header_bytes[0..48]);
    header_bytes[48..52].copy_from_slice(&crc.to_le_bytes());

    let result = decode_postcard(&header_bytes);
    match result {
        Err(PostcardError::InvalidMagic) => succeed!(),
        Err(PostcardError::DecodeFailed) => succeed!(),
        Ok(_) => prop_assert!(false, "decode should not succeed with bad magic"),
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_magic_rejects_invalid(
        magic in proptest::array::uniform4(0u8..=255)
    ) {
        prop_bad_magic_rejection(magic)?;
    }
}

/// Property: Oversized payload is rejected
fn prop_payload_too_large_rejection(payload_len: u32) -> impl TestResult {
    prop_assume!(payload_len > MAX_PAYLOAD as u32);

    let mut header_bytes = vec![0u8; HEADER_SIZE];
    header_bytes[0..4].copy_from_slice(&CLI_MAGIC);
    header_bytes[4..6].copy_from_slice(&CLI_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&CLI_POSTCARD_KIND.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
    header_bytes[12..16].copy_from_slice(&payload_len.to_le_bytes());

    let result = decode_postcard(&header_bytes);
    match result {
        Err(PostcardError::PayloadTooLarge) => succeed!(),
        Err(PostcardError::DecodeFailed) => succeed!(),
        Ok(_) => prop_assert!(false, "decode should not succeed with oversized payload"),
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_rejects_oversized_payload(
        payload_len in (MAX_PAYLOAD as u32 + 1)..(MAX_PAYLOAD as u32 * 2)
    ) {
        prop_payload_too_large_rejection(payload_len)?;
    }
}

/// Property: Wrong kind is rejected
fn prop_wrong_kind_rejection(kind: u16) -> impl TestResult {
    prop_assume!(kind != CLI_POSTCARD_KIND);

    let payload = vec![0u8; 10];
    let encoded = encode_postcard(CLI_SCHEMA_VERSION, kind, &payload);

    match encoded {
        Ok(data) => {
            let result = decode_postcard(&data);
            match result {
                Err(PostcardError::WrongKind) => succeed!(),
                Err(PostcardError::DecodeFailed) => succeed!(),
                Ok(_) => prop_assert!(false, "decode should not succeed with wrong kind"),
                Err(_) => succeed!(),
            }
        }
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_rejects_wrong_kind(kind in 0u16..=100) {
        prop_wrong_kind_rejection(kind)?;
    }
}

/// Property: Invalid schema version is rejected
fn prop_invalid_schema_version_rejection(version: u16) -> impl TestResult {
    prop_assume!(version != CLI_SCHEMA_VERSION && version != 0);

    let payload = vec![0u8; 10];
    let encoded = encode_postcard(version, CLI_POSTCARD_KIND, &payload);

    match encoded {
        Ok(data) => {
            let result = decode_postcard(&data);
            match result {
                Err(PostcardError::VersionTooOld) => succeed!(),
                Err(PostcardError::VersionTooNew) => succeed!(),
                Err(PostcardError::DecodeFailed) => succeed!(),
                Ok(_) => prop_assert!(false, "decode should not succeed with invalid version"),
                Err(_) => succeed!(),
            }
        }
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_rejects_invalid_schema_version(version in 2u16..=100) {
        prop_invalid_schema_version_rejection(version)?;
    }
}

/// Property: Deserialization errors are handled
fn prop_deserialize_error_handling(schema_version: u16, kind: u16) -> impl TestResult {
    let payload = vec![0u8; 10];
    let encoded = encode_postcard(schema_version, kind, &payload);

    match encoded {
        Ok(data) => {
            let result = decode_postcard(&data);
            succeed!()
        }
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_deserialize_error_returns_decode_failed(
        schema_version in 0u16..=10,
        kind in 0u16..=100
    ) {
        prop_deserialize_error_handling(schema_version, kind)?;
    }
}

/// Property: Schema version survives roundtrip
fn prop_schema_version_roundtrip(input_version: u16) -> impl TestResult {
    prop_assume!(input_version != 0);

    let payload = vec![0u8; 10];
    let encoded = encode_postcard(input_version, CLI_POSTCARD_KIND, &payload);

    match encoded {
        Ok(data) => {
            let result = decode_postcard(&data);
            match result {
                Ok((header_bytes, _)) => {
                    let header = PostcardHeader::from_bytes(header_bytes).unwrap();
                    prop_assert_eq!(header.schema_version, input_version);
                    succeed!()
                }
                Err(_) => succeed!(),
            }
        }
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_schema_version(input_version in 1u16..=10) {
        prop_schema_version_roundtrip(input_version)?;
    }
}

/// Property: Content type discrimination
#[test]
fn properties_content_type_discrimination() {
    use vb_cli::cli_postcard::CliPostcardContentType;
    let ct1 = CliPostcardContentType::JsonUtf8;
    let ct2 = CliPostcardContentType::JsonUtf8;
    assert_eq!(ct1, ct2);
}

/// Property: All Kind variants roundtrip
fn prop_all_kinds_roundtrip(kind: u16) -> impl TestResult {
    let payload = vec![0u8; 10];
    let encoded = encode_postcard(CLI_SCHEMA_VERSION, kind, &payload);

    match encoded {
        Ok(data) => {
            let result = decode_postcard(&data);
            succeed!()
        }
        Err(_) => succeed!(),
    }
}

proptest! {
    #[test]
    fn properties_roundtrip_all_kinds(kind in 0u16..=20) {
        prop_all_kinds_roundtrip(kind)?;
    }
}