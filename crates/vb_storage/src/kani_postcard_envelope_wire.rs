#![forbid(unsafe_code)]
#![cfg(kani)]
//! VB-STORAGE-POSTCARD-ENVELOPE-001: storage record envelope decode order checks.

use crate::binary::{write_digest, write_u16, write_u32, write_u64};
use crate::codec::header::{decode_record_header, header_crc32c};
use crate::codec::payload::decode_record_payload;
use crate::{
    constants::{
        CRC_OFFSET, CURRENT_SCHEMA_VERSION, DIGEST_BYTES, MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
    },
    error::JournalError,
    records::RecordKind,
};

const DEFAULT_MAX_PAYLOAD: u32 = MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
const FOUR_BYTE_PAYLOAD_LEN_U32: u32 = 4;
const FOUR_BYTE_PAYLOAD_BYTES: usize = 4;
const FOUR_BYTE_RECORD_BYTES: usize = 64;

fn require_write(result: Result<(), JournalError>) {
    if result.is_err() {
        kani::assume(false);
    }
}

fn checksum(header: &[u8; RECORD_HEADER_BYTES]) -> u32 {
    match header.get(..CRC_OFFSET) {
        Some(prefix) => header_crc32c(prefix),
        None => {
            kani::assume(false);
            0
        }
    }
}

fn write_checksum(header: &mut [u8; RECORD_HEADER_BYTES]) {
    let value = checksum(header);
    require_write(write_u32(header, CRC_OFFSET, value));
}

fn arbitrary_digest_bytes() -> [u8; DIGEST_BYTES] {
    kani::any()
}

fn header_with_fields(
    magic: u32,
    payload_len: u32,
    digest: [u8; DIGEST_BYTES],
) -> [u8; RECORD_HEADER_BYTES] {
    let mut header = [0_u8; RECORD_HEADER_BYTES];
    require_write(write_u32(&mut header, 0, magic));
    require_write(write_u16(&mut header, 4, CURRENT_SCHEMA_VERSION));
    require_write(write_u16(&mut header, 6, RecordKind::RunAccepted.id()));
    require_write(write_u32(&mut header, 8, RECORD_HEADER_LEN));
    require_write(write_u32(&mut header, 12, payload_len));
    require_write(write_u64(&mut header, 16, 0));
    require_write(write_digest(&mut header, &digest));
    write_checksum(&mut header);
    header
}

fn record_with_four_byte_payload(
    header: &[u8; RECORD_HEADER_BYTES],
    payload: &[u8; FOUR_BYTE_PAYLOAD_BYTES],
) -> [u8; FOUR_BYTE_RECORD_BYTES] {
    let mut record = [0_u8; FOUR_BYTE_RECORD_BYTES];
    match record.get_mut(..RECORD_HEADER_BYTES) {
        Some(target) => target.copy_from_slice(header),
        None => kani::assume(false),
    }
    match record.get_mut(RECORD_HEADER_BYTES..FOUR_BYTE_RECORD_BYTES) {
        Some(target) => target.copy_from_slice(payload),
        None => kani::assume(false),
    }
    record
}

/// H1: wrong magic is rejected before payload allocation or postcard decode.
#[kani::proof]
#[kani::unwind(8)]
fn kani_harness_bad_magic_no_allocation() {
    let expected_magic = MAGIC_JOURNAL_EVENT;
    let wrong_magic: u32 = kani::any();
    kani::assume(wrong_magic != expected_magic);
    let header = header_with_fields(wrong_magic, 0, arbitrary_digest_bytes());

    match decode_record_header(&header, expected_magic, DEFAULT_MAX_PAYLOAD) {
        Err(JournalError::BadMagic { found }) => {
            kani::assert(found == wrong_magic, "BadMagic contains wrong magic");
        }
        Err(_) => kani::assert(false, "wrong magic must fail at BadMagic"),
        Ok(_) => kani::assert(false, "wrong magic must not decode"),
    }
}

/// H2: payload length greater than the configured maximum is rejected at header decode.
#[kani::proof]
#[kani::unwind(8)]
fn kani_harness_payload_len_bounds() {
    let max_payload: u32 = kani::any();
    kani::assume(max_payload < u32::MAX);
    let oversized_payload_len = match max_payload.checked_add(1) {
        Some(value) => value,
        None => {
            kani::assume(false);
            0
        }
    };
    let header = header_with_fields(
        MAGIC_JOURNAL_EVENT,
        oversized_payload_len,
        arbitrary_digest_bytes(),
    );

    match decode_record_header(&header, MAGIC_JOURNAL_EVENT, max_payload) {
        Err(JournalError::PayloadTooLarge { len, max }) => {
            kani::assert(len == oversized_payload_len, "PayloadTooLarge len");
            kani::assert(max == max_payload, "PayloadTooLarge max");
        }
        Err(_) => kani::assert(false, "oversized payload must fail at PayloadTooLarge"),
        Ok(_) => kani::assert(false, "oversized payload must not decode"),
    }
}

/// H3: magic check precedes later header validations.
#[kani::proof]
#[kani::unwind(8)]
fn kani_harness_storage_decode_order() {
    let magic: u32 = kani::any();
    let header = header_with_fields(magic, 0, arbitrary_digest_bytes());
    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, DEFAULT_MAX_PAYLOAD);

    if magic != MAGIC_JOURNAL_EVENT {
        kani::assert(
            matches!(result, Err(JournalError::BadMagic { .. })),
            "wrong magic must return BadMagic before later checks",
        );
    }
}

/// H4: header CRC mismatch is rejected before payload digest checks.
#[kani::proof]
#[kani::unwind(8)]
fn kani_harness_crc_before_digest() {
    let mut header = header_with_fields(MAGIC_JOURNAL_EVENT, 0, arbitrary_digest_bytes());
    let bad_crc = checksum(&header).wrapping_add(1);
    require_write(write_u32(&mut header, CRC_OFFSET, bad_crc));

    match decode_record_header(&header, MAGIC_JOURNAL_EVENT, DEFAULT_MAX_PAYLOAD) {
        Err(JournalError::HeaderChecksumMismatch) => {}
        Err(_) => kani::assert(false, "bad CRC must fail at HeaderChecksumMismatch"),
        Ok(_) => kani::assert(false, "bad CRC must not decode"),
    }
}

/// H5: payload digest mismatch is reported before postcard payload decoding.
#[kani::proof]
#[kani::unwind(8)]
fn kani_harness_digest_before_postcard() {
    let payload: [u8; FOUR_BYTE_PAYLOAD_BYTES] = kani::any();
    let digest = arbitrary_digest_bytes();
    let payload_digest = *blake3::hash(&payload).as_bytes();
    kani::assume(digest != payload_digest);

    let header = header_with_fields(MAGIC_JOURNAL_EVENT, FOUR_BYTE_PAYLOAD_LEN_U32, digest);
    let record = record_with_four_byte_payload(&header, &payload);

    match decode_record_payload(&record, MAGIC_JOURNAL_EVENT, DEFAULT_MAX_PAYLOAD) {
        Err(JournalError::PayloadDigestMismatch) => {}
        Err(_) => kani::assert(false, "digest mismatch must fail before postcard decode"),
        Ok(_) => kani::assert(false, "digest mismatch must not decode"),
    }
}
