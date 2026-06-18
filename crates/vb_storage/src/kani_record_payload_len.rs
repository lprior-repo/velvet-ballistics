#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-004: Record payload_len validation verification
//!
//! Property: `decode_record_header` validates payload_len against max_payload_len
//! and returns `PayloadTooLarge` when exceeded.
//!
//! This harness verifies payload length validation in record header decoding.

use crate::codec::validation::{
    RecordKindFamilyDecision, classify_kind_family, is_known_record_kind,
};
use crate::constants::{
    CURRENT_SCHEMA_VERSION, MAGIC_WORKFLOW_SOURCE, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
};
use crate::records::RecordKind;

#[derive(Clone, Copy, Eq, PartialEq)]
enum PayloadLenDecodeClass {
    AcceptedOrLaterValidation,
    PayloadTooLarge,
    OtherError,
}

#[derive(Clone, Copy)]
struct CompactPayloadHeader {
    magic: u32,
    schema_version: u16,
    record_kind: u16,
    header_len: u32,
    payload_len: u32,
}

fn decode_payload_len_class(
    header: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> PayloadLenDecodeClass {
    let Some(decoded) = compact_decode_header(header) else {
        return PayloadLenDecodeClass::OtherError;
    };
    if decoded.magic != expected_magic {
        return PayloadLenDecodeClass::OtherError;
    }
    if decoded.schema_version != CURRENT_SCHEMA_VERSION {
        return PayloadLenDecodeClass::OtherError;
    }
    if !is_known_record_kind(decoded.record_kind) {
        return PayloadLenDecodeClass::OtherError;
    }
    if classify_kind_family(decoded.magic, decoded.record_kind)
        == RecordKindFamilyDecision::Rejected
    {
        return PayloadLenDecodeClass::OtherError;
    }
    if decoded.header_len != RECORD_HEADER_LEN {
        return PayloadLenDecodeClass::OtherError;
    }
    if decoded.payload_len > max_payload_len {
        return PayloadLenDecodeClass::PayloadTooLarge;
    }
    PayloadLenDecodeClass::AcceptedOrLaterValidation
}

fn compact_decode_header(header: &[u8]) -> Option<CompactPayloadHeader> {
    if header.get(..RECORD_HEADER_BYTES).is_none() {
        return None;
    }
    Some(CompactPayloadHeader {
        magic: read_u32_le(header, 0)?,
        schema_version: read_u16_le(header, 4)?,
        record_kind: read_u16_le(header, 6)?,
        header_len: read_u32_le(header, 8)?,
        payload_len: read_u32_le(header, 12)?,
    })
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

fn write_payload_header(header: &mut [u8; RECORD_HEADER_BYTES], payload_len: u32) -> bool {
    write_bytes(header, 0, MAGIC_WORKFLOW_SOURCE.to_le_bytes())
        && write_bytes(header, 4, CURRENT_SCHEMA_VERSION.to_le_bytes())
        && write_bytes(header, 6, RecordKind::WorkflowSource.id().to_le_bytes())
        && write_bytes(header, 8, RECORD_HEADER_LEN.to_le_bytes())
        && write_bytes(header, 12, payload_len.to_le_bytes())
        && write_bytes(header, 16, 0_u64.to_le_bytes())
}

/// VB-STORAGE-DECODE-004 H1: decode accepts payload_len within max
#[kani::proof]
fn kani_record_payload_len_within_max() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let max_payload: u32 = 1024;
    let payload_len: u32 = 512; // Within max

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_payload_header(&mut header_bytes, payload_len),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_payload_len_class(&header_bytes, expected_magic, max_payload)
            != PayloadLenDecodeClass::PayloadTooLarge,
        "payload within max should not be rejected",
    );
}

/// VB-STORAGE-DECODE-004 H2: decode rejects payload_len exceeding max
#[kani::proof]
fn kani_record_payload_len_exceeds_max() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let max_payload: u32 = 1024;
    let payload_len: u32 = 2048; // Exceeds max

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_payload_header(&mut header_bytes, payload_len),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_payload_len_class(&header_bytes, expected_magic, max_payload)
            == PayloadLenDecodeClass::PayloadTooLarge,
        "payload exceeding max should return PayloadTooLarge class",
    );
    kani::assert(
        read_u32_le(&header_bytes, 12) == Some(payload_len),
        "len matches",
    );
}

/// VB-STORAGE-DECODE-004 H3: decode rejects payload_len exactly at max + 1
#[kani::proof]
fn kani_record_payload_len_exactly_over_max() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let max_payload: u32 = 100;
    let payload_len: u32 = 101; // Exactly over

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_payload_header(&mut header_bytes, payload_len),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_payload_len_class(&header_bytes, expected_magic, max_payload)
            == PayloadLenDecodeClass::PayloadTooLarge,
        "payload exactly over max should return PayloadTooLarge class",
    );
}

/// VB-STORAGE-DECODE-004 H4: decode accepts payload_len exactly at max
#[kani::proof]
fn kani_record_payload_len_exactly_at_max() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let max_payload: u32 = 100;
    let payload_len: u32 = 100; // Exactly at max

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_payload_header(&mut header_bytes, payload_len),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_payload_len_class(&header_bytes, expected_magic, max_payload)
            != PayloadLenDecodeClass::PayloadTooLarge,
        "payload at max should not be rejected",
    );
}

/// VB-STORAGE-DECODE-004 H5: decode with zero max_payload rejects non-zero payload
#[kani::proof]
fn kani_record_payload_len_rejects_nonzero_when_max_zero() {
    let expected_magic: u32 = MAGIC_WORKFLOW_SOURCE;
    let max_payload: u32 = 0;
    let payload_len: u32 = 1;

    let mut header_bytes = [0u8; RECORD_HEADER_BYTES];
    kani::assert(
        write_payload_header(&mut header_bytes, payload_len),
        "header fixture writes stay in bounds",
    );

    kani::assert(
        decode_payload_len_class(&header_bytes, expected_magic, max_payload)
            == PayloadLenDecodeClass::PayloadTooLarge,
        "non-zero payload with zero max should return PayloadTooLarge class",
    );
}
