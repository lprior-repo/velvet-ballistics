#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-004: Record payload_len validation verification
//!
//! Property: `decode_record_header` validates payload_len against max_payload_len
//! and returns `PayloadTooLarge` when exceeded.
//!
//! This harness verifies payload length validation in record header decoding.

use crate::codec::header::decode_record_header;
use crate::constants::{CRC_OFFSET, RECORD_HEADER_BYTES, RECORD_HEADER_LEN, CURRENT_SCHEMA_VERSION};
use crate::error::JournalError;
use crate::records::RecordKind;

/// VB-STORAGE-DECODE-004 H1: decode accepts payload_len within max
#[kani::proof]
fn kani_record_payload_len_within_max() {
    let expected_magic: u32 = 0x5650424Cu32;
    let max_payload: u32 = 1024;
    let payload_len: u32 = 512; // Within max

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&payload_len.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, max_payload);
    match result {
        Ok(_) => kani::assert(true, "payload within max accepted"),
        Err(JournalError::PayloadTooLarge { .. }) => kani::assert(false, "payload within max should not be rejected"),
        Err(_) => kani::assert(true, "payload within max passes length check"),
    }
}

/// VB-STORAGE-DECODE-004 H2: decode rejects payload_len exceeding max
#[kani::proof]
fn kani_record_payload_len_exceeds_max() {
    let expected_magic: u32 = 0x5650424Cu32;
    let max_payload: u32 = 1024;
    let payload_len: u32 = 2048; // Exceeds max

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&payload_len.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, max_payload);
    kani::assert(result.is_err(), "payload exceeding max should return error");

    if let Err(JournalError::PayloadTooLarge { len, max }) = result {
        kani::assert(len == payload_len, "len matches");
        kani::assert(max == max_payload, "max matches");
    }
}

/// VB-STORAGE-DECODE-004 H3: decode rejects payload_len exactly at max + 1
#[kani::proof]
fn kani_record_payload_len_exactly_over_max() {
    let expected_magic: u32 = 0x5650424Cu32;
    let max_payload: u32 = 100;
    let payload_len: u32 = 101; // Exactly over

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&payload_len.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, max_payload);
    kani::assert(result.is_err(), "payload exactly over max should return error");
}

/// VB-STORAGE-DECODE-004 H4: decode accepts payload_len exactly at max
#[kani::proof]
fn kani_record_payload_len_exactly_at_max() {
    let expected_magic: u32 = 0x5650424Cu32;
    let max_payload: u32 = 100;
    let payload_len: u32 = 100; // Exactly at max

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&payload_len.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, max_payload);
    match result {
        Ok(_) => kani::assert(true, "payload at max accepted"),
        Err(JournalError::PayloadTooLarge { .. }) => kani::assert(false, "payload at max should not be rejected"),
        Err(_) => kani::assert(true, "payload at max passes length check"),
    }
}

/// VB-STORAGE-DECODE-004 H5: decode with zero max_payload rejects non-zero payload
#[kani::proof]
fn kani_record_payload_len_rejects_nonzero_when_max_zero() {
    let expected_magic: u32 = 0x5650424Cu32;
    let max_payload: u32 = 0;
    let payload_len: u32 = 1;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&payload_len.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, max_payload);
    kani::assert(result.is_err(), "non-zero payload with zero max should return error");
}
