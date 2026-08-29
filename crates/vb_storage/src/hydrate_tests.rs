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
mod hydrate_tests {
    use crate::EventSeq;
    use crate::recovery::hydrate::{
        SnapshotRecoveryInputViolation, TailEventMetadata, validate_recovery_data_present,
        validate_snapshot_metadata, validate_tail_run_metadata, validate_tail_seq_after_snapshot,
    };
    use vb_core::RunId;

    #[test]
    fn validate_snapshot_metadata_accepts_matching_run() {
        let run = RunId::new(1);
        let result = validate_snapshot_metadata(run, EventSeq::new(0), run);
        assert!(
            matches!(result, Ok(())),
            "matching run should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn validate_snapshot_metadata_rejects_mismatched_run() {
        let snapshot_run = RunId::new(1);
        let requested_run = RunId::new(2);
        let result = validate_snapshot_metadata(snapshot_run, EventSeq::new(5), requested_run);
        assert!(
            matches!(result, Err(SnapshotRecoveryInputViolation::SnapshotRunMismatch {
                snapshot_run: sr,
                snapshot_seq: _,
                expected_run: _
            }) if sr == RunId::new(1)),
            "should reject mismatched run, got {:?}",
            result
        );
    }

    #[test]
    fn validate_tail_run_metadata_accepts_matching_run() {
        let run = RunId::new(3);
        let meta = TailEventMetadata::new(run, EventSeq::new(0));
        let result = validate_tail_run_metadata(meta, run);
        assert!(
            matches!(result, Ok(())),
            "matching run should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn validate_tail_run_metadata_rejects_mismatched_run() {
        let run = RunId::new(4);
        let meta = TailEventMetadata::new(RunId::new(5), EventSeq::new(0));
        let result = validate_tail_run_metadata(meta, run);
        assert!(
            matches!(result, Err(SnapshotRecoveryInputViolation::TailRunMismatch {
                expected,
                actual
            }) if expected == RunId::new(4) && actual == RunId::new(5)),
            "should reject mismatched tail run, got {:?}",
            result
        );
    }

    #[test]
    fn validate_tail_seq_after_snapshot_accepts_larger_seq() {
        let meta = TailEventMetadata::new(RunId::new(6), EventSeq::new(10));
        let snapshot_seq = EventSeq::new(5);
        let result = validate_tail_seq_after_snapshot(meta, snapshot_seq);
        assert!(
            matches!(result, Ok(())),
            "larger seq should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn validate_tail_seq_after_snapshot_rejects_equal_seq() {
        let meta = TailEventMetadata::new(RunId::new(7), EventSeq::new(5));
        let snapshot_seq = EventSeq::new(5);
        let result = validate_tail_seq_after_snapshot(meta, snapshot_seq);
        assert!(
            matches!(
                result,
                Err(SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot { .. })
            ),
            "equal seq should be rejected, got {:?}",
            result
        );
    }

    #[test]
    fn validate_tail_seq_after_snapshot_rejects_smaller_seq() {
        let meta = TailEventMetadata::new(RunId::new(8), EventSeq::new(3));
        let snapshot_seq = EventSeq::new(5);
        let result = validate_tail_seq_after_snapshot(meta, snapshot_seq);
        assert!(
            matches!(
                result,
                Err(SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot { .. })
            ),
            "smaller seq should be rejected, got {:?}",
            result
        );
    }

    #[test]
    fn validate_recovery_data_present_accepts_when_tail_not_empty() {
        let result = validate_recovery_data_present(false, true, true, RunId::new(9));
        assert!(
            matches!(result, Ok(())),
            "should accept when tail is not empty, got {:?}",
            result
        );
    }

    #[test]
    fn validate_recovery_data_present_accepts_when_slots_not_empty() {
        let result = validate_recovery_data_present(true, false, true, RunId::new(10));
        assert!(
            matches!(result, Ok(())),
            "should accept when slots not empty, got {:?}",
            result
        );
    }

    #[test]
    fn validate_recovery_data_present_accepts_when_taint_not_empty() {
        let result = validate_recovery_data_present(true, true, false, RunId::new(11));
        assert!(
            matches!(result, Ok(())),
            "should accept when taint not empty, got {:?}",
            result
        );
    }

    #[test]
    fn validate_recovery_data_present_rejects_when_all_empty() {
        let run = RunId::new(12);
        let result = validate_recovery_data_present(true, true, true, run);
        assert!(
            matches!(result, Err(SnapshotRecoveryInputViolation::NoRecoveryData { run: r }) if r == run),
            "should reject when all empty, got {:?}",
            result
        );
    }

    #[test]
    fn tail_event_metadata_new_creates_with_correct_fields() {
        let run = RunId::new(13);
        let seq = EventSeq::new(42);
        let meta = TailEventMetadata::new(run, seq);
        assert_eq!(meta.run, run);
        assert_eq!(meta.seq, seq);
    }
}
