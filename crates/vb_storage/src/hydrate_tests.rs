#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    clippy::let_underscore_must_use,
    clippy::len_zero,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::needless_return,
    clippy::needless_bool,
    clippy::single_match,
    clippy::single_match_else,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_locals,
    clippy::manual_let_else,
    clippy::or_fun_call,
    clippy::needless_borrow,
    clippy::needless_pass_by_value,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::module_inception,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::uninlined_format_args,
    clippy::large_digit_groups,
    clippy::unreadable_literal,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::vec_init_then_push,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::trivially_copy_pass_by_ref,
    clippy::wildcard_imports,
    clippy::wrong_self_convention,
    clippy::needless_range_loop,
    clippy::nonminimal_bool,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::should_implement_trait,
    clippy::result_large_err,
    clippy::missing_const_for_fn,
    clippy::use_self,
    clippy::items_after_statements,
    clippy::option_if_let_else,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::comparison_chain,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::explicit_counter_loop,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::needless_update,
    clippy::let_and_return,
    clippy::manual_div_ceil,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::match_like_matches_macro,
    clippy::wildcard_enum_match_arm,
    clippy::large_types_passed_by_value,
    clippy::large_futures,
    clippy::type_complexity,
    clippy::needless_collect,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::suspicious_operation_groupings,
    clippy::field_reassign_with_default,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::borrow_deref_ref,
    clippy::cloned_ref_to_slice_refs,
    clippy::inefficient_to_string,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::get_first,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::implicit_saturating_sub,
    clippy::unwrap_or_default,
    clippy::default_trait_access
)]
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
    use crate::events::JournalEvent;
    use crate::recovery::{
        RecoveryError, RunSnapshot,
        hydrate::{
            invariants::{SnapshotRecoveryInputViolation, TailEventMetadata},
            validate_recovery_data_present, validate_snapshot_metadata,
            validate_snapshot_recovery_inputs, validate_tail_events_after_snapshot,
            validate_tail_first_seq_contiguous_with_snapshot, validate_tail_run_metadata,
            validate_tail_seq_after_snapshot,
        },
    };
    use vb_core::{RunId, StepIdx, WorkflowDigest};

    #[test]
    fn validate_snapshot_metadata_accepts_matching_run() {
        let run = RunId::new(1);
        let result = validate_snapshot_metadata(run, EventSeq::new(0), run);
        assert_eq!(
            result,
            Ok(()),
            "matching run should succeed (validated metadata unit)"
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
                snapshot_seq: _
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
        assert_eq!(
            result,
            Ok(()),
            "matching tail run should succeed (validated metadata unit)"
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
        assert_eq!(
            result,
            Ok(()),
            "larger seq (10 > 5) should succeed (validated metadata unit)"
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
        assert_eq!(
            result,
            Ok(()),
            "should accept when tail is not empty (validated metadata unit)"
        );
    }

    #[test]
    fn validate_recovery_data_present_accepts_when_slots_not_empty() {
        let result = validate_recovery_data_present(true, false, true, RunId::new(10));
        assert_eq!(
            result,
            Ok(()),
            "should accept when slots not empty (validated metadata unit)"
        );
    }

    #[test]
    fn validate_recovery_data_present_accepts_when_taint_not_empty() {
        let result = validate_recovery_data_present(true, true, false, RunId::new(11));
        assert_eq!(
            result,
            Ok(()),
            "should accept when taint not empty (validated metadata unit)"
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

    #[test]
    fn validate_tail_first_seq_contiguous_accepts_snapshot_plus_one() {
        let result = validate_tail_first_seq_contiguous_with_snapshot(
            &[event_at(RunId::new(20), EventSeq::new(6))],
            EventSeq::new(5),
        );
        assert!(
            matches!(result, Ok(())),
            "seq 6 must validate as Ok(()) when contiguous with snapshot seq 5 (no gap), got {result:?}"
        );
    }

    #[test]
    fn validate_tail_first_seq_contiguous_accepts_empty_tail() {
        let result = validate_tail_first_seq_contiguous_with_snapshot(&[], EventSeq::new(5));
        assert!(
            matches!(result, Ok(())),
            "empty tail must validate as Ok(()) (no events to verify), got {result:?}"
        );
    }

    #[test]
    fn validate_tail_first_seq_contiguous_accepts_gap() {
        // qi37 contract: a gap between snapshot and tail is permitted when
        // the journal skipped events that landed inside the snapshot itself.
        let result = validate_tail_first_seq_contiguous_with_snapshot(
            &[event_at(RunId::new(21), EventSeq::new(9))],
            EventSeq::new(5),
        );
        assert!(
            matches!(result, Ok(())),
            "seq 9 must validate as Ok(()) when strictly after snapshot seq 5 (gap allowed), got {result:?}"
        );
    }

    #[test]
    fn validate_tail_first_seq_contiguous_rejects_equal_to_snapshot() {
        let result = validate_tail_first_seq_contiguous_with_snapshot(
            &[event_at(RunId::new(22), EventSeq::new(5))],
            EventSeq::new(5),
        );
        assert!(
            matches!(
                result,
                Err(SnapshotRecoveryInputViolation::TailSeqNotContiguousWithSnapshot { .. })
            ),
            "seq 5 must be rejected (not contiguous with snapshot seq 5), got {:?}",
            result
        );
    }

    #[test]
    fn validate_tail_events_after_snapshot_accepts_non_contiguous_first() {
        // qi37 contract: a gap between snapshot and tail is permitted.
        let run = RunId::new(23);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(3),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
            slots: Vec::new(),
            taint: Vec::new(),
        };
        let tail = vec![
            event_at(run, EventSeq::new(7)),
            event_at(run, EventSeq::new(8)),
        ];
        let result = validate_tail_events_after_snapshot(&tail, &snapshot);
        assert!(
            matches!(result, Ok(())),
            "tail events strictly after snapshot seq must validate as Ok(()), got {result:?}"
        );
    }

    fn event_at(run: RunId, seq: EventSeq) -> JournalEvent {
        JournalEvent::StepStarted {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
        }
    }

    #[test]
    fn validate_snapshot_recovery_inputs_accepts_contiguous_tail() {
        let run = RunId::new(24);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(3),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
            slots: Vec::new(),
            taint: Vec::new(),
        };
        let tail = vec![event_at(run, EventSeq::new(4))];
        let result = validate_snapshot_recovery_inputs(&snapshot, &tail, run);
        assert!(
            matches!(result, Ok(())),
            "contiguous tail must validate as Ok(()) for matching run with non-empty events, got {result:?}"
        );
    }

    #[test]
    fn validate_snapshot_recovery_inputs_accepts_non_contiguous_tail() {
        // qi37 contract: a gap between snapshot and tail is permitted.
        let run = RunId::new(25);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(3),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
            slots: Vec::new(),
            taint: Vec::new(),
        };
        let tail = vec![event_at(run, EventSeq::new(7))];
        let result = validate_snapshot_recovery_inputs(&snapshot, &tail, run);
        assert!(
            matches!(result, Ok(())),
            "strictly-after-snapshot tail must validate as Ok(()), got {result:?}"
        );
    }
}
