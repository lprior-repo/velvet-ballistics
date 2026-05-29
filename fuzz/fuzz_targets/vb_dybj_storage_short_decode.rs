#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_storage::{
    JournalError, WorkflowSourceRecord, decode_record, MAGIC_WORKFLOW_SOURCE,
    MAX_WORKFLOW_SOURCE_BYTES, RECORD_HEADER_BYTES,
};

fuzz_target!(|data: &[u8]| {
    if data.len() < RECORD_HEADER_BYTES {
        let decoded = decode_record::<WorkflowSourceRecord>(
            data,
            MAGIC_WORKFLOW_SOURCE,
            MAX_WORKFLOW_SOURCE_BYTES,
        );
        assert!(matches!(decoded, Err(JournalError::UnexpectedEof)));
    }

    let seed_record = WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes([0xD1_u8; 32]),
        source: vec![0x76_u8, 0x62_u8, 0x2d_u8, 0x64_u8, 0x79_u8, 0x62_u8, 0x6a_u8],
    };
    let encoded = vb_storage::encode_record(
        MAGIC_WORKFLOW_SOURCE,
        vb_storage::RecordKind::WorkflowSource,
        1_u64,
        &seed_record,
        MAX_WORKFLOW_SOURCE_BYTES,
    );
    if let Ok(record) = encoded {
        if record.len() > RECORD_HEADER_BYTES {
            let variable_cut = RECORD_HEADER_BYTES + data.len() % (record.len() - RECORD_HEADER_BYTES);
            if variable_cut < record.len() {
                let decoded = decode_record::<WorkflowSourceRecord>(
                    &record[..variable_cut],
                    MAGIC_WORKFLOW_SOURCE,
                    MAX_WORKFLOW_SOURCE_BYTES,
                );
                assert!(matches!(decoded, Err(JournalError::UnexpectedEof)));
            }
        }
    }
});
