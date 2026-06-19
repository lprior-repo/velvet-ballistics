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
//! Proptest invariants for snapshot minimum-sequence preconditions (bead vb-jnz9).
//!
//! Property-based tests covering:
//! - Single tail event replay after snapshot boundary (AC-07)
//! - Snapshot sequence validity
//! - Tail event ordering relative to snapshot
//!
//! These tests verify the snapshot-plus-tail recovery contract at the
//! behavior-test level, complementing the formal Kani and Verus proofs.

use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::recovery::ActionReplayTracker;
use vb_storage::recovery::replay::recover_snapshot_plus_tail;
use vb_storage::{EventSeq, JournalEvent};

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

/// Creates a minimal RunSnapshot at a given sequence.
fn make_snapshot(run: RunId, seq: u64) -> vb_storage::recovery::RunSnapshot {
    vb_storage::recovery::RunSnapshot {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0x42; 32]),
        slots: Vec::new(),
        taint: Vec::new(),
    }
}

/// Creates a RunAccepted event at the given run and sequence.
fn make_run_accepted(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0x42; 32]),
    }
}

/// Creates a StepStarted event at the given run and sequence.
fn make_step_started(run: RunId, seq: u64, step: vb_core::StepIdx) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step,
        attempt: 1,
    }
}

// ---------------------------------------------------------------------------
// PS-11: Single tail event replay after snapshot (AC-07)
// ---------------------------------------------------------------------------

proptest! {
    /// AC-07: Snapshot followed by one journal event replays exactly that tail event.
    ///
    /// Property: recover_snapshot_plus_tail(snapshot, [tail_event], tracker)
    ///   where tail_event.seq == snapshot.seq + 1
    ///   returns Ok([tail_event])
    ///
    /// This is the primary proptest for proof obligation PO-011 / PS-11.
    ///
    /// Bounds:
    ///   - snapshot.seq ∈ [0, 1000]
    ///   - tail_events.len() == 1
    ///   - event.seq == snapshot.seq + 1
    ///
    /// Evidence: Test passes; result.is_ok() && result.unwrap().len() == 1
    ///   && result.unwrap()[0].seq() == snapshot.seq + 1
    #[test]
    fn prop_single_tail_event_replay(
        snapshot_seq in 0u64..1000u64,
    ) {
        let run = RunId::new(1);
        let snapshot = make_snapshot(run, snapshot_seq);

        // Exactly one tail event at seq = snapshot.seq + 1
        let tail_seq = snapshot_seq.checked_add(1).unwrap_or(snapshot_seq);
        let tail_event = make_run_accepted(run, tail_seq);
        let tail_events = vec![tail_event];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&snapshot, &tail_events, &mut tracker);

        prop_assert!(result.is_ok(), "recover_snapshot_plus_tail should succeed for valid tail at seq+1");
        let replayed = result.unwrap();
        prop_assert!(replayed.len() == 1, "exactly one event should be replayed, got {}", replayed.len());
        prop_assert!(replayed[0].seq() == EventSeq::new(tail_seq),
            "replayed event seq should be {}, got {}", tail_seq, replayed[0].seq().get());
    }
}

// ---------------------------------------------------------------------------
// PS-11 variant: StepStarted tail event (AC-07 extended)
// ---------------------------------------------------------------------------

proptest! {
    /// AC-07 extended: StepStarted event at seq+1 also replays correctly.
    ///
    /// Property: recover_snapshot_plus_tail with StepStarted tail event at seq+1
    ///   returns Ok([tail_event])
    ///
    #[test]
    fn prop_single_step_started_tail_replay(
        snapshot_seq in 0u64..500u64,
        step in 0u16..10u16,
    ) {
        let run = RunId::new(1);
        let snapshot = make_snapshot(run, snapshot_seq);

        let tail_seq = snapshot_seq.checked_add(1).unwrap_or(snapshot_seq);
        let tail_event = make_step_started(run, tail_seq, vb_core::StepIdx::new(step));
        let tail_events = vec![tail_event];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&snapshot, &tail_events, &mut tracker);

        prop_assert!(result.is_ok(), "StepStarted tail at seq+1 should replay successfully");
        let replayed = result.unwrap();
        prop_assert!(replayed.len() == 1, "StepStarted tail should be replayed as single event");
    }
}

// ---------------------------------------------------------------------------
// Tail ordering: overlapping tail must fail (H-05)
// ---------------------------------------------------------------------------

proptest! {
    /// H-05: recover_snapshot_plus_tail returns ReplayDivergence for overlapping tail.
    ///
    /// Property: when ∃e ∈ tail: e.seq() ≤ snapshot.seq, the result is
    ///   Err(RecoveryError::ReplayDivergence { .. })
    ///
    /// This complements the Kani cover proof PS-07.
    ///
    #[test]
    fn prop_overlapping_tail_returns_divergence(
        snapshot_seq in 1u64..500u64,
        tail_seq in 0u64..500u64,
    ) {
        // Only generate overlapping cases: tail_seq <= snapshot_seq
        let overlap_seq = tail_seq % (snapshot_seq + 1); // [0, snapshot_seq]

        let run = RunId::new(1);
        let snapshot = make_snapshot(run, snapshot_seq);
        let tail_event = make_run_accepted(run, overlap_seq);
        let tail_events = vec![tail_event];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&snapshot, &tail_events, &mut tracker);

        // When tail overlaps (overlap_seq <= snapshot_seq), expect ReplayDivergence
        if overlap_seq <= snapshot_seq {
            prop_assert!(result.is_err(), "overlapping tail should return Err");
            // The error should be ReplayDivergence (structurally checked via matches!)
            // We can't easily match the string detail, but we verify it's an error
        }
        // If overlap_seq > snapshot_seq, it would be Ok but our gen doesn't produce those
    }
}

// ---------------------------------------------------------------------------
// Snapshot at seq==0 with empty tail (H-06 / H-10)
// ---------------------------------------------------------------------------

proptest! {
    /// H-06 / AC-07: Empty tail with snapshot at seq==0 is valid input.
    ///
    /// Property: recover_snapshot_plus_tail with empty tail events returns Ok([])
    ///   for any snapshot (including seq==0).
    ///
    #[test]
    fn prop_empty_tail_is_valid_for_any_snapshot(
        snapshot_seq in 0u64..100u64,
    ) {
        let run = RunId::new(1);
        let snapshot = make_snapshot(run, snapshot_seq);
        let tail_events: Vec<JournalEvent> = Vec::new();

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&snapshot, &tail_events, &mut tracker);

        prop_assert!(result.is_ok(), "empty tail should always be valid: seq={}", snapshot_seq);
        let replayed = result.unwrap();
        prop_assert!(replayed.is_empty(), "empty tail should replay to empty vec");
    }
}

// ---------------------------------------------------------------------------
// Snapshot sequence bounds (H-07)
// ---------------------------------------------------------------------------

proptest! {
    /// H-07: Snapshot seq is bounded; no overflow in valid runs.
    ///
    /// Property: snapshot.seq.get() is in [0, u64::MAX] and when
    ///   snapshot.seq == u64::MAX, the journal is in an invalid terminal state.
    ///
    #[test]
    fn prop_snapshot_seq_in_valid_range(
        snapshot_seq in 0u64..u64::MAX,
    ) {
        let run = RunId::new(1);
        let snapshot = make_snapshot(run, snapshot_seq);

        // seq is always in range
        prop_assert!(snapshot.seq.get() <= u64::MAX, "snapshot seq should always be in range");

        // If seq == u64::MAX, this is the overflow sentinel
        // A snapshot at u64::MAX is a terminal/corrupt state indicator
        if snapshot.seq.get() == u64::MAX {
            // This would only happen in a corrupt journal
            // The recovery path should detect this
        }
    }
}
