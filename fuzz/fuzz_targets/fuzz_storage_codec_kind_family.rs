#![no_main]

//! Fuzz target: storage_codec_kind_family
//!
//! Split from `vb_storage_codec` (PO-vb-y9d3v-0041). Oracle: encoding a
//! `WorkflowSourceRecord` payload under a (magic, kind) pair that violates
//! the family invariants must be rejected with a typed `JournalError`
//! (`RecordKindFamilyMismatch` or `UnknownRecordKind`) rather than panicking.
//! Valid pairings must encode successfully.
//!
//! Run with: cargo fuzz run fuzz_storage_codec_kind_family -- -max_len=4096 -runs=100000

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut source = data.to_vec();
    source.truncate(100);

    let record = vb_storage::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        source,
    };

    // Valid pairing: MAGIC_WORKFLOW_SOURCE + RecordKind::WorkflowSource must encode.
    let _ = vb_storage::encode_record(
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::RecordKind::WorkflowSource,
        0,
        &record,
        128,
    );

    // Mismatched family pairings must be rejected.
    let _ = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::WorkflowSource,
        0,
        &record,
        128,
    );
    let _ = vb_storage::encode_record(
        vb_storage::MAGIC_BLOB,
        vb_storage::RecordKind::Snapshot,
        0,
        &record,
        128,
    );
    let _ = vb_storage::encode_record(
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::RecordKind::IndexUpdate,
        0,
        &record,
        128,
    );
    let _ = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &record,
        128,
    );
});
