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
mod snapshot_tests {
    use crate::{
        DIGEST_BYTES, EventSeq, FjallJournal, RunSnapshot,
    };
    use vb_core::{RunId, WorkflowDigest};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_snapshot(run: RunId, seq: u64, digest: WorkflowDigest) -> RunSnapshot {
        RunSnapshot {
            run,
            seq: EventSeq::new(seq),
            workflow: digest,
            slots: vec![0x01, 0x02, 0x03],
            taint: vec![0x00],
        }
    }

    #[test]
    fn put_snapshot_stores_and_retrieves_compact_snapshot() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1);
        let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);
        let snapshot = make_snapshot(run, 5, digest);

        journal.put_snapshot(&snapshot).expect("put_snapshot should succeed");

        let loaded = journal
            .snapshot(run, EventSeq::new(5))
            .expect("snapshot lookup should succeed")
            .expect("snapshot should exist");
        assert_eq!(loaded.run, run);
        assert_eq!(loaded.seq, EventSeq::new(5));
        assert_eq!(loaded.workflow, digest);
        assert_eq!(loaded.slots, vec![0x01, 0x02, 0x03]);
        assert_eq!(loaded.taint, vec![0x00]);
    }

    #[test]
    fn snapshot_returns_none_for_missing_run() {
        let (_temp, journal) = temp_journal();
        let missing_run = RunId::new(999);
        let result = journal.snapshot(missing_run, EventSeq::new(0));
        let found = result.expect("lookup should succeed");
        assert!(found.is_none(), "should return None for missing snapshot");
    }

    #[test]
    fn snapshot_returns_none_for_missing_sequence() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2);
        let digest = WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]);
        let snapshot = make_snapshot(run, 3, digest);
        journal.put_snapshot(&snapshot).expect("put should succeed");

        let result = journal.snapshot(run, EventSeq::new(7));
        let found = result.expect("lookup should succeed");
        assert!(
            found.is_none(),
            "should return None for non-existent sequence"
        );
    }

    #[test]
    fn multiple_snapshots_for_same_run_are_stored_independently() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3);
        let d1 = WorkflowDigest::from_bytes([0x33; DIGEST_BYTES]);
        let d2 = WorkflowDigest::from_bytes([0x44; DIGEST_BYTES]);

        let s1 = RunSnapshot {
            run,
            seq: EventSeq::new(1),
            workflow: d1,
            slots: vec![1],
            taint: vec![],
        };
        let s2 = RunSnapshot {
            run,
            seq: EventSeq::new(3),
            workflow: d2,
            slots: vec![2],
            taint: vec![10],
        };

        journal.put_snapshot(&s1).expect("put s1 should succeed");
        journal.put_snapshot(&s2).expect("put s2 should succeed");

        let loaded1 = journal
            .snapshot(run, EventSeq::new(1))
            .expect("get s1")
            .expect("s1 should exist");
        assert_eq!(loaded1.workflow, d1);
        assert_eq!(loaded1.slots, vec![1]);

        let loaded2 = journal
            .snapshot(run, EventSeq::new(3))
            .expect("get s2")
            .expect("s2 should exist");
        assert_eq!(loaded2.workflow, d2);
        assert_eq!(loaded2.slots, vec![2]);
    }

    #[test]
    fn write_snapshot_convenience_wrapper_works() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4);
        let digest = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(10),
            workflow: digest,
            slots: vec![0xAA],
            taint: vec![],
        };

        crate::write_snapshot(&journal, &snapshot).expect("write_snapshot should succeed");

        let loaded = journal
            .snapshot(run, EventSeq::new(10))
            .expect("get should succeed")
            .expect("snapshot should exist");
        assert_eq!(loaded.slots, vec![0xAA]);
    }

    #[test]
    fn snapshot_with_empty_slots_and_taint_is_stored_and_retrieved() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(5);
        let digest = WorkflowDigest::from_bytes([0x66; DIGEST_BYTES]);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
            slots: vec![],
            taint: vec![],
        };

        journal.put_snapshot(&snapshot).expect("put should succeed");
        let loaded = journal
            .snapshot(run, EventSeq::new(0))
            .expect("get should succeed")
            .expect("should exist");
        assert!(loaded.slots.is_empty());
        assert!(loaded.taint.is_empty());
    }

    #[test]
    fn snapshot_sequence_max_value_works() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(6);
        let digest = WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]);
        let snapshot = make_snapshot(run, u64::MAX, digest);

        journal.put_snapshot(&snapshot).expect("put at MAX seq should succeed");
        let loaded = journal
            .snapshot(run, EventSeq::MAX)
            .expect("get should succeed")
            .expect("should exist");
        assert_eq!(loaded.seq, EventSeq::MAX);
    }

    #[test]
    fn snapshot_run_mismatch_still_stores_and_retrieves_correctly() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(7);
        let run_b = RunId::new(8);
        let digest = WorkflowDigest::from_bytes([0x88; DIGEST_BYTES]);

        let sa = make_snapshot(run_a, 1, digest);
        let sb = make_snapshot(run_b, 1, digest);

        journal.put_snapshot(&sa).expect("put run_a snapshot");
        journal.put_snapshot(&sb).expect("put run_b snapshot");

        let la = journal
            .snapshot(run_a, EventSeq::new(1))
            .expect("get run_a")
            .expect("run_a should exist");
        let lb = journal
            .snapshot(run_b, EventSeq::new(1))
            .expect("get run_b")
            .expect("run_b should exist");
        assert_eq!(la.run, run_a);
        assert_eq!(lb.run, run_b);
    }
}
