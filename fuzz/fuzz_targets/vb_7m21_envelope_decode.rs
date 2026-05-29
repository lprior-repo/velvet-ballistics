#![no_main]
use libfuzzer_sys::fuzz_target;

// PO-vb-7m21-F001: Storage envelope full decode fuzz target.
//
// Exercises `decode_record<JournalEvent>` with arbitrary bytes to
// discover panics, Postcard decode failures, and digest mismatches
// on hostile data. The envelope format is:
//
//   [60-byte header] [variable-length payload]
//
// This target covers the full decode path: header validation,
// magic check, schema check, kind validation, CRC check,
// payload length bounds, digest verification, and Postcard deserialization.

fuzz_target!(|data: &[u8]| {
    // Full envelope decode — header + payload — with JournalEvent magic
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let max = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;

    let _ = vb_storage::decode_record_header(data, magic, max);
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max);

    // Also try with other magics to cover broader envelope space
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_INDEX_RECORD, 1024);
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_SNAPSHOT, 1024);

    // Try JournalEvent decode via the semantic-validating path
    let _ = vb_storage::codec::decode_journal_event(data, magic, max);
});
