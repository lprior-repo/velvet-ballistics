#![no_main]

//! Fuzz target: storage_codec_roundtrip
//!
//! Split from `vb_storage_codec` (PO-vb-y9d3v-0041). Oracle: encode→decode→encode
//! equality. For any successfully decodable `JournalEvent` payload, the
//! re-encoded record must decode to the same envelope + event, and a third
//! encode must produce bytes identical to the first encode. Any deviation is
//! a regression in the postcard codec round-trip.
//!
//! Also probes error paths (bad magic, oversized max-payload) to confirm that
//! `encode_record` rejects those inputs gracefully rather than panicking.
//!
//! Run with: cargo fuzz run fuzz_storage_codec_roundtrip -- -max_len=4096 -runs=100000

use libfuzzer_sys::fuzz_target;
use vb_storage::JournalError;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;

    // Probe decode with every known magic — must not panic for any input.
    observe_decode(vb_storage::decode_journal_event(data, magic, max_payload));
    observe_decode(vb_storage::decode_journal_event(
        data,
        vb_storage::MAGIC_BLOB,
        vb_storage::MAX_BLOB_BYTES,
    ));
    observe_decode(vb_storage::decode_journal_event(
        data,
        vb_storage::MAGIC_COMPILED_ARTIFACT,
        vb_storage::MAX_COMPILED_IR_BYTES,
    ));
    observe_decode(vb_storage::decode_journal_event(
        data,
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::MAX_SNAPSHOT_BYTES,
    ));
    observe_decode(vb_storage::decode_journal_event(
        data,
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::MAX_WORKFLOW_SOURCE_BYTES,
    ));
    observe_decode(vb_storage::decode_journal_event(
        data,
        vb_storage::MAGIC_INDEX_RECORD,
        vb_storage::MAX_RUN_HEADER_BYTES,
    ));

    let Ok((envelope, event)) = vb_storage::decode_journal_event(data, magic, max_payload) else {
        return;
    };

    let first_encoded_result = vb_storage::encode_record(
        magic,
        event.record_kind(),
        event.seq().get(),
        &event,
        max_payload,
    );
    assert!(
        first_encoded_result.is_ok(),
        "valid decoded journal event must encode: {:?}",
        first_encoded_result.as_ref().err()
    );
    let Ok(first_encoded) = first_encoded_result else {
        return;
    };

    // Round-trip: decode the freshly encoded bytes and re-encode them. The two
    // encodes must produce identical bytes.
    let redecoded_result = vb_storage::decode_journal_event(&first_encoded, magic, max_payload);
    assert!(
        redecoded_result.is_ok(),
        "freshly encoded journal event must decode: {:?}",
        redecoded_result.as_ref().err()
    );
    let Ok((redecoded_envelope, redecoded_event)) = redecoded_result else {
        return;
    };

    let second_encoded_result = vb_storage::encode_record(
        magic,
        redecoded_event.record_kind(),
        redecoded_event.seq().get(),
        &redecoded_event,
        max_payload,
    );
    assert!(
        second_encoded_result.is_ok(),
        "redecoded journal event must encode: {:?}",
        second_encoded_result.as_ref().err()
    );
    let Ok(second_encoded) = second_encoded_result else {
        return;
    };

    assert_eq!(
        first_encoded, second_encoded,
        "encode→decode→encode produced different bytes"
    );
    assert_eq!(
        envelope.sequence, redecoded_envelope.sequence,
        "envelope sequence lost across round-trip"
    );
    assert_eq!(
        envelope.record_kind, redecoded_envelope.record_kind,
        "envelope record_kind lost across round-trip"
    );
    assert_eq!(
        envelope.schema_version, redecoded_envelope.schema_version,
        "envelope schema_version lost across round-trip"
    );

    // Error path probes: bad magic + zero max_payload must reject, not panic.
    assert_family_mismatch(
        vb_storage::encode_record(
            0xFFFF_FFFFu32,
            event.record_kind(),
            event.seq().get(),
            &event,
            max_payload,
        ),
        0xFFFF_FFFFu32,
        event.record_kind().id(),
    );
    assert_payload_too_large(
        vb_storage::encode_record(magic, event.record_kind(), event.seq().get(), &event, 0),
        0,
    );
});

fn observe_decode(
    result: Result<(vb_storage::RecordEnvelope, vb_storage::JournalEvent), JournalError>,
) {
    match result {
        Ok((_envelope, event)) => {
            assert!(
                event.is_valid(),
                "successful decode must produce a valid event"
            );
        }
        Err(error) => assert_roundtrip_decode_error(error),
    }
}

fn assert_roundtrip_decode_error(error: JournalError) {
    assert!(
        matches!(
            error,
            JournalError::UnexpectedEof
                | JournalError::HeaderChecksumMismatch
                | JournalError::PayloadDigestMismatch
                | JournalError::PostcardDecodeFailed(_)
                | JournalError::InvalidEvent
                | JournalError::BadMagic { .. }
                | JournalError::PayloadTooLarge { .. }
                | JournalError::RecordKindFamilyMismatch { .. }
                | JournalError::UnknownRecordKind { .. }
                | JournalError::UnsupportedSchemaVersion { .. }
                | JournalError::MigrationRequired { .. }
                | JournalError::HeaderLengthMismatch { .. }
                | JournalError::RecordKindPayloadMismatch { .. }
                | JournalError::ReplayEnvelopeSequenceMismatch { .. }
        ),
        "journal event decode must fail with a typed storage codec error"
    );
}

fn assert_family_mismatch(result: Result<Vec<u8>, JournalError>, magic: u32, kind: u16) {
    assert!(
        matches!(
            result,
            Err(JournalError::RecordKindFamilyMismatch {
                magic: actual_magic,
                kind: actual_kind,
            }) if actual_magic == magic && actual_kind == kind
        ),
        "bad magic must return JournalError::RecordKindFamilyMismatch"
    );
}

fn assert_payload_too_large(result: Result<Vec<u8>, JournalError>, max: u32) {
    assert!(
        matches!(
            result,
            Err(JournalError::PayloadTooLarge { len, max: actual_max }) if len > actual_max && actual_max == max
        ),
        "zero max payload must return JournalError::PayloadTooLarge"
    );
}
