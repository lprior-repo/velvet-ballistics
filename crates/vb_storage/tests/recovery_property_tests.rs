#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
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
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
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
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, SlotIdx, StepIdx};
use vb_storage::recovery::hydrate::{
    hydrate_dimensions_positive, hydrate_events_preconditions,
    hydrate_snapshot_tail_seq_after_snapshot,
};
use vb_storage::recovery::replay::core::{
    replay_attempt_is_current, replay_attempt_is_stale, replay_step_order_diverges,
};
use vb_storage::recovery::replay::summary::recovery_dimension_count_from_index;
use vb_storage::recovery::{
    ActionReplayTracker, DigestCheck, RunSnapshot, UnsupportedRecoveryState,
};
use vb_storage::{EventSeq, JournalEvent};

fn unsupported_flags() -> impl Strategy<Value = UnsupportedRecoveryState> {
    (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(slot_values, slot_taint, action_payloads)| UnsupportedRecoveryState {
            slot_values,
            slot_taint,
            action_payloads,
        },
    )
}

proptest! {
    #[test]
    fn proptest_unsupported_recovery_state_union(left in unsupported_flags(), right in unsupported_flags()) {
        let union = left.union(right);
        prop_assert!(UnsupportedRecoveryState::SUPPORTED.is_fully_supported());
        prop_assert!(left.union_matches_flags(right, union));
    }

    #[test]
    fn proptest_seed_dimensions(max_index in prop_oneof![Just(None), any::<u16>().prop_map(Some)]) {
        let result = recovery_dimension_count_from_index(max_index, RunId::new(7));
        match (max_index, result) {
            (None, Ok(count)) => prop_assert_eq!(count, 0),
            (Some(u16::MAX), Err(_)) => {}
            (Some(index), Ok(count)) => prop_assert_eq!(count, index + 1),
            other => prop_assert!(false, "unexpected dimension result: {other:?}"),
        }
    }

    #[test]
    fn proptest_action_replay_tracker_monotonic(action in any::<u16>(), step in any::<u16>()) {
        let action = ActionId::new(action);
        let step = StepIdx::new(step);
        let mut completed = ActionReplayTracker::new();
        completed.mark_completed(action, step);
        prop_assert!(completed.has_completed(action, step));
        prop_assert!(completed.is_resolved(action, step));

        let mut failed = ActionReplayTracker::new();
        failed.mark_failed(action, step);
        prop_assert!(failed.has_failed(action, step));
        prop_assert!(failed.is_resolved(action, step));
    }

    #[test]
    fn proptest_digest_check_hierarchy(_unit in Just(())) {
        prop_assert!(DigestCheck::WorkflowSourceOnly.is_strictly_weaker_than(DigestCheck::WorkflowAndIr));
        prop_assert!(DigestCheck::WorkflowAndIr.is_strictly_weaker_than(DigestCheck::Full));
        prop_assert!(DigestCheck::Full.checks_full());
        prop_assert!(DigestCheck::Full.checks_compiled_ir());
        prop_assert!(DigestCheck::WorkflowAndIr.checks_workflow_source());
    }

    #[test]
    fn proptest_hydrate_run_frame_preconditions(snapshot_seq in 0_u64..1_000, tail_delta in 1_u64..1_000) {
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(snapshot_seq),
            workflow: vb_core::WorkflowDigest::from_bytes([1; 32]),
            slots: vec![1],
            taint: vec![],
        };
        let tail = vec![JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(snapshot_seq.saturating_add(tail_delta)),
            result: SlotIdx::new(0),
            attempt: 1,
        }];
        prop_assert!(hydrate_snapshot_tail_seq_after_snapshot(&snapshot, &tail));
        prop_assert!(hydrate_dimensions_positive(1, 1));
    }

    #[test]
    fn proptest_hydrate_run_frame_from_events_preconditions(non_empty in any::<bool>()) {
        let events = if non_empty {
            vec![JournalEvent::RunFailedEvent { run: RunId::new(1), seq: EventSeq::new(1), attempt: 1 }]
        } else {
            Vec::new()
        };
        prop_assert_eq!(hydrate_events_preconditions(&events), non_empty);
    }

    #[test]
    fn proptest_replay_events_attempt_filter(attempt in prop_oneof![Just(None), any::<u16>().prop_map(Some)], max_attempt in any::<u16>(), previous in prop_oneof![Just(None), any::<u16>().prop_map(|v| Some(StepIdx::new(v)))], current in any::<u16>()) {
        let observed = attempt.unwrap_or(1);
        prop_assert_eq!(replay_attempt_is_current(attempt, max_attempt), observed >= max_attempt);
        prop_assert_eq!(replay_attempt_is_stale(attempt, max_attempt), observed < max_attempt);
        prop_assert_eq!(replay_step_order_diverges(previous, StepIdx::new(current)), previous.is_some_and(|step| current < step.get()));
    }
}
