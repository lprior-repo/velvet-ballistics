#![no_main]

//! Fuzz target: storage_codec_schema_version
//!
//! Split from `vb_storage_codec` (PO-vb-y9d3v-0041). Oracle: mutating the
//! schema-version field (bytes 4..6) of a freshly encoded record, with the
//! CRC32C checksum recomputed over the prefix bytes, must cause the decoder
//! to return a typed `JournalError::UnsupportedSchemaVersion`. The same
//! applies to an unknown record-kind (bytes 6..8 → `UnknownRecordKind`) and
//! a wrong header-length field (bytes 8..12 → `HeaderLengthMismatch`).
//! A regression that silently accepts any of these mutations is a critical
//! storage-layer soundness bug.
//!
//! Run with: cargo fuzz run fuzz_storage_codec_schema_version -- -max_len=4096 -runs=100000

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, StepIdx};
use vb_storage::JournalError;

const SCHEMA_VERSION_OFFSET: usize = 4;
const RECORD_KIND_OFFSET: usize = 6;
const HEADER_LENGTH_OFFSET: usize = 8;

fuzz_target!(|data: &[u8]| {
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let schema_version = vb_storage::CURRENT_SCHEMA_VERSION;

    let event = valid_event_from_data(data);
    let encoded = encode_valid_event(&event, magic, max_payload);

    assert!(
        encoded.len() >= 60,
        "valid encoded event must contain a full record header"
    );
    if encoded.len() < 60 {
        return;
    }
    let base_encoded = encoded;

    // Schema version +1 (with recomputed CRC over the checked prefix).
    let next = schema_version.wrapping_add(1);
    let mut encoded = base_encoded.clone();
    write_u16_at_must(&mut encoded, SCHEMA_VERSION_OFFSET, next);
    refresh_crc_must(&mut encoded);
    assert_decode_error(
        &encoded,
        magic,
        max_payload,
        ExpectedDecodeError::UnsupportedSchemaVersion(next),
    );

    // Schema version -1 (with recomputed CRC).
    let prev = schema_version.checked_sub(1);
    assert!(
        prev.is_some(),
        "current schema must be non-zero for migration probe"
    );
    let Some(prev) = prev else {
        return;
    };
    let mut encoded = base_encoded.clone();
    write_u16_at_must(&mut encoded, SCHEMA_VERSION_OFFSET, prev);
    refresh_crc_must(&mut encoded);
    assert_decode_error(
        &encoded,
        magic,
        max_payload,
        ExpectedDecodeError::MigrationRequired {
            from: prev,
            to: schema_version,
        },
    );

    // Unknown record-kind wire id 999 (with recomputed CRC).
    let unknown_kind: u16 = 999;
    let mut encoded = base_encoded.clone();
    write_u16_at_must(&mut encoded, RECORD_KIND_OFFSET, unknown_kind);
    refresh_crc_must(&mut encoded);
    assert_decode_error(
        &encoded,
        magic,
        max_payload,
        ExpectedDecodeError::UnknownRecordKind(unknown_kind),
    );

    // Wrong header-length field 99 (with recomputed CRC).
    let wrong_header_len: u32 = 99;
    let mut encoded = base_encoded;
    write_u32_at_must(&mut encoded, HEADER_LENGTH_OFFSET, wrong_header_len);
    refresh_crc_must(&mut encoded);
    assert_decode_error(
        &encoded,
        magic,
        max_payload,
        ExpectedDecodeError::HeaderLengthMismatch(wrong_header_len),
    );
});

#[derive(Debug, Clone, Copy)]
enum ExpectedDecodeError {
    UnsupportedSchemaVersion(u16),
    MigrationRequired { from: u16, to: u16 },
    UnknownRecordKind(u16),
    HeaderLengthMismatch(u32),
}

fn valid_event_from_data(data: &[u8]) -> vb_storage::JournalEvent {
    let run = RunId::new(u64::from(data.first().copied().unwrap_or(0)).saturating_add(1));
    let seq = vb_storage::EventSeq::new(u64::from(data.get(1).copied().unwrap_or(0)));
    let step = StepIdx::new(u16::from(data.get(2).copied().unwrap_or(0)));
    let attempt = u16::from(data.get(3).copied().unwrap_or(0)).saturating_add(1);
    vb_storage::JournalEvent::StepStarted {
        run,
        seq,
        step,
        attempt,
    }
}

fn encode_valid_event(event: &vb_storage::JournalEvent, magic: u32, max_payload: u32) -> Vec<u8> {
    let encoded = vb_storage::encode_record(
        magic,
        event.record_kind(),
        event.seq().get(),
        event,
        max_payload,
    );
    assert!(
        encoded.is_ok(),
        "valid generated journal event must encode: {:?}",
        encoded.as_ref().err()
    );
    let Ok(encoded) = encoded else {
        return Vec::new();
    };
    encoded
}

fn assert_decode_error(bytes: &[u8], magic: u32, max_payload: u32, expected: ExpectedDecodeError) {
    let result = vb_storage::decode_journal_event(bytes, magic, max_payload);
    assert!(
        matches_expected_decode_error(result, expected),
        "decode_journal_event returned a non-matching error for deterministic header mutation"
    );
}

fn matches_expected_decode_error(
    result: Result<(vb_storage::RecordEnvelope, vb_storage::JournalEvent), JournalError>,
    expected: ExpectedDecodeError,
) -> bool {
    match (result, expected) {
        (
            Err(JournalError::UnsupportedSchemaVersion { version }),
            ExpectedDecodeError::UnsupportedSchemaVersion(expected_version),
        ) => version == expected_version,
        (
            Err(JournalError::MigrationRequired { from, to }),
            ExpectedDecodeError::MigrationRequired {
                from: expected_from,
                to: expected_to,
            },
        ) => from == expected_from && to == expected_to,
        (
            Err(JournalError::UnknownRecordKind { kind }),
            ExpectedDecodeError::UnknownRecordKind(expected_kind),
        ) => kind == expected_kind,
        (
            Err(JournalError::HeaderLengthMismatch { found }),
            ExpectedDecodeError::HeaderLengthMismatch(expected_found),
        ) => found == expected_found,
        _ => false,
    }
}

fn write_u16_at_must(bytes: &mut [u8], offset: usize, value: u16) {
    assert!(
        write_bytes_at(bytes, offset, &value.to_le_bytes()),
        "valid encoded record must contain u16 mutation field"
    );
}

fn write_u32_at_must(bytes: &mut [u8], offset: usize, value: u32) {
    assert!(
        write_bytes_at(bytes, offset, &value.to_le_bytes()),
        "valid encoded record must contain u32 mutation field"
    );
}

fn write_bytes_at(bytes: &mut [u8], offset: usize, value: &[u8]) -> bool {
    let Some(end) = offset.checked_add(value.len()) else {
        return false;
    };
    let Some(target) = bytes.get_mut(offset..end) else {
        return false;
    };
    if target.len() != value.len() {
        return false;
    }
    target.copy_from_slice(value);
    true
}

fn refresh_crc_must(bytes: &mut [u8]) {
    let prefix = bytes.get(..vb_storage::CRC_OFFSET);
    assert!(
        prefix.is_some(),
        "valid encoded record must contain CRC prefix"
    );
    let Some(prefix) = prefix else {
        return;
    };
    let checksum = crc32c::crc32c(prefix);
    write_u32_at_must(bytes, vb_storage::CRC_OFFSET, checksum);
}
