#![allow(dead_code)]
#![forbid(unsafe_code)]

//! Flux RS harness for `vb-rpch` r8.
//!
//! This file is a scoped, single-file Flux harness. It mirrors the pure proof
//! surfaces exposed by the Holzman State 11 production implementation without
//! importing the full `vb_storage` crate. The full crate currently passes a
//! Flux tooling smoke check, but these refinements are the claim evidence for
//! `VFR-R2-FLUX-001..007` at the lightweight pure-surface scope.
//!
//! Production-source correspondence anchors:
//! - `crates/vb_storage/src/recovery/types.rs::UnsupportedRecoveryState`
//! - `crates/vb_storage/src/recovery/types.rs::ActionReplayTracker` public
//!   resolution predicates, with `HashSet` membership abstracted to booleans.
//! - `crates/vb_storage/src/recovery/types.rs::DigestCheck`
//! - `crates/vb_storage/src/recovery/replay/summary.rs` dimension helpers.
//! - `crates/vb_storage/src/recovery/hydrate.rs` and
//!   `crates/vb_storage/src/recovery/replay/core.rs` pure precondition helpers.

extern crate flux_rs;

use flux_rs::attrs::*;

// VFR-R2-FLUX-001 / INV-002.
#[refined_by(slot_values: bool, slot_taint: bool, action_payloads: bool, pending_actions: bool)]
struct UnsupportedRecoveryState {
    #[field(bool[slot_values])]
    slot_values: bool,
    #[field(bool[slot_taint])]
    slot_taint: bool,
    #[field(bool[action_payloads])]
    action_payloads: bool,
    #[field(bool[pending_actions])]
    pending_actions: bool,
}

#[spec(fn() -> UnsupportedRecoveryState[false, false, false, false])]
fn supported_recovery_state() -> UnsupportedRecoveryState {
    UnsupportedRecoveryState {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    }
}

#[spec(
    fn(
        left: UnsupportedRecoveryState[@lsv, @lst, @lap, @lpa],
        right: UnsupportedRecoveryState[@rsv, @rst, @rap, @rpa],
    ) -> UnsupportedRecoveryState[lsv || rsv, lst || rst, lap || rap, lpa || rpa]
)]
fn union_recovery_state(
    left: UnsupportedRecoveryState,
    right: UnsupportedRecoveryState,
) -> UnsupportedRecoveryState {
    UnsupportedRecoveryState {
        slot_values: left.slot_values || right.slot_values,
        slot_taint: left.slot_taint || right.slot_taint,
        action_payloads: left.action_payloads || right.action_payloads,
        pending_actions: left.pending_actions || right.pending_actions,
    }
}

#[spec(fn(state: UnsupportedRecoveryState[@sv, @st, @ap, @pa]) -> bool[!sv && !st && !ap && !pa])]
fn is_fully_supported(state: UnsupportedRecoveryState) -> bool {
    !state.slot_values && !state.slot_taint && !state.action_payloads && !state.pending_actions
}

#[spec(fn() -> bool[true])]
fn proof_supported_constant_has_no_false_field() -> bool {
    is_fully_supported(supported_recovery_state())
}

#[spec(fn() -> bool[true])]
fn proof_union_of_supported_states_stays_supported() -> bool {
    let merged = union_recovery_state(supported_recovery_state(), supported_recovery_state());
    is_fully_supported(merged)
}

#[should_fail]
#[spec(fn() -> UnsupportedRecoveryState[false, false, false, false])]
fn negative_supported_rejects_true_slot_values() -> UnsupportedRecoveryState {
    UnsupportedRecoveryState {
        slot_values: true,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    }
}

// VFR-R2-FLUX-004 / INV-005.
#[refined_by(rank: int)]
enum DigestCheck {
    #[variant(DigestCheck[1])]
    WorkflowSourceOnly,
    #[variant(DigestCheck[2])]
    WorkflowAndIr,
    #[variant(DigestCheck[3])]
    Full,
}

#[spec(fn(level: DigestCheck[@rank]) -> u8[rank])]
fn digest_hierarchy_rank(level: DigestCheck) -> u8 {
    match level {
        DigestCheck::WorkflowSourceOnly => 1,
        DigestCheck::WorkflowAndIr => 2,
        DigestCheck::Full => 3,
    }
}

#[spec(fn(level: DigestCheck[@rank]) -> bool[rank >= 1])]
fn checks_workflow_source(level: DigestCheck) -> bool {
    digest_hierarchy_rank(level) >= digest_hierarchy_rank(DigestCheck::WorkflowSourceOnly)
}

#[spec(fn(level: DigestCheck[@rank]) -> bool[rank >= 2])]
fn checks_compiled_ir(level: DigestCheck) -> bool {
    digest_hierarchy_rank(level) >= digest_hierarchy_rank(DigestCheck::WorkflowAndIr)
}

#[spec(fn(level: DigestCheck[@rank]) -> bool[rank >= 3])]
fn checks_full(level: DigestCheck) -> bool {
    digest_hierarchy_rank(level) >= digest_hierarchy_rank(DigestCheck::Full)
}

#[spec(fn(left: DigestCheck[@left_rank], right: DigestCheck[@right_rank]) -> bool[left_rank < right_rank])]
fn is_strictly_weaker_than(left: DigestCheck, right: DigestCheck) -> bool {
    digest_hierarchy_rank(left) < digest_hierarchy_rank(right)
}

#[spec(fn() -> bool[true])]
fn proof_digest_hierarchy_is_strict() -> bool {
    is_strictly_weaker_than(DigestCheck::WorkflowSourceOnly, DigestCheck::WorkflowAndIr)
        && is_strictly_weaker_than(DigestCheck::WorkflowAndIr, DigestCheck::Full)
        && is_strictly_weaker_than(DigestCheck::WorkflowSourceOnly, DigestCheck::Full)
        && checks_workflow_source(DigestCheck::WorkflowSourceOnly)
        && checks_workflow_source(DigestCheck::WorkflowAndIr)
        && checks_workflow_source(DigestCheck::Full)
        && !checks_compiled_ir(DigestCheck::WorkflowSourceOnly)
        && checks_compiled_ir(DigestCheck::WorkflowAndIr)
        && checks_compiled_ir(DigestCheck::Full)
        && !checks_full(DigestCheck::WorkflowAndIr)
        && checks_full(DigestCheck::Full)
}

#[should_fail]
#[spec(fn() -> bool[true])]
fn negative_digest_source_only_is_not_full() -> bool {
    checks_full(DigestCheck::WorkflowSourceOnly)
}

// VFR-R2-FLUX-003 / INV-004.
// `HashSet<(ActionId, StepIdx)>` membership is intentionally abstracted to the
// pure support predicate booleans exposed by `has_completed`, `has_failed`, and
// `is_resolved` in production. This proves the logical resolution surface, not
// the `HashSet` implementation.
#[refined_by(completed: bool, failed: bool)]
struct ActionReplayTrackerSurface {
    #[field(bool[completed])]
    completed: bool,
    #[field(bool[failed])]
    failed: bool,
}

#[spec(fn() -> ActionReplayTrackerSurface[false, false])]
fn new_tracker_surface() -> ActionReplayTrackerSurface {
    ActionReplayTrackerSurface {
        completed: false,
        failed: false,
    }
}

#[spec(fn(tracker: ActionReplayTrackerSurface[@_completed, @failed]) -> ActionReplayTrackerSurface[true, failed])]
fn mark_completed_surface(tracker: ActionReplayTrackerSurface) -> ActionReplayTrackerSurface {
    ActionReplayTrackerSurface {
        completed: true,
        failed: tracker.failed,
    }
}

#[spec(fn(tracker: ActionReplayTrackerSurface[@completed, @_failed]) -> ActionReplayTrackerSurface[completed, true])]
fn mark_failed_surface(tracker: ActionReplayTrackerSurface) -> ActionReplayTrackerSurface {
    ActionReplayTrackerSurface {
        completed: tracker.completed,
        failed: true,
    }
}

#[spec(fn(tracker: ActionReplayTrackerSurface[@completed, @failed]) -> bool[completed])]
fn has_completed_surface(tracker: ActionReplayTrackerSurface) -> bool {
    tracker.completed
}

#[spec(fn(tracker: ActionReplayTrackerSurface[@completed, @failed]) -> bool[failed])]
fn has_failed_surface(tracker: ActionReplayTrackerSurface) -> bool {
    tracker.failed
}

#[spec(fn(tracker: ActionReplayTrackerSurface[@completed, @failed]) -> bool[completed || failed])]
fn is_resolved_surface(tracker: ActionReplayTrackerSurface) -> bool {
    tracker.completed || tracker.failed
}

#[spec(fn() -> bool[true])]
fn proof_tracker_resolution_after_completion_is_monotone() -> bool {
    let tracker = mark_completed_surface(new_tracker_surface());
    has_completed_surface(tracker) || is_resolved_surface(mark_completed_surface(new_tracker_surface()))
}

#[spec(fn() -> bool[true])]
fn proof_tracker_resolution_after_failure_is_monotone() -> bool {
    let tracker = mark_failed_surface(new_tracker_surface());
    has_failed_surface(tracker) || is_resolved_surface(mark_failed_surface(new_tracker_surface()))
}

// VFR-R2-FLUX-002 / INV-003.
#[spec(fn(max_index: u16{max_index < 65535}) -> u16{count: count > 0})]
fn recovery_dimension_count_from_present_index(max_index: u16) -> u16 {
    max_index + 1
}

#[spec(fn(step_count: u16, slot_count: u16) -> bool[step_count > 0 && slot_count > 0])]
fn recovery_seed_dimensions_positive_surface(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}

#[spec(fn(max_index: u16{max_index < 65535}) -> bool[true])]
fn proof_observed_dimension_count_is_positive(max_index: u16) -> bool {
    recovery_dimension_count_from_present_index(max_index) > 0
}

#[spec(fn(step_count: u16{step_count > 0}, slot_count: u16{slot_count > 0}) -> bool[true])]
fn proof_seed_dimensions_positive_when_checked(step_count: u16, slot_count: u16) -> bool {
    recovery_seed_dimensions_positive_surface(step_count, slot_count)
}

#[should_fail]
#[spec(fn() -> bool[true])]
fn negative_zero_seed_dimension_is_rejected() -> bool {
    recovery_seed_dimensions_positive_surface(0, 1)
}

// VFR-R2-FLUX-005 / PRE-001 and VFR-R2-FLUX-006 / PRE-002.
#[refined_by(run_matches: bool, seq_after_snapshot: bool, has_evidence: bool)]
struct HydrateSnapshotTailSurface {
    #[field(bool[run_matches])]
    run_matches: bool,
    #[field(bool[seq_after_snapshot])]
    seq_after_snapshot: bool,
    #[field(bool[has_evidence])]
    has_evidence: bool,
}

#[spec(fn(surface: HydrateSnapshotTailSurface[@run_matches, @seq_after_snapshot, @has_evidence]) -> bool[run_matches && seq_after_snapshot && has_evidence])]
fn hydrate_snapshot_tail_preconditions_surface(surface: HydrateSnapshotTailSurface) -> bool {
    surface.run_matches && surface.seq_after_snapshot && surface.has_evidence
}

#[spec(fn(surface: HydrateSnapshotTailSurface[true, true, true]) -> bool[true])]
fn proof_hydrate_snapshot_tail_preconditions(surface: HydrateSnapshotTailSurface) -> bool {
    hydrate_snapshot_tail_preconditions_surface(surface)
}

#[spec(fn(events_len: usize) -> bool[events_len > 0])]
fn hydrate_events_preconditions_surface(events_len: usize) -> bool {
    events_len > 0
}

#[spec(fn(events_len: usize{events_len > 0}) -> bool[true])]
fn proof_hydrate_events_nonempty_precondition(events_len: usize) -> bool {
    hydrate_events_preconditions_surface(events_len)
}

// VFR-R2-FLUX-007 / POST-009 pure replay precondition surface.
#[refined_by(is_state_effect: bool, is_stale: bool)]
struct ReplayEventSurface {
    #[field(bool[is_state_effect])]
    is_state_effect: bool,
    #[field(bool[is_stale])]
    is_stale: bool,
}

#[spec(fn(attempt: u16, max_attempt: u16) -> bool[attempt >= max_attempt])]
fn replay_attempt_is_current_surface(attempt: u16, max_attempt: u16) -> bool {
    attempt >= max_attempt
}

#[spec(fn(attempt: u16, max_attempt: u16) -> bool[attempt < max_attempt])]
fn replay_attempt_is_stale_surface(attempt: u16, max_attempt: u16) -> bool {
    attempt < max_attempt
}

#[spec(fn(event: ReplayEventSurface[@is_state_effect, @is_stale]) -> bool[is_state_effect && is_stale])]
fn replay_event_is_stale_state_effect_surface(event: ReplayEventSurface) -> bool {
    event.is_state_effect && event.is_stale
}

#[spec(fn(previous: u16, current: u16) -> bool[current < previous])]
fn replay_step_order_diverges_surface(previous: u16, current: u16) -> bool {
    current < previous
}

#[spec(fn(attempt: u16, max_attempt: u16{attempt < max_attempt}) -> bool[true])]
fn proof_stale_attempt_is_not_current(attempt: u16, max_attempt: u16) -> bool {
    replay_attempt_is_stale_surface(attempt, max_attempt)
        && !replay_attempt_is_current_surface(attempt, max_attempt)
}

#[spec(fn(event: ReplayEventSurface[true, true]) -> bool[true])]
fn proof_stale_state_effect_surface(event: ReplayEventSurface) -> bool {
    replay_event_is_stale_state_effect_surface(event)
}

#[spec(fn(previous: u16, current: u16{current < previous}) -> bool[true])]
fn proof_step_order_divergence_surface(previous: u16, current: u16) -> bool {
    replay_step_order_diverges_surface(previous, current)
}
