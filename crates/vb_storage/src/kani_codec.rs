#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses proving `decode_record_header` never panics on hostile input.
//!
//! These proofs verify that `decode_record_header` returns an error (rather
//! than panicking) when given malformed or truncated data.

use crate::{
    codec::decode_record_header,
    constants::{CRC_OFFSET, CURRENT_SCHEMA_VERSION, RECORD_HEADER_BYTES},
    error::JournalError,
};

const MAX_PAYLOAD_LEN: u32 = 1024;
const EXPECTED_MAGIC: u32 = 0x5642_4A45;

fn harness_for_length(header_len: usize) {
    kani::cover!(true, "harness_for_length called");
}

#[kani::proof]
fn kani_truncated_header_zero_bytes() {
    harness_for_length(0);
    let header: &[u8] = &[];
    let result = decode_record_header(header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, JournalError::UnexpectedEof));
}

#[kani::proof]
fn kani_truncated_header_30_bytes() {
    harness_for_length(30);
    let header: [u8; 30] = kani::any();
    let result = decode_record_header(&header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, JournalError::UnexpectedEof));
}

#[kani::proof]
fn kani_truncated_header_59_bytes() {
    harness_for_length(59);
    let header: [u8; 59] = kani::any();
    let result = decode_record_header(&header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, JournalError::UnexpectedEof));
}

#[kani::proof]
fn kani_bad_magic_bytes() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    header[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
    let result = decode_record_header(&header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, JournalError::BadMagic { .. }));
}

#[kani::proof]
fn kani_wrong_magic_any_value() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    let wrong_magic: u32 = kani::any();
    kani::assume(wrong_magic != EXPECTED_MAGIC);
    header[0..4].copy_from_slice(&wrong_magic.to_le_bytes());
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&10u16.to_le_bytes());
    header[8..12].copy_from_slice(&60u32.to_le_bytes());
    header[12..16].copy_from_slice(&0u32.to_le_bytes());
    let checksum = crc32c::crc32c(&header[..CRC_OFFSET]);
    header[CRC_OFFSET..CRC_OFFSET.saturating_add(4)]
        .copy_from_slice(&checksum.to_le_bytes());
    let result = decode_record_header(&header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, JournalError::BadMagic { .. }));
}

#[kani::proof]
fn kani_future_schema_version() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    header[0..4].copy_from_slice(&EXPECTED_MAGIC.to_le_bytes());
    let future_version = CURRENT_SCHEMA_VERSION.saturating_add(1);
    header[4..6].copy_from_slice(&future_version.to_le_bytes());
    header[6..8].copy_from_slice(&10u16.to_le_bytes());
    header[8..12].copy_from_slice(&60u32.to_le_bytes());
    header[12..16].copy_from_slice(&0u32.to_le_bytes());
    let checksum = crc32c::crc32c(&header[..CRC_OFFSET]);
    header[CRC_OFFSET..CRC_OFFSET.saturating_add(4)]
        .copy_from_slice(&checksum.to_le_bytes());
    let result = decode_record_header(&header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, JournalError::UnsupportedSchemaVersion { .. }));
}

#[kani::proof]
fn kani_past_schema_version() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    header[0..4].copy_from_slice(&EXPECTED_MAGIC.to_le_bytes());
    let past_version: u16 = kani::any();
    kani::assume(past_version < CURRENT_SCHEMA_VERSION);
    header[4..6].copy_from_slice(&past_version.to_le_bytes());
    header[6..8].copy_from_slice(&10u16.to_le_bytes());
    header[8..12].copy_from_slice(&60u32.to_le_bytes());
    header[12..16].copy_from_slice(&0u32.to_le_bytes());
    let checksum = crc32c::crc32c(&header[..CRC_OFFSET]);
    header[CRC_OFFSET..CRC_OFFSET.saturating_add(4)]
        .copy_from_slice(&checksum.to_le_bytes());
    let result = decode_record_header(&header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, JournalError::MigrationRequired { .. }));
}

#[kani::proof]
fn kani_bad_crc() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    header[0..4].copy_from_slice(&EXPECTED_MAGIC.to_le_bytes());
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&10u16.to_le_bytes());
    header[8..12].copy_from_slice(&60u32.to_le_bytes());
    header[12..16].copy_from_slice(&0u32.to_le_bytes());
    let good_checksum = crc32c::crc32c(&header[..CRC_OFFSET]);
    let bad_checksum = good_checksum.wrapping_add(1);
    header[CRC_OFFSET..CRC_OFFSET.saturating_add(4)]
        .copy_from_slice(&bad_checksum.to_le_bytes());
    let result = decode_record_header(&header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, JournalError::HeaderChecksumMismatch));
}

#[kani::proof]
fn kani_arbitrary_header_60_bytes() {
    let header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    let result = decode_record_header(&header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    match result {
        Ok(_) => {
            kani::cover!(true, "decode succeeded");
        }
        Err(JournalError::UnexpectedEof) => {
            kani::cover!(true, "UnexpectedEof");
        }
        Err(JournalError::BadMagic { .. }) => {
            kani::cover!(true, "BadMagic");
        }
        Err(JournalError::UnsupportedSchemaVersion { .. }) => {
            kani::cover!(true, "UnsupportedSchemaVersion");
        }
        Err(JournalError::MigrationRequired { .. }) => {
            kani::cover!(true, "MigrationRequired");
        }
        Err(JournalError::HeaderChecksumMismatch) => {
            kani::cover!(true, "HeaderChecksumMismatch");
        }
        Err(_) => {
            kani::cover!(true, "other error");
        }
    }
}

#[kani::proof]
fn kani_decode_header_exhaustive_error_coverage() {
    let header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    let result = decode_record_header(&header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN);
    kani::cover!(
        result.is_ok(),
        "decode_record_header returns Ok on arbitrary 60 bytes"
    );
    kani::cover!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "decode_record_header returns UnexpectedEof"
    );
    kani::cover!(
        matches!(result, Err(JournalError::BadMagic { .. })),
        "decode_record_header returns BadMagic"
    );
    kani::cover!(
        matches!(result, Err(JournalError::HeaderChecksumMismatch)),
        "decode_record_header returns HeaderChecksumMismatch"
    );
}
