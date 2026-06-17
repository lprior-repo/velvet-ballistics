#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::mrwe6_seams::{Mrwe6RecoveryOutcome, mrwe6_recovery_outcome};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits};

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..Default::default()
    }
}

fn scheduled(run: RunId, step: StepIdx, action: ActionId, seq: u64) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run,
        seq: EventSeq::new(seq),
        step,
        action,
        attempt: 1,
    }
}

fn completed(run: RunId, step: StepIdx, action: ActionId, seq: u64) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(seq),
        step,
        action,
        attempt: 1,
    }
}

proptest! {
    #![proptest_config(config())]
    #[test]
    fn vb_mrwe6_recovery_index_action_inventory_matches_unresolved_events(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(), resolved in any::<bool>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&schedule).expect("schedule");
        if resolved {
            let completion = completed(run, step, action, 1);
            journal.append_strict(&completion).expect("completion");
        }
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert_eq!(reopened.has_action_index_entry(key).expect("index read"), !resolved);
        prop_assert!(reopened.events_for_run(run).expect("events").contains(&schedule));
    }

    #[test]
    fn vb_mrwe6_recovery_reports_parity_defect_when_scheduled_marker_is_missing(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&schedule).expect("schedule");
        journal.delete_action_index(action, run, step).expect("delete marker");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");

        prop_assert!(!reopened.has_action_index_entry(key).expect("index read"));
        prop_assert_eq!(
            mrwe6_recovery_outcome(&schedule, None, false, false).expect("recovery outcome"),
            Mrwe6RecoveryOutcome::ParityDefect
        );
    }

    #[test]
    fn vb_mrwe6_recovery_ignores_unflushed_queue_entries_after_reopen(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        let queue = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        queue.enqueue_journaled(schedule).expect("enqueue only");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");

        prop_assert!(reopened.events_for_run(run).expect("events").is_empty());
        prop_assert!(!reopened.has_action_index_entry(key).expect("index read"));
    }
}

#[test]
fn vb_mrwe6_recovery_outcome_matrix_keeps_fallback_and_defect_explicit() {
    let run = RunId::new(10);
    let step = StepIdx::new(2);
    let action = ActionId::new(3);
    let schedule = scheduled(run, step, action, 0);
    let completion = completed(run, step, action, 1);
    let mismatched_completion = completed(run, step, ActionId::new(4), 2);

    assert_eq!(
        mrwe6_recovery_outcome(&schedule, None, true, false).expect("pending inventory outcome"),
        Mrwe6RecoveryOutcome::PendingInventory
    );
    assert_eq!(
        mrwe6_recovery_outcome(&schedule, Some(&completion), false, false)
            .expect("resolved no pending outcome"),
        Mrwe6RecoveryOutcome::ResolvedNoPending
    );
    assert_eq!(
        mrwe6_recovery_outcome(&schedule, Some(&mismatched_completion), true, false)
            .expect("parity defect outcome"),
        Mrwe6RecoveryOutcome::ParityDefect
    );
    assert_eq!(
        mrwe6_recovery_outcome(&schedule, None, false, true).expect("legacy fallback outcome"),
        Mrwe6RecoveryOutcome::LegacyFallback
    );
}
