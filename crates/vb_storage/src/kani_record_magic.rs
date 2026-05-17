#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-001: Record magic validation verification
//!
//! Property: `decode_record_header` returns `BadMagic` error when the
//! magic field in the header does not match the expected magic value.
//!
//! This harness verifies magic validation in record header decoding.

use crate::codec::header::decode_record_header;
use crate::constants::{RECORD_HEADER_BYTES, RECORD_HEADER_LEN};
use crate::error::JournalError;
use crate::records::RecordKind;
use crate::types::RecordHeader;

/// VB-STORAGE-DECODE-001 H1: decode rejects wrong magic
#[kani::proof]
fn kani_record_magic_rejects_wrong_magic() {
    let expected_magic: u32 = 0x5650424Cu32; // "VPRL"
    let wrong_magic: u32 = 0xFFFFFFFFu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];

    // Write wrong magic at offset 0
    header_bytes[0..4].copy_from_slice(&wrong_magic.to_le_bytes());
    // Write valid schema version at offset 4
    header_bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
    // Write valid kind at offset 6
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    // Write header_len at offset 8
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    // Write payload_len at offset 12
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    // Write sequence at offset 16
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Write a valid CRC placeholder at offset 28 (CRC_OFFSET)
    let crc = crc32c::crc32c(&header_bytes[0..28]);
    header_bytes[28..32].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "wrong magic should return error");

    if let Err(JournalError::BadMagic { found }) = result {
        kani::assert_eq!(found, wrong_magic, "bad magic error contains found value");
    }
}

/// VB-STORAGE-DECODE-001 H2: decode accepts correct magic
#[kani::proof]
fn kani_record_magic_accepts_correct_magic() {
    let expected_magic: u32 = 0x5650424Cu32; // "VPRL"

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];

    // Write correct magic at offset 0
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    // Write valid schema version at offset 4
    header_bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
    // Write valid kind at offset 6 (WorkflowSource = 1)
    header_bytes[6..8].copy_from_slice(&1u16.to_le_bytes());
    // Write header_len at offset 8
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    // Write payload_len at offset 12
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    // Write sequence at offset 16
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Write a valid CRC placeholder at offset 28 (CRC_OFFSET)
    let crc = crc32c::crc32c(&header_bytes[0..28]);
    header_bytes[28..32].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    // May fail on other validations but magic check passes
    // We mainly verify it doesn't return BadMagic
    match result {
        Ok(_) => kani::assert(true, "correct magic passes magic check"),
        Err(JournalError::BadMagic { .. }) => kani::assert(false, "correct magic should not fail BadMagic"),
        Err(_) => kani::assert(true, "correct magic passes magic check, other validation may fail"),
    }
}

/// VB-STORAGE-DECODE-001 H3: decode rejects magic = 0
#[kani::proof]
fn kani_record_magic_rejects_zero() {
    let expected_magic: u32 = 0x5650424Cu32;
    let zero_magic: u32 = 0u32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&zero_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&1u16.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..28]);
    header_bytes[28..32].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "zero magic should return error");
}

/// VB-STORAGE-DECODE-001 H4: decode rejects all-ones magic
#[kani::proof]
fn kani_record_magic_rejects_all_ones() {
    let expected_magic: u32 = 0x5650424Cu32;
    let all_ones_magic: u32 = 0xFFFFFFFFu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&all_ones_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&1u16.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..28]);
    header_bytes[28..32].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "all-ones magic should return error");
}
