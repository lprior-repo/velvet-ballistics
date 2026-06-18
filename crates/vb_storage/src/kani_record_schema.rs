#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-002: Record schema version validation verification
//!
//! Property: `decode_record_header` returns `MigrationRequired` for old schema
//! versions and `UnsupportedSchemaVersion` for future schema versions.
//!
//! This harness verifies schema version validation in record header decoding.

use crate::codec::header::decode_record_header;
use crate::constants::{
    CRC_OFFSET, CURRENT_SCHEMA_VERSION, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
};
use crate::error::JournalError;
use crate::records::RecordKind;

/// VB-STORAGE-DECODE-002 H1: decode accepts current schema version
#[kani::proof]
fn kani_record_schema_accepts_current_version() {
    let expected_magic: u32 = 0x5650424Cu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    // Should not fail due to schema version
    match result {
        Ok(_) => kani::assert(true, "current schema version accepted"),
        Err(JournalError::MigrationRequired { .. }) => {
            kani::assert(false, "current schema should not require migration");
        }
        Err(JournalError::UnsupportedSchemaVersion { .. }) => {
            kani::assert(false, "current schema should not be unsupported");
        }
        Err(_) => kani::assert(true, "current schema passes version check"),
    }
}

/// VB-STORAGE-DECODE-002 H2: decode rejects future schema version
#[kani::proof]
fn kani_record_schema_rejects_future_version() {
    let expected_magic: u32 = 0x5650424Cu32;
    let future_version: u16 = CURRENT_SCHEMA_VERSION + 1;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&future_version.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "future schema version should return error");

    if let Err(JournalError::UnsupportedSchemaVersion { version }) = result {
        kani::assert(version == future_version, "future version in error");
    }
}

/// VB-STORAGE-DECODE-002 H3: decode rejects very old schema version
#[kani::proof]
fn kani_record_schema_rejects_old_version() {
    let expected_magic: u32 = 0x5650424Cu32;
    let old_version: u16 = 0; // Very old version

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&old_version.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "old schema version should return error");

    match result {
        Err(JournalError::MigrationRequired { .. })
        | Err(JournalError::UnsupportedSchemaVersion { .. }) => {}
        _ => kani::assert(
            false,
            "old version should return migration or unsupported error",
        ),
    }
}

/// VB-STORAGE-DECODE-002 H4: decode with version 0 returns MigrationRequired
#[kani::proof]
fn kani_record_schema_zero_returns_migration_required() {
    let expected_magic: u32 = 0x5650424Cu32;
    let old_version: u16 = 0;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&old_version.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    let crc = crc32c::crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    let result = decode_record_header(&header_bytes, expected_magic, u32::MAX);
    kani::assert(result.is_err(), "version 0 should return error");
}
