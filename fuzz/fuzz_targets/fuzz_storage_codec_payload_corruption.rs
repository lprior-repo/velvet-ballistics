#![no_main]

//! Fuzz target: storage_codec_payload_corruption.
//! Oracle: generate a valid `JournalEvent`, mutate deterministic header,
//! digest, payload, sequence, and truncation fields, then require exact
//! `JournalError` variants from production `decode_journal_event`.

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, StepIdx};
use vb_storage::JournalError;

const MAGIC_OFFSET: usize = 0;
const SCHEMA_VERSION_OFFSET: usize = 4;
const RECORD_KIND_OFFSET: usize = 6;
const HEADER_LENGTH_OFFSET: usize = 8;
const PAYLOAD_LENGTH_OFFSET: usize = 12;
const SEQUENCE_OFFSET: usize = 16;
const DIGEST_OFFSET: usize = 24;
const CRC_OFFSET: usize = 56;
const PAYLOAD_OFFSET: usize = 60;

fuzz_target!(|data: &[u8]| {
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let event = valid_event_from_data(data);
    let encoded = encode_valid_event(&event, magic, max_payload);

    // Sanity: the freshly encoded bytes must round-trip successfully.
    assert!(
        vb_storage::decode_journal_event(&encoded, magic, max_payload).is_ok(),
        "freshly encoded event must decode"
    );

    // Single-byte flip corruption across magic/schema/kind/len/seq/digest/crc.
    assert_corruption_error(
        &encoded,
        MAGIC_OFFSET,
        magic,
        max_payload,
        ExpectedDecodeError::BadMagic,
    );
    assert_corruption_error(
        &encoded,
        SCHEMA_VERSION_OFFSET,
        magic,
        max_payload,
        ExpectedDecodeError::UnsupportedSchemaVersion,
    );
    assert_corruption_error(
        &encoded,
        HEADER_LENGTH_OFFSET,
        magic,
        max_payload,
        ExpectedDecodeError::HeaderLengthMismatch,
    );
    assert_corruption_error(
        &encoded,
        CRC_OFFSET,
        magic,
        max_payload,
        ExpectedDecodeError::HeaderChecksumMismatch,
    );
    assert_corruption_error(
        &encoded,
        PAYLOAD_OFFSET,
        magic,
        max_payload,
        ExpectedDecodeError::PayloadDigestMismatch,
    );

    let unknown_kind = 999u16;
    let mut corrupted = encoded.clone();
    write_bytes_at_must(
        &mut corrupted,
        RECORD_KIND_OFFSET,
        &unknown_kind.to_le_bytes(),
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::UnknownRecordKind(unknown_kind),
    );

    let too_large_len = max_payload.saturating_add(1);
    let mut corrupted = encoded.clone();
    write_bytes_at_must(
        &mut corrupted,
        PAYLOAD_LENGTH_OFFSET,
        &too_large_len.to_le_bytes(),
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::PayloadTooLarge {
            len: too_large_len,
            max: max_payload,
        },
    );

    let envelope_seq = event.seq().get().saturating_add(1);
    let mut corrupted = encoded.clone();
    write_bytes_at_must(&mut corrupted, SEQUENCE_OFFSET, &envelope_seq.to_le_bytes());
    refresh_crc_must(&mut corrupted);
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::ReplayEnvelopeSequenceMismatch {
            envelope_seq,
            payload_seq: event.seq().get(),
        },
    );

    for digest_offset in [DIGEST_OFFSET, 32, 48] {
        let mut corrupted = corrupted_at_must(&encoded, digest_offset);
        refresh_crc_must(&mut corrupted);
        assert_decode_error(
            &corrupted,
            magic,
            max_payload,
            ExpectedDecodeError::PayloadDigestMismatch,
        );
    }

    // Truncation at every prefix length — decoder must reject, not panic.
    let cap = encoded.len().min(vb_storage::RECORD_HEADER_BYTES);
    for truncation in 0..cap {
        let Some(prefix) = encoded.get(..truncation) else {
            continue;
        };
        assert_decode_error(
            prefix,
            magic,
            max_payload,
            ExpectedDecodeError::UnexpectedEof,
        );
    }
});

#[derive(Debug, Clone, Copy)]
enum ExpectedDecodeError {
    BadMagic,
    UnsupportedSchemaVersion,
    UnknownRecordKind(u16),
    HeaderLengthMismatch,
    PayloadTooLarge { len: u32, max: u32 },
    HeaderChecksumMismatch,
    PayloadDigestMismatch,
    ReplayEnvelopeSequenceMismatch { envelope_seq: u64, payload_seq: u64 },
    UnexpectedEof,
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
    let result = vb_storage::encode_record(
        magic,
        event.record_kind(),
        event.seq().get(),
        event,
        max_payload,
    );
    assert!(result.is_ok(), "valid generated journal event must encode");
    let Ok(encoded) = result else {
        return Vec::new();
    };
    encoded
}

fn assert_corruption_error(
    encoded: &[u8],
    offset: usize,
    magic: u32,
    max_payload: u32,
    expected: ExpectedDecodeError,
) {
    let corrupted = corrupted_at_must(encoded, offset);
    assert_decode_error(&corrupted, magic, max_payload, expected);
}

fn corrupted_at_must(encoded: &[u8], offset: usize) -> Vec<u8> {
    let mut corrupted = encoded.to_vec();
    match corrupted.get_mut(offset) {
        Some(byte) => {
            *byte = byte.wrapping_add(1);
            corrupted
        }
        None => {
            assert!(
                offset < corrupted.len(),
                "valid encoded record must contain requested corruption offset"
            );
            corrupted
        }
    }
}

fn assert_decode_error(bytes: &[u8], magic: u32, max_payload: u32, expected: ExpectedDecodeError) {
    let result = vb_storage::decode_journal_event(bytes, magic, max_payload);
    assert!(
        matches_expected_error(result, expected),
        "decode_journal_event returned a non-matching error for deterministic corruption"
    );
}

fn matches_expected_error(
    result: Result<(vb_storage::RecordEnvelope, vb_storage::JournalEvent), JournalError>,
    expected: ExpectedDecodeError,
) -> bool {
    match (result, expected) {
        (Err(JournalError::BadMagic { .. }), ExpectedDecodeError::BadMagic) => true,
        (
            Err(JournalError::UnsupportedSchemaVersion { .. }),
            ExpectedDecodeError::UnsupportedSchemaVersion,
        ) => true,
        (
            Err(JournalError::UnknownRecordKind { kind }),
            ExpectedDecodeError::UnknownRecordKind(expected),
        ) => kind == expected,
        (
            Err(JournalError::HeaderLengthMismatch { .. }),
            ExpectedDecodeError::HeaderLengthMismatch,
        ) => true,
        (
            Err(JournalError::PayloadTooLarge { len, max }),
            ExpectedDecodeError::PayloadTooLarge {
                len: expected_len,
                max: expected_max,
            },
        ) => len == expected_len && max == expected_max,
        (
            Err(JournalError::HeaderChecksumMismatch),
            ExpectedDecodeError::HeaderChecksumMismatch,
        ) => true,
        (Err(JournalError::PayloadDigestMismatch), ExpectedDecodeError::PayloadDigestMismatch) => {
            true
        }
        (
            Err(JournalError::ReplayEnvelopeSequenceMismatch {
                envelope_seq,
                payload_seq,
                ..
            }),
            ExpectedDecodeError::ReplayEnvelopeSequenceMismatch {
                envelope_seq: expected_envelope_seq,
                payload_seq: expected_payload_seq,
            },
        ) => envelope_seq == expected_envelope_seq && payload_seq == expected_payload_seq,
        (Err(JournalError::UnexpectedEof), ExpectedDecodeError::UnexpectedEof) => true,
        _ => false,
    }
}

fn write_bytes_at_must(bytes: &mut [u8], offset: usize, value: &[u8]) {
    assert!(
        write_bytes_at(bytes, offset, value),
        "valid encoded record must contain mutation field"
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
    let Some(prefix) = bytes.get(..vb_storage::CRC_OFFSET) else {
        assert!(
            bytes.len() >= vb_storage::CRC_OFFSET,
            "valid encoded record must contain CRC prefix"
        );
        return;
    };
    let checksum = crc32c::crc32c(prefix);
    write_bytes_at_must(bytes, vb_storage::CRC_OFFSET, &checksum.to_le_bytes());
}
