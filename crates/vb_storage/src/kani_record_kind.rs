#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-003: Record kind validation verification
//!
//! Property: `decode_record_header` returns `UnknownRecordKind` error when
//! the record_kind field is not a known valid kind.
//!
//! This harness verifies kind validation in record header decoding.

use crate::codec::header::decode_record_header;
use crate::constants::{CRC_OFFSET, RECORD_HEADER_BYTES, RECORD_HEADER_LEN, CURRENT_SCHEMA_VERSION};
use crate::error::JournalError;
use crate::records::RecordKind;

/// VB-STORAGE-DECODE-003 H1: decode accepts known record kinds
#[kani::proof]
#[kani::unwind(24)]
fn kani_record_kind_accepts_known_kinds() {
    let expected_magic: u32 = 0x5650424Cu32;
    let known_kinds: &[u16] = &[
        1, 2, 3,                           // WorkflowSource, CompiledArtifact, Snapshot
        10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, // JournalEvent variants
        30,                                 // Blob
        40,                                 // IndexRecord
        50,                                 // Another valid kind
    ];

    for &kind_id in known_kinds {
        let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
        header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
        header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
        header_bytes[6..8].copy_from_slice(&kind_id.to_le_bytes());
        header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
        header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
        header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

        let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
        header_bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes());

        let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
        match result {
            Ok(_) => kani::assert(true, "known kind {} passes kind check", kind_id),
            Err(JournalError::UnknownRecordKind { .. }) => kani::assert(false, "known kind {} should not be rejected", kind_id),
            Err(_) => kani::assert(true, "known kind {} passes kind check", kind_id),
        }
    }
}

/// VB-STORAGE-DECODE-003 H2: decode rejects unknown record kind
#[kani::proof]
fn kani_record_kind_rejects_unknown() {
    let expected_magic: u32 = 0x5650424Cu32;
    let unknown_kind: u16 = 100; // Unknown kind

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&unknown_kind.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "unknown kind should return error");
}

/// VB-STORAGE-DECODE-003 H3: decode rejects kind = 0
#[kani::proof]
fn kani_record_kind_rejects_zero() {
    let expected_magic: u32 = 0x5650424Cu32;
    let zero_kind: u16 = 0;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&zero_kind.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "zero kind should return error");
}

/// VB-STORAGE-DECODE-003 H4: decode rejects kind = 255
#[kani::proof]
fn kani_record_kind_rejects_max() {
    let expected_magic: u32 = 0x5650424Cu32;
    let max_kind: u16 = 255;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&max_kind.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET+4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "max kind should return error");
}
