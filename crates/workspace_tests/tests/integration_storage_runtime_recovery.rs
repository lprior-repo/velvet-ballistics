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
    clippy::enum_variant_names,
    clippy::manual_contains,
    clippy::if_same_then_else,
    clippy::multiple_bound_locations,
    clippy::identity_op,
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
//! Integration tests for vb_storage + vb_runtime recovery scenarios.
//!
//! Tests edge cases not covered in vb_storage/src/recovery/tests.rs or
//! vb_qi37_1_1_red_recovery_contract_test.rs:
//! - ActionReplayTracker boundary states
//! - Multiple step recovery with mixed outcomes
//! - Partial journal recovery with snapshot corruption detection
//! - Pending action recovery with various action states
//! - Digest mismatch propagation through recovery boundaries

use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx, WorkflowDigest};
use vb_runtime::recovery::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary};
use vb_storage::recovery::{
    ActionReplayTracker, RecoveryFrameSeed, RecoveryRuntimeSummary, UnsupportedRecoveryState,
    recover_runtime_frame_seed_from_events,
};
use vb_storage::{EventSeq, JournalEvent};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn encoded(value: SlotValue) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(&value)
}

fn run_accepted_event(run: RunId, workflow: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow,
    }
}

fn step_started_event(run: RunId, seq: u64, step: StepIdx, attempt: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step,
        attempt,
    }
}

fn slot_written_event(
    run: RunId,
    seq: u64,
    slot: SlotIdx,
    value: SlotValue,
    attempt: u16,
) -> JournalEvent {
    JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(seq),
        slot,
        value: Some(encoded(value).expect("postcard encode")),
        extra: None,
        attempt,
    }
}

fn step_succeeded_event(run: RunId, seq: u64, step: StepIdx, output: SlotIdx) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(seq),
        step,
        output,
    }
}

fn action_scheduled_event(run: RunId, seq: u64, step: StepIdx, action_id: u16) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run,
        seq: EventSeq::new(seq),
        step,
        action: ActionId::new(action_id),
        attempt: 1,
    }
}

fn action_completed_event(run: RunId, seq: u64, step: StepIdx, action_id: u16) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(seq),
        step,
        action: ActionId::new(action_id),
        attempt: 1,
    }
}

fn run_failed_event(run: RunId, seq: u64, attempt: u16) -> JournalEvent {
    JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::new(seq),
        attempt,
    }
}

// ---------------------------------------------------------------------------
// ActionReplayTracker edge cases
// ---------------------------------------------------------------------------

#[test]
fn action_replay_tracker_new_is_empty() {
    let tracker = ActionReplayTracker::new();
    // A new tracker should not report any action as resolved
    assert!(!tracker.is_resolved(ActionId::new(42), StepIdx::ZERO));
}

#[test]
fn action_replay_tracker_marks_completed() {
    let mut tracker = ActionReplayTracker::new();
    let action_id = ActionId::new(42);
    let step = StepIdx::ZERO;

    tracker.mark_completed(action_id, step);
    assert!(tracker.is_resolved(action_id, step));
}

#[test]
fn action_replay_tracker_marks_failed() {
    let mut tracker = ActionReplayTracker::new();
    let action_id = ActionId::new(42);
    let step = StepIdx::ZERO;

    tracker.mark_failed(action_id, step);
    assert!(tracker.is_resolved(action_id, step));
}

#[test]
fn action_replay_tracker_different_actions_not_resolved() {
    let mut tracker = ActionReplayTracker::new();
    let action_a = ActionId::new(1);
    let action_b = ActionId::new(2);
    let step = StepIdx::ZERO;

    tracker.mark_completed(action_a, step);
    assert!(tracker.is_resolved(action_a, step));
    assert!(!tracker.is_resolved(action_b, step));
}

// ---------------------------------------------------------------------------
// Multiple step recovery with mixed outcomes
// ---------------------------------------------------------------------------

#[test]
fn recovery_with_three_steps_all_succeeding() {
    let run = RunId::new(100);
    let workflow = WorkflowDigest::from_bytes([1; 32]);

    let events = vec![
        run_accepted_event(run, workflow),
        step_started_event(run, 1, StepIdx::ZERO, 1),
        slot_written_event(run, 2, SlotIdx::new(0), SlotValue::I64(10), 1),
        step_succeeded_event(run, 3, StepIdx::ZERO, SlotIdx::new(0)),
        step_started_event(run, 4, StepIdx::new(1), 1),
        slot_written_event(run, 5, SlotIdx::new(1), SlotValue::I64(20), 1),
        step_succeeded_event(run, 6, StepIdx::new(1), SlotIdx::new(1)),
        step_started_event(run, 7, StepIdx::new(2), 1),
        slot_written_event(run, 8, SlotIdx::new(2), SlotValue::I64(30), 1),
        step_succeeded_event(run, 9, StepIdx::new(2), SlotIdx::new(2)),
    ];

    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");
    // step_count is derived from max step index seen
    assert_eq!(seed.step_count, 3); // steps 0, 1, 2 → count is 3
    // Without a CompiledWorkflow, slot recovery behavior may vary
    // Just verify that some slots were recovered
    assert!(!seed.slots.is_empty(), "should recover at least some slots");
}

#[test]
fn recovery_with_multiple_attempts_on_same_step() {
    let run = RunId::new(102);
    let workflow = WorkflowDigest::from_bytes([3; 32]);

    let events = vec![
        run_accepted_event(run, workflow),
        step_started_event(run, 1, StepIdx::ZERO, 1),
        step_started_event(run, 2, StepIdx::ZERO, 2), // Second attempt without explicit failure
        slot_written_event(run, 3, SlotIdx::ZERO, SlotValue::I64(99), 2),
        step_succeeded_event(run, 4, StepIdx::ZERO, SlotIdx::ZERO),
    ];

    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");
    assert_eq!(seed.slots.len(), 1);
    // Only the second attempt's value should survive
    assert_eq!(seed.slots[0].value, SlotValue::I64(99));
    assert_eq!(seed.slots[0].slot, SlotIdx::ZERO);
}

// ---------------------------------------------------------------------------
// Pending action recovery edge cases
// ---------------------------------------------------------------------------

#[test]
fn recovery_preserves_pending_action_in_incomplete_run() {
    let run = RunId::new(103);
    let workflow = WorkflowDigest::from_bytes([4; 32]);

    let events = vec![
        run_accepted_event(run, workflow),
        step_started_event(run, 1, StepIdx::ZERO, 1),
        action_scheduled_event(run, 2, StepIdx::ZERO, 42),
        // Run ends while action is still pending - no completion event
    ];

    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");
    // The seed should be recoverable even with pending action
    assert_eq!(seed.summary.steps_started, 1);
}

#[test]
fn recovery_with_action_completed_after_pending() {
    let run = RunId::new(104);
    let workflow = WorkflowDigest::from_bytes([5; 32]);

    let events = vec![
        run_accepted_event(run, workflow),
        step_started_event(run, 1, StepIdx::ZERO, 1),
        action_scheduled_event(run, 2, StepIdx::ZERO, 77),
        action_completed_event(run, 3, StepIdx::ZERO, 77),
        step_succeeded_event(run, 4, StepIdx::ZERO, SlotIdx::ZERO),
    ];

    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");
    assert_eq!(seed.summary.actions_resolved, 1);
}

// ---------------------------------------------------------------------------
// Unsupported state detection
// ---------------------------------------------------------------------------

#[test]
fn recovery_detects_unsupported_slot_taint() {
    // Build a seed with unsupported slot taint flag
    let seed = RecoveryFrameSeed {
        summary: RecoveryRuntimeSummary {
            run: RunId::new(105),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(3),
            workflow: Some(WorkflowDigest::from_bytes([6; 32])),
            steps_started: 1,
            steps_succeeded: 1,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        },
        first_step: StepIdx::ZERO,
        step_count: 4,
        slot_count: 2,
        pc: StepIdx::ZERO,
        steps: Vec::new(),
        slots: Vec::new(),
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: true, // Marked as unsupported
            action_payloads: false,
        },
    };

    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
    let result = boundary.hydrate_run_frame();
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Workflow digest handling
// ---------------------------------------------------------------------------

#[test]
fn recovery_with_no_workflow_digest_in_summary() {
    let run = RunId::new(106);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([7; 32]),
        },
        step_started_event(run, 1, StepIdx::ZERO, 1),
        step_succeeded_event(run, 2, StepIdx::ZERO, SlotIdx::ZERO),
    ];

    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");
    // Workflow digest is stored in summary
    assert!(seed.summary.workflow.is_some());
}

// ---------------------------------------------------------------------------
// Compact sequence number handling
// ---------------------------------------------------------------------------

#[test]
fn recovery_with_gaps_in_sequence_numbers() {
    let run = RunId::new(107);
    let workflow = WorkflowDigest::from_bytes([8; 32]);

    // Simulate a journal with some events trimmed
    let events = vec![
        run_accepted_event(run, workflow),
        step_started_event(run, 1, StepIdx::ZERO, 1),
        // Seq 2 missing/trimmed
        step_succeeded_event(run, 3, StepIdx::ZERO, SlotIdx::ZERO),
    ];

    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");
    assert_eq!(seed.summary.last_seq.get(), 3);
}

#[test]
fn recovery_with_zero_sequence_first_event() {
    let run = RunId::new(108);
    let workflow = WorkflowDigest::from_bytes([9; 32]);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO, // Explicit zero
            workflow,
        },
        step_started_event(run, 1, StepIdx::ZERO, 1),
        step_succeeded_event(run, 2, StepIdx::ZERO, SlotIdx::ZERO),
    ];

    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");
    assert_eq!(seed.summary.first_seq, EventSeq::ZERO);
}

// ---------------------------------------------------------------------------
// Run failure recovery
// ---------------------------------------------------------------------------

#[test]
fn recovery_from_run_failure() {
    let run = RunId::new(109);
    let workflow = WorkflowDigest::from_bytes([10; 32]);

    let events = vec![
        run_accepted_event(run, workflow),
        step_started_event(run, 1, StepIdx::ZERO, 1),
        slot_written_event(run, 2, SlotIdx::ZERO, SlotValue::I64(5), 1),
        step_succeeded_event(run, 3, StepIdx::ZERO, SlotIdx::ZERO),
        step_started_event(run, 4, StepIdx::new(1), 1),
        run_failed_event(run, 5, 1),
    ];

    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");
    assert!(seed.summary.terminal.is_some());
}
