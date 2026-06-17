#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
use super::{encoded_record, flip_byte, scribble_u32};
use crate::JournalError;
use crate::codec::decode_journal_event;
use crate::constants::MAGIC_JOURNAL_EVENT;

#[test]
fn decode_rejects_truncated_payload() {
    let mut bytes = encoded_record();
    let new_len = bytes.len().saturating_sub(4);
    bytes.truncate(new_len);
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("truncated payload must fail decode");
    assert!(
        matches!(err, JournalError::UnexpectedEof),
        "truncated payload must yield UnexpectedEof, got {err:?}"
    );
}

#[test]
fn decode_rejects_swapped_magic() {
    let mut bytes = encoded_record();
    scribble_u32(&mut bytes, 0);
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("swapped magic must fail decode");
    assert!(
        matches!(err, JournalError::BadMagic { .. }),
        "swapped magic must yield BadMagic, got {err:?}"
    );
}

#[test]
fn decode_rejects_corrupted_crc32c() {
    let mut bytes = encoded_record();
    scribble_u32(&mut bytes, 56);
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("corrupted CRC32C must fail decode");
    assert!(
        matches!(err, JournalError::HeaderChecksumMismatch),
        "corrupted CRC32C must yield HeaderChecksumMismatch, got {err:?}"
    );
}

#[test]
fn decode_rejects_blake3_digest_mismatch() {
    let mut bytes = encoded_record();
    flip_byte(&mut bytes, 40);
    let new_crc = crc32c::crc32c(&bytes[..56]);
    bytes[56..60].copy_from_slice(&new_crc.to_le_bytes());
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("BLAKE3 digest mismatch must fail decode");
    assert!(
        matches!(err, JournalError::PayloadDigestMismatch),
        "BLAKE3 digest mismatch must yield PayloadDigestMismatch, got {err:?}"
    );
}

#[test]
fn decode_rejects_payload_len_overflow() {
    let mut bytes = encoded_record();
    let max_payload = 65_536_u32;
    let max_bytes = u32::MAX.to_le_bytes();
    for (i, slot) in bytes.iter_mut().enumerate().skip(12).take(4) {
        *slot = max_bytes[i - 12];
    }
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, max_payload)
        .expect_err("payload_len overflow must fail decode");
    assert!(
        matches!(err, JournalError::PayloadTooLarge { .. }),
        "payload_len overflow must yield PayloadTooLarge, got {err:?}"
    );
}

#[test]
fn decode_rejects_header_len_mismatch() {
    let mut bytes = encoded_record();
    scribble_u32(&mut bytes, 8);
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("header_len mismatch must fail decode");
    assert!(
        matches!(err, JournalError::HeaderLengthMismatch { .. }),
        "header_len mismatch must yield HeaderLengthMismatch, got {err:?}"
    );
}

#[test]
fn decode_rejects_record_kind_outside_family() {
    let mut bytes = encoded_record();
    let kind_bytes = 0x00_FF_u16.to_le_bytes();
    for (i, slot) in bytes.iter_mut().enumerate().skip(6).take(2) {
        *slot = kind_bytes[i - 6];
    }
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("record_kind outside family must fail decode");
    assert!(
        matches!(
            err,
            JournalError::UnknownRecordKind { .. } | JournalError::RecordKindFamilyMismatch { .. }
        ),
        "invalid record_kind must yield UnknownRecordKind or RecordKindFamilyMismatch, got {err:?}"
    );
}

#[test]
fn decode_rejects_unknown_record_kind_family() {
    let mut bytes = encoded_record();
    for slot in bytes.iter_mut().skip(6).take(2) {
        *slot = 0;
    }
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("unknown record_kind must fail decode");
    assert!(
        matches!(
            err,
            JournalError::UnknownRecordKind { .. } | JournalError::RecordKindFamilyMismatch { .. }
        ),
        "unknown record_kind family must yield typed error, got {err:?}"
    );
}
