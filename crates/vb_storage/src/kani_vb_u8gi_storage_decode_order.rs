#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harness for storage header decode-order taxonomy.
//!
//! This harness verifies that `decode_record_header` returns errors in the
//! correct priority order:
//! 1. BadMagic (magic mismatch)
//! 2. UnsupportedSchemaVersion (version too new)
//! 3. UnknownRecordKind (kind not in valid set)
//! 4. RecordKindFamilyMismatch (kind not valid for MAGIC_JOURNAL_EVENT family)
//! 5. HeaderLengthMismatch (header_len != 60)
//! 6. PayloadTooLarge (payload_len > max_payload_len)
//!
//! The harness uses `kani::assume` to isolate each error path, ensuring
//! that only the target error can fire.

use crate::codec::decode_record_header;
use crate::constants::{CURRENT_SCHEMA_VERSION, MAGIC_JOURNAL_EVENT, RECORD_HEADER_LEN};
use crate::error::JournalError;

/// VB-U8GI-STORAGE-DECODE-ORDER-001: BadMagic has highest priority.
#[kani::proof]
#[kani::unwind(10)]
pub fn vb_u8gi_storage_decode_order_bad_magic() {
    let header: [u8; 60] = kani::any();
    let max_payload_len: u32 = kani::any();
    // Generate a magic that is NOT MAGIC_JOURNAL_EVENT
    let bad_magic: u32 = kani::any();
    kani::assume(bad_magic != MAGIC_JOURNAL_EVENT);
    let mut header = header;
    header[0..4].copy_from_slice(&bad_magic.to_le_bytes());
    // Ensure other fields won't cause earlier errors
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes()); // valid version
    header[6..8].copy_from_slice(&10u16.to_le_bytes()); // valid kind for journal
    header[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes()); // valid header_len

    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, max_payload_len);
    kani::assert(matches!(result, Err(JournalError::BadMagic { .. }), "assertion failed"),
        "BadMagic has highest priority",
    );
}

/// VB-U8GI-STORAGE-DECODE-ORDER-002: UnsupportedSchemaVersion is checked after magic.
#[kani::proof]
#[kani::unwind(10)]
pub fn vb_u8gi_storage_decode_order_bad_version() {
    let mut header: [u8; 60] = kani::any();
    let max_payload_len: u32 = kani::any();
    // Set correct magic
    header[0..4].copy_from_slice(&MAGIC_JOURNAL_EVENT.to_le_bytes());
    // Set version > CURRENT_SCHEMA_VERSION
    let bad_version = CURRENT_SCHEMA_VERSION
        .checked_add(1)
        .unwrap_or(CURRENT_SCHEMA_VERSION + 1);
    header[4..6].copy_from_slice(&bad_version.to_le_bytes());
    // Ensure other fields won't cause earlier errors
    header[6..8].copy_from_slice(&10u16.to_le_bytes()); // valid kind for journal
    header[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes()); // valid header_len

    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, max_payload_len);
    kani::assert(matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. }), "assertion failed"),
        "UnsupportedSchemaVersion checked after magic",
    );
}

/// VB-U8GI-STORAGE-DECODE-ORDER-003: UnknownRecordKind for invalid kind values.
#[kani::proof]
#[kani::unwind(10)]
pub fn vb_u8gi_storage_decode_order_unknown_kind() {
    let mut header: [u8; 60] = kani::any();
    let max_payload_len: u32 = kani::any();
    // Set correct magic
    header[0..4].copy_from_slice(&MAGIC_JOURNAL_EVENT.to_le_bytes());
    // Set valid version
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    // Set invalid kind (not in 1|2|3|10..=27|30|40|50)
    let invalid_kind: u16 = kani::any();
    kani::assume(!matches!(invalid_kind, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50));
    header[6..8].copy_from_slice(&invalid_kind.to_le_bytes());
    header[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes()); // valid header_len

    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, max_payload_len);
    kani::assert(matches!(result, Err(JournalError::UnknownRecordKind { .. }), "assertion failed"),
        "UnknownRecordKind for invalid kind values",
    );
}

/// VB-U8GI-STORAGE-DECODE-ORDER-004: RecordKindFamilyMismatch for kind not in 10..=27.
/// This tests kinds 1, 2, 3, 30, 40, 50 which are valid kinds but not valid
/// for MAGIC_JOURNAL_EVENT (only 10..=27 are valid for journal events).
#[kani::proof]
#[kani::unwind(10)]
pub fn vb_u8gi_storage_decode_order_family_mismatch() {
    let mut header: [u8; 60] = kani::any();
    let max_payload_len: u32 = kani::any();
    // Set correct magic
    header[0..4].copy_from_slice(&MAGIC_JOURNAL_EVENT.to_le_bytes());
    // Set valid version
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    // Set kind in 1|2|3|30|40|50 (valid kind but not for journal)
    let kind: u16 = kani::any();
    kani::assume(matches!(kind, 1 | 2 | 3 | 30 | 40 | 50));
    header[6..8].copy_from_slice(&kind.to_le_bytes());
    // Ensure valid header_len
    header[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    // Constrain payload_len to avoid PayloadTooLarge masking the family error
    let payload_len: u32 = kani::any();
    kani::assume(payload_len <= max_payload_len);
    header[12..16].copy_from_slice(&payload_len.to_le_bytes());

    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, max_payload_len);
    kani::assert(matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. }), "assertion failed"),
        "RecordKindFamilyMismatch for kind not in 10..=27",
    );
}

/// VB-U8GI-STORAGE-DECODE-ORDER-005: HeaderLengthMismatch checked after kind family.
#[kani::proof]
#[kani::unwind(10)]
pub fn vb_u8gi_storage_decode_order_bad_header_len() {
    let mut header: [u8; 60] = kani::any();
    let max_payload_len: u32 = kani::any();
    // Set correct magic
    header[0..4].copy_from_slice(&MAGIC_JOURNAL_EVENT.to_le_bytes());
    // Set valid version
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    // Set kind in 10..=27 (valid for journal)
    let kind: u16 = kani::any();
    kani::assume(matches!(kind, 10..=27));
    header[6..8].copy_from_slice(&kind.to_le_bytes());
    // Set invalid header_len
    let bad_header_len: u32 = kani::any();
    kani::assume(bad_header_len != RECORD_HEADER_LEN);
    header[8..12].copy_from_slice(&bad_header_len.to_le_bytes());
    // Constrain payload_len to avoid PayloadTooLarge
    let payload_len: u32 = kani::any();
    kani::assume(payload_len <= max_payload_len);
    header[12..16].copy_from_slice(&payload_len.to_le_bytes());

    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, max_payload_len);
    kani::assert(matches!(result, Err(JournalError::HeaderLengthMismatch { .. }), "assertion failed"),
        "HeaderLengthMismatch checked after kind family",
    );
}

/// VB-U8GI-STORAGE-DECODE-ORDER-006: PayloadTooLarge has lowest priority.
#[kani::proof]
#[kani::unwind(10)]
pub fn vb_u8gi_storage_decode_order_payload_too_large() {
    let mut header: [u8; 60] = kani::any();
    let max_payload_len: u32 = kani::any();
    // Set correct magic
    header[0..4].copy_from_slice(&MAGIC_JOURNAL_EVENT.to_le_bytes());
    // Set valid version
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    // Set kind in 10..=27 (valid for journal)
    let kind: u16 = kani::any();
    kani::assume(matches!(kind, 10..=27));
    header[6..8].copy_from_slice(&kind.to_le_bytes());
    // Set valid header_len
    header[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    // Set oversized payload_len
    let payload_len: u32 = kani::any();
    kani::assume(payload_len > max_payload_len);
    header[12..16].copy_from_slice(&payload_len.to_le_bytes());

    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, max_payload_len);
    kani::assert(matches!(result, Err(JournalError::PayloadTooLarge { .. }), "assertion failed"),
        "PayloadTooLarge has lowest priority",
    );
}
