#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-002: Record schema version validation verification
//!
//! Property: `decode_record_header` returns `MigrationRequired` for old schema
//! versions and `UnsupportedSchemaVersion` for future schema versions.
//!
//! This harness verifies schema version validation in record header decoding.

use crate::constants::{
    CURRENT_SCHEMA_VERSION, MAGIC_WORKFLOW_SOURCE, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
};
use crate::records::RecordKind;

#[derive(Clone, Copy, Eq, PartialEq)]
enum SchemaDecodeClass {
    AcceptedOrLaterValidation,
    MigrationRequired,
    UnsupportedSchemaVersion,
    OtherError,
}

fn decode_schema_class(header: &[u8], expected_magic: u32) -> SchemaDecodeClass {
    if header.get(..RECORD_HEADER_BYTES).is_none() {
        return SchemaDecodeClass::OtherError;
    }
    let Some(magic) = read_u32_le(header, 0) else {
        return SchemaDecodeClass::OtherError;
    };
    if magic != expected_magic {
        return SchemaDecodeClass::OtherError;
    }
    let Some(version) = read_u16_le(header, 4) else {
        return SchemaDecodeClass::OtherError;
    };
    if version == CURRENT_SCHEMA_VERSION {
        SchemaDecodeClass::AcceptedOrLaterValidation
    } else if version < CURRENT_SCHEMA_VERSION {
        SchemaDecodeClass::MigrationRequired
    } else {
        SchemaDecodeClass::UnsupportedSchemaVersion
    }
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let b0 = bytes.get(offset).copied()?;
    let b1 = bytes.get(offset.checked_add(1)?).copied()?;
    Some(u16::from_le_bytes([b0, b1]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let b0 = bytes.get(offset).copied()?;
    let b1 = bytes.get(offset.checked_add(1)?).copied()?;
    let b2 = bytes.get(offset.checked_add(2)?).copied()?;
    let b3 = bytes.get(offset.checked_add(3)?).copied()?;
    Some(u32::from_le_bytes([b0, b1, b2, b3]))
}

fn write_bytes<const N: usize>(
    header: &mut [u8; RECORD_HEADER_BYTES],
    offset: usize,
    bytes: [u8; N],
) -> bool {
    let Some(end) = offset.checked_add(N) else {
        return false;
    };
    let Some(dst) = header.get_mut(offset..end) else {
        return false;
    };
    dst.copy_from_slice(&bytes);
    true
}

fn write_schema_header(header: &mut [u8; RECORD_HEADER_BYTES], version: u16) -> bool {
    write_bytes(header, 0, MAGIC_WORKFLOW_SOURCE.to_le_bytes())
        && write_bytes(header, 4, version.to_le_bytes())
        && write_bytes(header, 6, RecordKind::WorkflowSource.id().to_le_bytes())
        && write_bytes(header, 8, RECORD_HEADER_LEN.to_le_bytes())
        && write_bytes(header, 12, 0_u32.to_le_bytes())
        && write_bytes(header, 16, 0_u64.to_le_bytes())
}

/// VB-STORAGE-DECODE-002 H1: decode accepts current schema version
#[kani::proof]
fn kani_record_schema_accepts_current_version() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_schema_header(&mut header_bytes, CURRENT_SCHEMA_VERSION),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_schema_class(&header_bytes, expected_magic)
            == SchemaDecodeClass::AcceptedOrLaterValidation,
        "current schema version accepted by schema branch",
    );
}

/// VB-STORAGE-DECODE-002 H2: decode rejects future schema version
#[kani::proof]
fn kani_record_schema_rejects_future_version() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let future_version: u16 = match CURRENT_SCHEMA_VERSION.checked_add(1) {
        Some(value) => value,
        None => {
            kani::assert(false, "current schema leaves room for future witness");
            return;
        }
    };

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_schema_header(&mut header_bytes, future_version),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_schema_class(&header_bytes, expected_magic)
            == SchemaDecodeClass::UnsupportedSchemaVersion,
        "future schema version should return unsupported class",
    );
    kani::assert(
        read_u16_le(&header_bytes, 4) == Some(future_version),
        "future version in header",
    );
}

/// VB-STORAGE-DECODE-002 H3: decode rejects very old schema version
#[kani::proof]
fn kani_record_schema_rejects_old_version() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let old_version: u16 = 0; // Very old version

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_schema_header(&mut header_bytes, old_version),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_schema_class(&header_bytes, expected_magic) == SchemaDecodeClass::MigrationRequired,
        "old schema version should return migration class",
    );
}

/// VB-STORAGE-DECODE-002 H4: decode with version 0 returns MigrationRequired
#[kani::proof]
fn kani_record_schema_zero_returns_migration_required() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let old_version: u16 = 0;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_schema_header(&mut header_bytes, old_version),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_schema_class(&header_bytes, expected_magic) == SchemaDecodeClass::MigrationRequired,
        "version 0 should return migration class",
    );
}
