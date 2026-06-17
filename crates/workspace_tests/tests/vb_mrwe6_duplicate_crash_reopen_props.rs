#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::derivable_impls,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::io_other_error,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::new_without_default,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::mrwe6_seams::{
    Mrwe6DuplicateRetryDecision, Mrwe6EventClass, mrwe6_duplicate_retry_decision_from_facts,
};
use vb_storage::{EventSeq, FjallJournal, JournalError, JournalEvent};

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..Default::default()
    }
}

fn scheduled(run: RunId, step: StepIdx, action: ActionId, seq: u64, attempt: u16) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run,
        seq: EventSeq::new(seq),
        step,
        action,
        attempt,
    }
}

fn completed(run: RunId, step: StepIdx, action: ActionId, seq: u64, attempt: u16) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(seq),
        step,
        action,
        attempt,
    }
}

fn unrelated(run: RunId, seq: u64, attempt: u16) -> JournalEvent {
    JournalEvent::RunKilled {
        run,
        seq: EventSeq::new(seq),
        attempt,
    }
}

proptest! {
    #![proptest_config(config())]
    #[test]
    fn vb_mrwe6_duplicate_schedule_crash_reopen_idempotent_or_conflict(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(), divergent in any::<bool>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let first = scheduled(run, step, action, 0, 7);
        let retry = scheduled(run, step, action, 0, if divergent { 8 } else { 7 });
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&first).expect("append first");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let result = reopened.append_strict(&retry);
        if divergent {
            prop_assert!(matches!(result, Err(JournalError::DuplicateEvent { .. })), "divergent retry must return duplicate conflict");
        } else {
            prop_assert!(result.is_ok());
        }
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(reopened.has_action_index_entry(key).expect("index read"));
    }

    #[test]
    fn vb_mrwe6_equal_duplicate_requires_existing_pending_marker_after_reopen(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let first = scheduled(run, step, action, 0, 7);
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&first).expect("append first");
        journal.delete_action_index(action, run, step).expect("remove marker to model parity defect");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let result = reopened.append_strict(&first);

        prop_assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "equal retry without pending marker must be rejected as DuplicateEvent"
        );
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(!reopened.has_action_index_entry(key).expect("index read"));
    }
}

#[test]
fn vb_mrwe6_duplicate_decision_matrix_rejects_divergent_missing_and_unsupported_classes() {
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Scheduled, true),
        Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(false, Mrwe6EventClass::Scheduled, true),
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Scheduled, false),
        Mrwe6DuplicateRetryDecision::MissingExpectedIndexState
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Resolution, false),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Resolution, true),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Unrelated, false),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Unrelated, true),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(false, Mrwe6EventClass::Resolution, false),
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(false, Mrwe6EventClass::Unrelated, true),
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
    );
}

#[test]
fn vb_mrwe6_public_journal_rejects_equal_resolution_duplicate_after_reopen() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run = RunId::new(100);
    let step = StepIdx::new(4);
    let action = ActionId::new(15);
    let resolution = completed(run, step, action, 2, 1);
    let mut journal = FjallJournal::open(temp.path(), None).expect("open");
    journal
        .append_strict(&resolution)
        .expect("append resolution");
    journal.close().expect("close");
    drop(journal);

    let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
    let result = reopened.append_strict(&resolution);

    assert!(
        matches!(
            result,
            Err(JournalError::DuplicateEvent { run: actual_run, seq })
                if actual_run == run && seq == EventSeq::new(2)
        ),
        "resolution retry must reject with exact DuplicateEvent run/seq"
    );
}

#[test]
fn vb_mrwe6_public_journal_unrelated_duplicates_do_not_use_mrwe6_duplicate_retry_kernel() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run = RunId::new(101);
    let event = unrelated(run, 3, 1);
    let mut journal = FjallJournal::open(temp.path(), None).expect("open");
    journal.append_strict(&event).expect("append unrelated");
    journal.close().expect("close");
    drop(journal);

    let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
    let result = reopened.append_strict(&event);

    assert!(
        matches!(
            result,
            Err(JournalError::DuplicateEvent { run: actual_run, seq })
                if actual_run == run && seq == EventSeq::new(3)
        ),
        "unrelated duplicate must reject with exact DuplicateEvent run/seq"
    );
}
