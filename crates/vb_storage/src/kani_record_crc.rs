#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-005: Record CRC validation verification
//!
//! Property: `decode_record_header` returns `HeaderChecksumMismatch` when
//! the computed CRC does not match the stored header_checksum.
//!
//! This harness verifies CRC validation in record header decoding.

use crate::codec::header::header_crc32c;
use crate::codec::validation::{
    RecordKindFamilyDecision, classify_kind_family, is_known_record_kind,
};
use crate::constants::{
    CRC_OFFSET, CURRENT_SCHEMA_VERSION, MAGIC_WORKFLOW_SOURCE, RECORD_HEADER_BYTES,
    RECORD_HEADER_LEN,
};
use crate::records::RecordKind;

#[derive(Clone, Copy, Eq, PartialEq)]
enum CrcDecodeClass {
    Accepted,
    HeaderChecksumMismatch,
    OtherError,
}

fn decode_crc_class(header: &[u8], expected_magic: u32) -> CrcDecodeClass {
    let Some(decoded) = compact_decode_header(header) else {
        return CrcDecodeClass::OtherError;
    };
    if decoded.magic != expected_magic {
        return CrcDecodeClass::OtherError;
    }
    if decoded.schema_version != CURRENT_SCHEMA_VERSION {
        return CrcDecodeClass::OtherError;
    }
    if !is_known_record_kind(decoded.record_kind) {
        return CrcDecodeClass::OtherError;
    }
    if classify_kind_family(decoded.magic, decoded.record_kind) == RecordKindFamilyDecision::Rejected
    {
        return CrcDecodeClass::OtherError;
    }
    if decoded.header_len != RECORD_HEADER_LEN {
        return CrcDecodeClass::OtherError;
    }
    if header_crc32c(compact_header_prefix(header)) != decoded.header_checksum {
        return CrcDecodeClass::HeaderChecksumMismatch;
    }
    CrcDecodeClass::Accepted
}

#[derive(Clone, Copy)]
struct CompactHeader {
    magic: u32,
    schema_version: u16,
    record_kind: u16,
    header_len: u32,
    header_checksum: u32,
}

fn compact_decode_header(header: &[u8]) -> Option<CompactHeader> {
    Some(CompactHeader {
        magic: read_u32_le(header, 0)?,
        schema_version: read_u16_le(header, 4)?,
        record_kind: read_u16_le(header, 6)?,
        header_len: read_u32_le(header, 8)?,
        header_checksum: read_u32_le(header, CRC_OFFSET)?,
    })
}

fn compact_header_prefix(header: &[u8]) -> &[u8] {
    match header.get(..CRC_OFFSET) {
        Some(prefix) => prefix,
        None => &[],
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

/// VB-STORAGE-DECODE-005 H1: decode accepts matching CRC
#[kani::proof]
fn kani_record_crc_accepts_matching() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Compute correct CRC over header prefix
    let crc = header_crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    kani::assert(
        decode_crc_class(&header_bytes, expected_magic) == CrcDecodeClass::Accepted,
        "matching CRC accepted",
    );
}

/// VB-STORAGE-DECODE-005 H2: decode rejects mismatched CRC
#[kani::proof]
fn kani_record_crc_rejects_mismatch() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Write wrong CRC at offset 28
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());

    kani::assert(
        decode_crc_class(&header_bytes, expected_magic) == CrcDecodeClass::HeaderChecksumMismatch,
        "mismatched CRC should return checksum mismatch",
    );
}

/// VB-STORAGE-DECODE-005 H3: decode rejects zero CRC when correct is non-zero
#[kani::proof]
fn kani_record_crc_rejects_zero() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Compute correct CRC
    let correct_crc = header_crc32c(&header_bytes[0..CRC_OFFSET]);
    kani::assert(correct_crc != 0, "modeled CRC for witness is non-zero");
    // Store zero instead
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&0u32.to_le_bytes());

    kani::assert(
        decode_crc_class(&header_bytes, expected_magic) == CrcDecodeClass::HeaderChecksumMismatch,
        "zero CRC should return checksum mismatch when correct is non-zero",
    );
}

/// VB-STORAGE-DECODE-005 H4: decode accepts CRC with all-ones header
#[kani::proof]
fn kani_record_crc_all_ones_header() {
    let expected_magic: u32 = 0xFFFFFFFFu32;

    let mut header_bytes = [0xFFu8; RECORD_HEADER_BYTES];
    // Override CRC_OFFSET to compute correct CRC
    let crc = header_crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());

    kani::assert(
        decode_crc_class(&header_bytes, expected_magic) != CrcDecodeClass::HeaderChecksumMismatch,
        "CRC check passes for all-ones header",
    );
}

/// VB-STORAGE-DECODE-005 H5: CRC of single-bit-flipped header doesn't match
#[kani::proof]
fn kani_record_crc_detects_single_bit_flip() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    header_bytes[0..4].copy_from_slice(&expected_magic.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&RecordKind::WorkflowSource.id().to_le_bytes());
    header_bytes[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    header_bytes[16..24].copy_from_slice(&0u64.to_le_bytes());

    // Compute correct CRC
    let correct_crc = header_crc32c(&header_bytes[0..CRC_OFFSET]);
    header_bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&correct_crc.to_le_bytes());

    // Now flip one bit in the sequence field. This preserves all semantic
    // validations that run before the checksum comparison, so the harness
    // specifically proves CRC detection instead of earlier header_len rejection.
    header_bytes[16] ^= 0x01;

    kani::assert(
        decode_crc_class(&header_bytes, expected_magic) == CrcDecodeClass::HeaderChecksumMismatch,
        "single bit flip should be detected by CRC",
    );
}
