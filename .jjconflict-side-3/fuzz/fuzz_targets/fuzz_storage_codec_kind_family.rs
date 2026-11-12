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
use vb_storage::JournalError;

fuzz_target!(|data: &[u8]| {
    let mut source = data.to_vec();
    source.truncate(100);

    let record = vb_storage::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        source,
    };

    // Valid pairing: MAGIC_WORKFLOW_SOURCE + RecordKind::WorkflowSource must encode.
    let valid = vb_storage::encode_record(
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::RecordKind::WorkflowSource,
        0,
        &record,
        vb_storage::MAX_WORKFLOW_SOURCE_BYTES,
    );
    assert!(
        valid.is_ok(),
        "valid workflow-source family pair must encode"
    );

    // Mismatched family pairings must be rejected.
    assert_family_mismatch(
        vb_storage::encode_record(
            vb_storage::MAGIC_JOURNAL_EVENT,
            vb_storage::RecordKind::WorkflowSource,
            0,
            &record,
            128,
        ),
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::WorkflowSource.id(),
    );
    assert_family_mismatch(
        vb_storage::encode_record(
            vb_storage::MAGIC_BLOB,
            vb_storage::RecordKind::Snapshot,
            0,
            &record,
            128,
        ),
        vb_storage::MAGIC_BLOB,
        vb_storage::RecordKind::Snapshot.id(),
    );
    assert_family_mismatch(
        vb_storage::encode_record(
            vb_storage::MAGIC_SNAPSHOT,
            vb_storage::RecordKind::IndexUpdate,
            0,
            &record,
            128,
        ),
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::RecordKind::IndexUpdate.id(),
    );
    assert_family_mismatch(
        vb_storage::encode_record(
            vb_storage::MAGIC_WORKFLOW_SOURCE,
            vb_storage::RecordKind::RunAccepted,
            0,
            &record,
            128,
        ),
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::RecordKind::RunAccepted.id(),
    );
});

fn assert_family_mismatch(
    result: Result<Vec<u8>, JournalError>,
    expected_magic: u32,
    expected_kind: u16,
) {
    assert!(
        matches!(
            result,
            Err(JournalError::RecordKindFamilyMismatch { magic, kind })
                if magic == expected_magic && kind == expected_kind
        ),
        "mismatched storage record family must return JournalError::RecordKindFamilyMismatch"
    );
}
