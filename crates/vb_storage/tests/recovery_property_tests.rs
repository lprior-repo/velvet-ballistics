#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports)]

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
