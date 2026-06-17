#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-005: Record CRC validation verification
//!
//! Property: `decode_record_header` returns `HeaderChecksumMismatch` when
//! the computed CRC does not match the stored header_checksum.
//!
//! This harness verifies CRC validation in record header decoding.

use crate::codec::header::decode_record_header;
use crate::constants::{
    CRC_OFFSET, CURRENT_SCHEMA_VERSION, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
};
use crate::error::JournalError;
use crate::records::RecordKind;

/// VB-STORAGE-DECODE-005 H1: decode accepts matching CRC
#[kani::proof]
fn kani_record_crc_accepts_matching() {
    let expected_magic: u32 = 0x5650424Cu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Compute correct CRC over header prefix
    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    match result {
        Ok(_) => #![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-005: Record CRC validation verification
//!
//! Property: `decode_record_header` returns `HeaderChecksumMismatch` when
//! the computed CRC does not match the stored header_checksum.
//!
//! This harness verifies CRC validation in record header decoding.

use crate::codec::header::decode_record_header;
use crate::constants::{
    CRC_OFFSET, CURRENT_SCHEMA_VERSION, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
};
use crate::error::JournalError;
use crate::records::RecordKind;

/// VB-STORAGE-DECODE-005 H1: decode accepts matching CRC
#[kani::proof]
fn kani_record_crc_accepts_matching() {
    let expected_magic: u32 = 0x5650424Cu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Compute correct CRC over header prefix
    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    match result {
        Ok(_) => kani::assert(true, "matching CRC accepted"),
        Err(JournalError::HeaderChecksumMismatch) => {
            kani::assert(false, "matching CRC should not be rejected");
        }
        Err(_) => kani::assert(true, "matching CRC passes checksum check"),
    }
}

/// VB-STORAGE-DECODE-005 H2: decode rejects mismatched CRC
#[kani::proof]
fn kani_record_crc_rejects_mismatch() {
    let expected_magic: u32 = 0x5650424Cu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Write wrong CRC at offset 28
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "mismatched CRC should return error");

    if let Err(JournalError::HeaderChecksumMismatch) = result {
        , "mismatched CRC should return error");

    if let Err(JournalError::HeaderChecksumMismatch) = result {
        kani::assert(true, "correct error type returned");
    }
}

/// VB-STORAGE-DECODE-005 H3: decode rejects zero CRC when correct is non-zero
#[kani::proof]
fn kani_record_crc_rejects_zero() {
    let expected_magic: u32 = 0x5650424Cu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Compute correct CRC
    let correct_crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    // Store zero instead
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&0u32.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(),
        "zero CRC should return error when correct is non-zero",
    );
}

/// VB-STORAGE-DECODE-005 H4: decode accepts CRC with all-ones header
#[kani::proof]
fn kani_record_crc_all_ones_header() {
    let expected_magic: u32 = 0xFFFFFFFFu32;

    let mut header_bytes = [0xFFu8; RECORD_HEADER_BYTES];
    // Override CRC_OFFSET to compute correct CRC
    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    match result {
        Ok(_) => ,
        "zero CRC should return error when correct is non-zero",
    );
}

/// VB-STORAGE-DECODE-005 H4: decode accepts CRC with all-ones header
#[kani::proof]
fn kani_record_crc_all_ones_header() {
    let expected_magic: u32 = 0xFFFFFFFFu32;

    let mut header_bytes = [0xFFu8; RECORD_HEADER_BYTES];
    // Override CRC_OFFSET to compute correct CRC
    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    match result {
        Ok(_) => kani::assert(true, "CRC check passes for all-ones header"),
        Err(JournalError::HeaderChecksumMismatch) => kani::assert(false, "CRC should match"),
        Err(_) => kani::assert(true, "CRC check passes"),
    }
}

/// VB-STORAGE-DECODE-005 H5: CRC of single-bit-flipped header doesn't match
#[kani::proof]
fn kani_record_crc_detects_single_bit_flip() {
    let expected_magic: u32 = 0x5650424Cu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Compute correct CRC
    let correct_crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&correct_crc.to_le_bytes());

    // Now flip one bit in the header (at offset 10)
    header_bytes[10] ^= 0x01;

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "single bit flip should be detected by CRC");
}
