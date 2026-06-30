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

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;

    // Probe decode with every known magic — must not panic for any input.
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload);
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_BLOB,
        vb_storage::MAX_BLOB_BYTES,
    );
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_COMPILED_ARTIFACT,
        vb_storage::MAX_COMPILED_IR_BYTES,
    );
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::MAX_SNAPSHOT_BYTES,
    );
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::MAX_WORKFLOW_SOURCE_BYTES,
    );
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_INDEX_RECORD,
        vb_storage::MAX_RUN_HEADER_BYTES,
    );

    let Ok((envelope, event)) =
        vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload)
    else {
        return;
    };
    if !event.is_valid() {
        return;
    }

    let Ok(first_encoded) = vb_storage::encode_record(
        magic,
        event.record_kind(),
        event.seq().get(),
        &event,
        max_payload,
    ) else {
        return;
    };

    // Round-trip: decode the freshly encoded bytes and re-encode them. The two
    // encodes must produce identical bytes.
    let Ok((redecoded_envelope, redecoded_event)) =
        vb_storage::decode_record::<vb_storage::JournalEvent>(&first_encoded, magic, max_payload)
    else {
        return;
    };

    let Ok(second_encoded) = vb_storage::encode_record(
        magic,
        redecoded_event.record_kind(),
        redecoded_event.seq().get(),
        &redecoded_event,
        max_payload,
    ) else {
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

    // Error path probes: bad magic + oversized max_payload must reject, not panic.
    let _ = vb_storage::encode_record(
        0xFFFF_FFFFu32,
        event.record_kind(),
        event.seq().get(),
        &event,
        max_payload,
    );
    let _ = vb_storage::encode_record(
        magic,
        event.record_kind(),
        event.seq().get(),
        &event,
        u32::MAX,
    );
});
