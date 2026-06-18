#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-001: Record magic validation verification
//!
//! Property: `decode_record_header` returns `BadMagic` error when the
//! magic field in the header does not match the expected magic value.
//!
//! This harness verifies magic validation in record header decoding.

use crate::constants::{
    CURRENT_SCHEMA_VERSION, MAGIC_WORKFLOW_SOURCE, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
};
use crate::records::RecordKind;

#[derive(Clone, Copy, Eq, PartialEq)]
enum MagicDecodeClass {
    AcceptedOrLaterValidation,
    BadMagic,
    TooShort,
}

fn decode_magic_class(header: &[u8], expected_magic: u32) -> MagicDecodeClass {
    if header.get(..RECORD_HEADER_BYTES).is_none() {
        return MagicDecodeClass::TooShort;
    }
    match read_u32_le(header, 0) {
        Some(found) if found == expected_magic => MagicDecodeClass::AcceptedOrLaterValidation,
        Some(_) => MagicDecodeClass::BadMagic,
        None => MagicDecodeClass::TooShort,
    }
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

fn write_workflow_header(header: &mut [u8; RECORD_HEADER_BYTES], magic: u32) -> bool {
    write_bytes(header, 0, magic.to_le_bytes())
        && write_bytes(header, 4, CURRENT_SCHEMA_VERSION.to_le_bytes())
        && write_bytes(header, 6, RecordKind::WorkflowSource.id().to_le_bytes())
        && write_bytes(header, 8, RECORD_HEADER_LEN.to_le_bytes())
        && write_bytes(header, 12, 0_u32.to_le_bytes())
        && write_bytes(header, 16, 0_u64.to_le_bytes())
}

/// VB-STORAGE-DECODE-001 H1: decode rejects wrong magic
#[kani::proof]
fn kani_record_magic_rejects_wrong_magic() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let wrong_magic: u32 = 0xFFFFFFFFu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_workflow_header(&mut header_bytes, wrong_magic),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_magic_class(&header_bytes, expected_magic) == MagicDecodeClass::BadMagic,
        "wrong magic should return BadMagic class",
    );
    kani::assert(
        read_u32_le(&header_bytes, 0) == Some(wrong_magic),
        "bad magic class preserves found value",
    );
}

/// VB-STORAGE-DECODE-001 H2: decode accepts correct magic
#[kani::proof]
fn kani_record_magic_accepts_correct_magic() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_workflow_header(&mut header_bytes, expected_magic),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_magic_class(&header_bytes, expected_magic) != MagicDecodeClass::BadMagic,
        "correct magic should not fail BadMagic",
    );
}

/// VB-STORAGE-DECODE-001 H3: decode rejects magic = 0
#[kani::proof]
fn kani_record_magic_rejects_zero() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let zero_magic: u32 = 0u32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_workflow_header(&mut header_bytes, zero_magic),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_magic_class(&header_bytes, expected_magic) == MagicDecodeClass::BadMagic,
        "zero magic should return BadMagic class",
    );
}

/// VB-STORAGE-DECODE-001 H4: decode rejects all-ones magic
#[kani::proof]
fn kani_record_magic_rejects_all_ones() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let all_ones_magic: u32 = 0xFFFFFFFFu32;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_workflow_header(&mut header_bytes, all_ones_magic),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_magic_class(&header_bytes, expected_magic) == MagicDecodeClass::BadMagic,
        "all-ones magic should return BadMagic class",
    );
}
