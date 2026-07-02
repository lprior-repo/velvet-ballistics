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

fuzz_target!(|data: &[u8]| {
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let schema_version = vb_storage::CURRENT_SCHEMA_VERSION;

    let Ok((_, event)) =
        vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload)
    else {
        return;
    };
    if !event.is_valid() {
        return;
    }

    let Ok(mut encoded) = vb_storage::encode_record(
        magic,
        event.record_kind(),
        event.seq().get(),
        &event,
        max_payload,
    ) else {
        return;
    };

    if encoded.len() < 60 {
        return;
    }

    // Schema version +1 (with recomputed CRC over [..56]).
    let next = schema_version.wrapping_add(1);
    encoded[4..6].copy_from_slice(&next.to_le_bytes());
    let checksum = crc32c::crc32c(&encoded[..56]);
    encoded[56..60].copy_from_slice(&checksum.to_le_bytes());
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);

    // Schema version -1 (with recomputed CRC).
    let prev = schema_version.wrapping_sub(1);
    encoded[4..6].copy_from_slice(&prev.to_le_bytes());
    let checksum = crc32c::crc32c(&encoded[..56]);
    encoded[56..60].copy_from_slice(&checksum.to_le_bytes());
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);

    // Unknown record-kind wire id 999 (with recomputed CRC).
    let unknown_kind: u16 = 999;
    encoded[6..8].copy_from_slice(&unknown_kind.to_le_bytes());
    let checksum = crc32c::crc32c(&encoded[..56]);
    encoded[56..60].copy_from_slice(&checksum.to_le_bytes());
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);

    // Wrong header-length field 99 (with recomputed CRC).
    let wrong_header_len: u32 = 99;
    encoded[8..12].copy_from_slice(&wrong_header_len.to_le_bytes());
    let checksum = crc32c::crc32c(&encoded[..56]);
    encoded[56..60].copy_from_slice(&checksum.to_le_bytes());
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);
});
