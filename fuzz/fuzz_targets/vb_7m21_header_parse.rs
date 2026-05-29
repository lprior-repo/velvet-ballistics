#![no_main]
use libfuzzer_sys::fuzz_target;

// PO-vb-7m21-F002: Header parse-only fuzz target.
//
// Exercises only the header decode path to isolate header-level
// validation from payload concerns. Tests the 60-byte header parse
// with all magic values and variable max_payload_len bounds.
//
// This target covers:
//   - Truncated header paths (0-59 bytes)
//   - Bad magic values
//   - Future/past schema versions
//   - Unknown record kinds
//   - Kind-family mismatches
//   - Header length mismatches
//   - CRC checksum failures

fuzz_target!(|data: &[u8]| {
    // Header-only decode, bypassing payload concerns
    let _ = vb_storage::decode_record_header(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    let _ = vb_storage::decode_record_header(
        data,
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::MAX_WORKFLOW_SOURCE_BYTES,
    );
    let _ = vb_storage::decode_record_header(
        data,
        vb_storage::MAGIC_COMPILED_ARTIFACT,
        vb_storage::MAX_COMPILED_IR_BYTES,
    );
    let _ = vb_storage::decode_record_header(
        data,
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::MAX_SNAPSHOT_BYTES,
    );
    let _ = vb_storage::decode_record_header(
        data,
        vb_storage::MAGIC_BLOB,
        vb_storage::MAX_BLOB_BYTES,
    );
    let _ = vb_storage::decode_record_header(
        data,
        vb_storage::MAGIC_INDEX_RECORD,
        vb_storage::MAX_RUN_HEADER_BYTES,
    );

    // Edge: max_payload_len = 0 should reject almost everything
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_JOURNAL_EVENT, 0);
});
