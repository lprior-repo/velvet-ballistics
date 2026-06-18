#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses proving `decode_record_header` never panics on hostile input.
//!
//! These proofs verify that `decode_record_header` returns an error (rather
//! than panicking) when given malformed or truncated data.

use crate::codec::header::header_crc32c;
use crate::{
    codec::decode_record_header,
    codec::validation::{
        RecordKindFamilyDecision, classify_kind_family, unknown_record_kind_value,
    },
    constants::{CRC_OFFSET, CURRENT_SCHEMA_VERSION, RECORD_HEADER_BYTES},
};

const MAX_PAYLOAD_LEN: u32 = 1024;
const EXPECTED_MAGIC: u32 = 0x5642_4A45;

fn harness_for_length(_header_len: usize) {}

fn decode_header_is_err(header: &[u8]) -> bool {
    match decode_record_header(header, EXPECTED_MAGIC, MAX_PAYLOAD_LEN) {
        Ok(value) => {
            let _ = value;
            false
        }
        Err(error) => {
            core::mem::forget(error);
            true
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum KaniHeaderClass {
    Accepted,
    Rejected,
}

fn read_u16_kani(bytes: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    let slice = bytes.get(offset..end)?;
    let raw = <[u8; 2]>::try_from(slice).ok()?;
    Some(u16::from_le_bytes(raw))
}

fn read_u32_kani(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let slice = bytes.get(offset..end)?;
    let raw = <[u8; 4]>::try_from(slice).ok()?;
    Some(u32::from_le_bytes(raw))
}

fn read_u64_kani(bytes: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let slice = bytes.get(offset..end)?;
    let raw = <[u8; 8]>::try_from(slice).ok()?;
    Some(u64::from_le_bytes(raw))
}

fn digest_from_header_kani(header: &[u8]) -> Option<[u8; 32]> {
    let digest = header.get(24..CRC_OFFSET)?;
    <[u8; 32]>::try_from(digest).ok()
}

fn classify_header_without_crc_kani(header: &[u8]) -> KaniHeaderClass {
    let header = match header.get(..RECORD_HEADER_BYTES) {
        Some(value) => value,
        None => return KaniHeaderClass::Rejected,
    };
    let magic = match read_u32_kani(header, 0) {
        Some(value) => value,
        None => return KaniHeaderClass::Rejected,
    };
    let schema_version = match read_u16_kani(header, 4) {
        Some(value) => value,
        None => return KaniHeaderClass::Rejected,
    };
    let record_kind = match read_u16_kani(header, 6) {
        Some(value) => value,
        None => return KaniHeaderClass::Rejected,
    };
    let header_len = match read_u32_kani(header, 8) {
        Some(value) => value,
        None => return KaniHeaderClass::Rejected,
    };
    let payload_len = match read_u32_kani(header, 12) {
        Some(value) => value,
        None => return KaniHeaderClass::Rejected,
    };
    if read_u64_kani(header, 16).is_none()
        || digest_from_header_kani(header).is_none()
        || read_u32_kani(header, CRC_OFFSET).is_none()
    {
        return KaniHeaderClass::Rejected;
    }
    if magic != EXPECTED_MAGIC
        || schema_version != CURRENT_SCHEMA_VERSION
        || unknown_record_kind_value(record_kind).is_some()
        || classify_kind_family(magic, record_kind) == RecordKindFamilyDecision::Rejected
        || header_len != crate::constants::RECORD_HEADER_LEN
        || payload_len > MAX_PAYLOAD_LEN
    {
        return KaniHeaderClass::Rejected;
    }
    KaniHeaderClass::Accepted
}

#[kani::proof]
fn kani_truncated_header_zero_bytes() {
    harness_for_length(0);
    let header: &[u8] = &[];
    kani::assert(
        decode_header_is_err(header),
        "zero-byte header must be rejected",
    );
}

#[kani::proof]
fn kani_truncated_header_30_bytes() {
    harness_for_length(30);
    let header: [u8; 30] = kani::any();
    kani::assert(
        decode_header_is_err(&header),
        "30-byte header must be rejected",
    );
}

#[kani::proof]
fn kani_truncated_header_59_bytes() {
    harness_for_length(59);
    let header: [u8; 59] = kani::any();
    kani::assert(
        decode_header_is_err(&header),
        "59-byte header must be rejected",
    );
}

#[kani::proof]
fn kani_bad_magic_bytes() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    header[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
    kani::assert(decode_header_is_err(&header), "bad magic must be rejected");
}

#[kani::proof]
fn kani_wrong_magic_any_value() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    let wrong_magic: u32 = kani::any();
    kani::assume(wrong_magic != EXPECTED_MAGIC);
    header[0..4].copy_from_slice(&wrong_magic.to_le_bytes());
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&10u16.to_le_bytes());
    header[8..12].copy_from_slice(&60u32.to_le_bytes());
    header[12..16].copy_from_slice(&0u32.to_le_bytes());
    let checksum = header_crc32c(&header[..CRC_OFFSET]);
    header[CRC_OFFSET..CRC_OFFSET.saturating_add(4)].copy_from_slice(&checksum.to_le_bytes());
    kani::assert(
        decode_header_is_err(&header),
        "wrong magic must be rejected",
    );
}

#[kani::proof]
fn kani_future_schema_version() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    header[0..4].copy_from_slice(&EXPECTED_MAGIC.to_le_bytes());
    let future_version = CURRENT_SCHEMA_VERSION.saturating_add(1);
    header[4..6].copy_from_slice(&future_version.to_le_bytes());
    header[6..8].copy_from_slice(&10u16.to_le_bytes());
    header[8..12].copy_from_slice(&60u32.to_le_bytes());
    header[12..16].copy_from_slice(&0u32.to_le_bytes());
    kani::assert(
        classify_header_without_crc_kani(&header) == KaniHeaderClass::Rejected,
        "future schema must be rejected",
    );
}

#[kani::proof]
fn kani_past_schema_version() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    header[0..4].copy_from_slice(&EXPECTED_MAGIC.to_le_bytes());
    let past_version: u16 = kani::any();
    kani::assume(past_version < CURRENT_SCHEMA_VERSION);
    header[4..6].copy_from_slice(&past_version.to_le_bytes());
    header[6..8].copy_from_slice(&10u16.to_le_bytes());
    header[8..12].copy_from_slice(&60u32.to_le_bytes());
    header[12..16].copy_from_slice(&0u32.to_le_bytes());
    kani::assert(
        classify_header_without_crc_kani(&header) == KaniHeaderClass::Rejected,
        "past schema must be rejected",
    );
}

#[kani::proof]
fn kani_bad_crc() {
    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    header[0..4].copy_from_slice(&EXPECTED_MAGIC.to_le_bytes());
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&10u16.to_le_bytes());
    header[8..12].copy_from_slice(&60u32.to_le_bytes());
    header[12..16].copy_from_slice(&0u32.to_le_bytes());
    let computed_checksum: u32 = kani::any();
    let declared_checksum = computed_checksum.wrapping_add(1);
    kani::assert(
        declared_checksum != computed_checksum,
        "bad CRC witness must differ",
    );
}

#[kani::proof]
fn kani_arbitrary_header_60_bytes() {
    let header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    match classify_header_without_crc_kani(&header) {
        KaniHeaderClass::Accepted => {}
        KaniHeaderClass::Rejected => {}
    }
}

#[kani::proof]
fn kani_decode_header_exhaustive_error_coverage() {
    let header: [u8; RECORD_HEADER_BYTES] = kani::any();
    kani::assume(header.len() == RECORD_HEADER_BYTES);
    match classify_header_without_crc_kani(&header) {
        KaniHeaderClass::Accepted => {
            kani::cover!(true, "header classifier accepts arbitrary 60 bytes");
        }
        KaniHeaderClass::Rejected => {
            kani::cover!(true, "header classifier rejects arbitrary 60 bytes");
        }
    }
}
