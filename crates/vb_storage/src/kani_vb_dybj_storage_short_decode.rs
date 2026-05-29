#![cfg(kani)]
#![forbid(unsafe_code)]

//! PO-VB-DYBJ-010: short storage inputs return UnexpectedEof before payload decode.

use crate::{
    JournalError, WorkflowSourceRecord, decode_record, encode_record,
    constants::{MAGIC_WORKFLOW_SOURCE, MAX_WORKFLOW_SOURCE_BYTES, RECORD_HEADER_BYTES},
};
use vb_core::WorkflowDigest;

#[kani::proof]
fn kani_vb_dybj_storage_short_inputs_unexpected_eof() {
    let short_len: usize = kani::any();
    kani::assume(short_len < RECORD_HEADER_BYTES);
    let bytes = vec![0_u8; short_len];
    let result = decode_record::<WorkflowSourceRecord>(
        &bytes,
        MAGIC_WORKFLOW_SOURCE,
        MAX_WORKFLOW_SOURCE_BYTES,
    );
    assert!(matches!(result, Err(JournalError::UnexpectedEof)));

    let payload = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([7_u8; 32]),
        source: vec![1_u8, 2_u8, 3_u8, 4_u8],
    };
    let encoded = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        crate::RecordKind::WorkflowSource,
        1_u64,
        &payload,
        MAX_WORKFLOW_SOURCE_BYTES,
    );
    assert!(encoded.is_ok());
    if let Ok(record) = encoded {
        let cut: usize = kani::any();
        kani::assume(cut >= RECORD_HEADER_BYTES);
        kani::assume(cut < record.len());
        let truncated = &record[..cut];
        let truncated_result = decode_record::<WorkflowSourceRecord>(
            truncated,
            MAGIC_WORKFLOW_SOURCE,
            MAX_WORKFLOW_SOURCE_BYTES,
        );
        assert!(matches!(truncated_result, Err(JournalError::UnexpectedEof)));
    }
}
