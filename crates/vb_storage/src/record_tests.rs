#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod record_tests {
    use crate::{
        BlobRecord, CompiledIrRecord, DIGEST_BYTES, RecordKind, RunHeaderRecord,
        WorkflowSourceRecord,
    };
    use vb_core::{RunId, WorkflowDigest, WorkflowId};

    #[test]
    fn record_kind_workflow_source_has_correct_id() {
        assert_eq!(RecordKind::WorkflowSource.id(), 1);
    }

    #[test]
    fn record_kind_compiled_ir_has_correct_id() {
        assert_eq!(RecordKind::CompiledIr.id(), 2);
    }

    #[test]
    fn record_kind_run_header_has_correct_id() {
        assert_eq!(RecordKind::RunHeader.id(), 3);
    }

    #[test]
    fn record_kind_snapshot_has_correct_id() {
        assert_eq!(RecordKind::Snapshot.id(), 30);
    }

    #[test]
    fn record_kind_blob_has_correct_id() {
        assert_eq!(RecordKind::Blob.id(), 40);
    }

    #[test]
    fn record_kind_event_variants_have_correct_ids() {
        assert_eq!(RecordKind::RunAccepted.id(), 10);
        assert_eq!(RecordKind::StepStarted.id(), 11);
        assert_eq!(RecordKind::SlotWritten.id(), 12);
        assert_eq!(RecordKind::ActionScheduled.id(), 13);
        assert_eq!(RecordKind::ActionCompleted.id(), 14);
        assert_eq!(RecordKind::ActionFailed.id(), 15);
        assert_eq!(RecordKind::WaitScheduled.id(), 16);
        assert_eq!(RecordKind::AskScheduled.id(), 17);
        assert_eq!(RecordKind::AskAnswered.id(), 18);
        assert_eq!(RecordKind::RetryScheduled.id(), 19);
        assert_eq!(RecordKind::RunCancelled.id(), 21);
        assert_eq!(RecordKind::RunFinished.id(), 22);
        assert_eq!(RecordKind::RunFailed.id(), 23);
        assert_eq!(RecordKind::RunAdmission.id(), 24);
    }

    #[test]
    fn record_kind_all_ids_are_unique() {
        let ids: Vec<u16> = vec![
            RecordKind::WorkflowSource.id(),
            RecordKind::CompiledIr.id(),
            RecordKind::RunHeader.id(),
            RecordKind::RunAccepted.id(),
            RecordKind::StepStarted.id(),
            RecordKind::SlotWritten.id(),
            RecordKind::ActionScheduled.id(),
            RecordKind::ActionCompleted.id(),
            RecordKind::ActionFailed.id(),
            RecordKind::WaitScheduled.id(),
            RecordKind::AskScheduled.id(),
            RecordKind::AskAnswered.id(),
            RecordKind::RetryScheduled.id(),
            RecordKind::StepFailed.id(),
            RecordKind::RunCancelled.id(),
            RecordKind::RunFinished.id(),
            RecordKind::RunFailed.id(),
            RecordKind::RunAdmission.id(),
            RecordKind::RunResumed.id(),
            RecordKind::RunRetried.id(),
            RecordKind::RunAnswered.id(),
            RecordKind::Snapshot.id(),
            RecordKind::Blob.id(),
            RecordKind::IndexUpdate.id(),
        ];
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "all record kind IDs must be unique");
    }

    #[test]
    fn workflow_source_record_has_expected_fields() {
        let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);
        let record = WorkflowSourceRecord {
            digest,
            source: b"test source".to_vec(),
        };
        assert_eq!(record.digest, digest);
        assert_eq!(record.source, b"test source".to_vec());
    }

    #[test]
    fn compiled_ir_record_has_expected_fields() {
        let digest = WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]);
        let record = CompiledIrRecord {
            digest,
            ir: b"test ir".to_vec(),
        };
        assert_eq!(record.digest, digest);
        assert_eq!(record.ir, b"test ir".to_vec());
    }

    #[test]
    fn run_header_record_has_expected_fields() {
        let run = RunId::new(42);
        let record = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(7),
            compiled_digest: WorkflowDigest::from_bytes([0x33; DIGEST_BYTES]),
            status: 2,
            accepted_at_ms: 9999,
        };
        assert_eq!(record.run, run);
        assert_eq!(record.workflow_id, WorkflowId::new(7));
        assert_eq!(record.status, 2);
        assert_eq!(record.accepted_at_ms, 9999);
    }

    #[test]
    fn blob_record_has_expected_fields() {
        let digest: [u8; DIGEST_BYTES] = [0x44; 32];
        let record = BlobRecord {
            digest,
            bytes: vec![1, 2, 3],
        };
        assert_eq!(record.digest, digest);
        assert_eq!(record.bytes, vec![1, 2, 3]);
    }

    #[test]
    fn blob_record_accepts_empty_bytes() {
        let digest: [u8; DIGEST_BYTES] = [0x55; 32];
        let record = BlobRecord {
            digest,
            bytes: vec![],
        };
        assert!(record.bytes.is_empty());
        assert_eq!(record.digest, digest);
    }
}
