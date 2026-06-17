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
    unused_variables,
)]

#![forbid(unsafe_code)]
//! vb-40cfh — recovery_replay proptest for `vb_storage::recovery::replay_events`.
//!
//! Generates random valid `JournalEvent` sequences, applies a fuzz-style
//! recovery-layer mutation to the sequence, then feeds the mutated sequence
//! through `recovery::replay_events` and asserts the typed
//! `RecoveryError` variant matches the mutation class.
//!
//! ## Recovery-layer mutation classes
//!
//! The five classes cover the named recovery invariants that the
//! `replay_events` boundary enforces:
//! 1. `TruncateEventSequence` — drop an event from the middle of the
//!    sequence so the recovery layer observes a sequence gap.
//!    `validate_contiguous_sequences` returns
//!    `RecoveryError::ReplayDivergence` with a `sequence violation` detail.
//! 2. `DuplicateEventSeq` — append a duplicate `ActionCompletedEvent`
//!    twice. The duplicate seq trips `validate_contiguous_sequences`
//!    before the per-event idempotency check, so the surface variant is
//!    `RecoveryError::ReplayDivergence` with a `sequence violation`
//!    detail (not `NonIdempotentActionBlocked`, which only fires across
//!    replay attempts, not within a single contiguous sequence).
//! 3. `ReorderEvents` — swap two adjacent events. The swapped seq pair
//!    produces a `sequence violation` `ReplayDivergence`.
//! 4. `EmptyEventSequence` — boundary case: an empty sequence should
//!    succeed with `Ok(())` because `replay_events` is a no-op over zero
//!    events. Asserted as a separate `#[test]`.
//! 5. `FutureEventSeq` — events with seq numbers beyond the contiguous
//!    range. The validator observes a `sequence violation` and returns
//!    `RecoveryError::ReplayDivergence`.
//!
//! ## Boundedness (Power-of-Ten Rule 2)
//!
//! 1000 cases × 4 mutation classes (TruncateEventSequence,
//! DuplicateEventSeq, ReorderEvents, FutureEventSeq) = 4000 maximum
//! iterations. The `ProptestConfig` declares the case count explicitly;
//! the input range for the run identifier and event count is small and
//! statically bounded. The fifth class (EmptyEventSequence) is asserted
//! by a single deterministic `#[test]`, not a proptest.

use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;

use crate::recovery::{ActionReplayTracker, RecoveryError, replay_events};
use crate::{EventSeq, JournalEvent};
use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest};

const RECOVERY_REPLAY_PROPTEST_CASES: u32 = 1000;
const MAX_EVENTS_PER_SEQUENCE: u16 = 6;
const MAX_RUN_VAL: u64 = 10_000;
const TRUNCATE_MIN_EVENTS: u16 = 3;

fn recovery_replay_config() -> ProptestConfig {
    ProptestConfig {
        cases: RECOVERY_REPLAY_PROPTEST_CASES,
        failure_persistence: None,
        ..Default::default()
    }
}

/// The five mutation classes the recovery-layer contract names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryMutation {
    /// Drop events from the middle of the sequence (creates a sequence gap).
    TruncateEventSequence,
    /// Replay a non-idempotent action event twice (idempotency check fires).
    DuplicateEventSeq,
    /// Swap two adjacent events (sequence ordering broken).
    ReorderEvents,
    /// Future seq numbers beyond the contiguous range.
    FutureEventSeq,
}

fn arb_recovery_mutation() -> impl Strategy<Value = RecoveryMutation> {
    (0_u8..4).prop_map(|raw| match raw {
        0 => RecoveryMutation::TruncateEventSequence,
        1 => RecoveryMutation::DuplicateEventSeq,
        2 => RecoveryMutation::ReorderEvents,
        _ => RecoveryMutation::FutureEventSeq,
    })
}

/// Build a valid `Vec<JournalEvent>` (RunAccepted, then alternating
/// StepStarted/StepSucceeded, then optionally a terminal RunFinished).
fn build_valid_event_sequence(run_val: u64, count: u16) -> Vec<JournalEvent> {
    let run = RunId::new(run_val);
    let digest = WorkflowDigest::from_bytes([0xA5_u8; 32]);
    let mut events: Vec<JournalEvent> = Vec::new();
    let total = u64::from(count.max(1));
    for offset in 0..total {
        let seq_value = offset.saturating_add(1);
        let event = if offset == 0 {
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(seq_value),
                workflow: digest,
            }
        } else {
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(seq_value),
                step: StepIdx::new(u16::try_from(offset.saturating_sub(1)).unwrap_or(0)),
                attempt: 1,
            }
        };
        events.push(event);
    }
    events
}

fn truncate_sequence(mut events: Vec<JournalEvent>, _at: usize) -> Vec<JournalEvent> {
    if events.len() < 3 {
        return events;
    }
    let drop_idx = 1_usize.min(events.len() - 1);
    events.remove(drop_idx);
    events
}

fn duplicate_event_seq(events: Vec<JournalEvent>, at: usize) -> Vec<JournalEvent> {
    if events.is_empty() {
        return events;
    }
    let run = match events.first() {
        Some(JournalEvent::RunAccepted { run, .. }) => *run,
        _ => return events,
    };
    let last_seq = events.last().map_or(1, |event| event.seq().get());
    let action = ActionId::new(1);
    let step_idx = u16::try_from(at).unwrap_or(0);
    let mut events = events;
    let completed = JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(last_seq.saturating_add(1)),
        step: StepIdx::new(step_idx),
        action,
        attempt: 1,
    };
    events.push(completed.clone());
    events.push(completed);
    events
}

fn reorder_events(mut events: Vec<JournalEvent>) -> Vec<JournalEvent> {
    if events.len() < 2 {
        return events;
    }
    events.swap(0, 1);
    events
}

fn future_event_seq(mut events: Vec<JournalEvent>, skip: u64) -> Vec<JournalEvent> {
    if let Some(last) = events.last() {
        let current = last.seq().get();
        if let Some(new_seq) = current.checked_add(skip) {
            let bumped = EventSeq::new(new_seq);
            for event in events.iter_mut() {
                *event = bump_seq(event, bumped);
            }
        }
    }
    events
}

fn bump_seq(event: &JournalEvent, new_seq: EventSeq) -> JournalEvent {
    match event {
        JournalEvent::RunAccepted { run, workflow, .. } => JournalEvent::RunAccepted {
            run: *run,
            seq: new_seq,
            workflow: *workflow,
        },
        JournalEvent::StepStarted {
            run, step, attempt, ..
        } => JournalEvent::StepStarted {
            run: *run,
            seq: new_seq,
            step: *step,
            attempt: *attempt,
        },
        other => other.clone(),
    }
}

/// Dispatcher: apply the chosen mutation to a valid event sequence.
fn apply_recovery_mutation(
    events: Vec<JournalEvent>,
    mutation: RecoveryMutation,
    at: usize,
    skip: u64,
) -> Vec<JournalEvent> {
    match mutation {
        RecoveryMutation::TruncateEventSequence => truncate_sequence(events, at),
        RecoveryMutation::DuplicateEventSeq => duplicate_event_seq(events, at),
        RecoveryMutation::ReorderEvents => reorder_events(events),
        RecoveryMutation::FutureEventSeq => future_event_seq(events, skip),
    }
}

/// Assert that the observed `RecoveryError` matches the chosen mutation
/// class. Tighter than the prior proptest: every non-empty mutation
/// surfaces as `RecoveryError::ReplayDivergence` with a
/// "sequence violation" detail. The classifier verifies the detail
/// string contains the `expected`/`found` markers so a regression that
/// removed the validation message would fail the proptest.
fn assert_recovery_class(
    result: &Result<Vec<JournalEvent>, RecoveryError>,
    mutation: RecoveryMutation,
) -> TestCaseResult {
    match result {
        Ok(_) => assert_unexpected_success(mutation, result),
        Err(RecoveryError::ReplayDivergence { detail, .. }) => {
            assert_sequence_violation_detail(mutation, detail)
        }
        Err(other) => assert_unexpected_variant(mutation, other),
    }
}

fn assert_unexpected_success(
    class: RecoveryMutation,
    result: &Result<Vec<JournalEvent>, RecoveryError>,
) -> TestCaseResult {
    prop_assert!(
        false,
        "mutation class {class:?} unexpectedly succeeded: {result:?}"
    );
    Ok(())
}

fn assert_sequence_violation_detail(class: RecoveryMutation, detail: &str) -> TestCaseResult {
    prop_assert!(
        detail.contains("sequence violation"),
        "mutation {class:?} must surface as ReplayDivergence with a \
         sequence violation detail, got detail = {detail:?}"
    );
    prop_assert!(
        detail.contains("expected") && detail.contains("found"),
        "mutation {class:?} ReplayDivergence detail must include \
         expected/found markers, got detail = {detail:?}"
    );
    Ok(())
}

fn assert_unexpected_variant(class: RecoveryMutation, other: &RecoveryError) -> TestCaseResult {
    prop_assert!(
        false,
        "mutation class {class:?} produced unexpected RecoveryError \
         variant: {other:?}"
    );
    Ok(())
}

proptest! {
    #![proptest_config(recovery_replay_config())]

    #[test]
    fn recovery_replay_typed_errors_match_mutation_class(
        run_val in 1_u64..=MAX_RUN_VAL,
        event_count in TRUNCATE_MIN_EVENTS..=MAX_EVENTS_PER_SEQUENCE,
        target_index in 0_u16..=MAX_EVENTS_PER_SEQUENCE,
        skip_delta in 2_u64..=16,
        mutation in arb_recovery_mutation(),
    ) {
        let events = build_valid_event_sequence(run_val, event_count);
        let at = usize::from(target_index) % events.len();
        let mutated = apply_recovery_mutation(events, mutation, at, skip_delta);

        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&mutated, &mut tracker, &[]);

        prop_assert!(assert_recovery_class(&result, mutation).is_ok());
    }
}

/// Boundary case: `replay_events(&[])` must succeed and return an
/// empty replay vector. The recovery contract treats an empty sequence
/// as a no-op, not as a sequence gap.
#[test]
fn recovery_replay_empty_sequence_is_ok() {
    let mut tracker = ActionReplayTracker::new();
    let result = replay_events(&[], &mut tracker, &[]);
    assert!(result.is_ok(), "empty replay must be Ok, got {result:?}");
    assert!(result.unwrap().is_empty());
}
