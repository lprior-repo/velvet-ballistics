#![allow(dead_code)]
#![forbid(unsafe_code)]

//! Flux RS harness for `vb-rpch` r9.
//!
//! Scoped single-file Flux proof artifact mapped to the pure production proof
//! surfaces exposed in `crates/vb_storage/src/recovery/*`. The harness avoids
//! full-crate recovery/I/O/data-structure verification and instead verifies the
//! same quantifier-free boolean, ordering, and bounded-arithmetic semantics as
//! the named production helpers. This is intentionally not a toy replacement for
//! temporal replay behavior; the residual source-correspondence and collection
//! boundaries are ledgered in `.beads/vb-rpch/trusted-base-ledger.flux-r9.jsonl`.
//!
//! Production-source correspondence anchors:
//! - `types.rs::UnsupportedRecoveryState::{SUPPORTED, union,
//!   is_fully_supported, union_matches_flags}`
//! - `types.rs::ActionReplayTracker::{has_completed, has_failed, is_resolved}`
//!   with one action/step membership abstracted to booleans.
//! - `types.rs::DigestCheck::{hierarchy_rank, checks_* ,
//!   is_strictly_weaker_than}`
//! - `summary.rs::{recovery_dimension_count_from_index,
//!   recovery_observed_dimension_is_positive,
//!   recovery_seed_dimensions_positive}`
//! - `hydrate.rs::{hydrate_snapshot_tail_preconditions,
//!   hydrate_events_preconditions, hydrate_dimensions_positive}`
//! - `replay/core.rs::{replay_attempt_is_current, replay_attempt_is_stale,
//!   replay_event_is_stale_state_effect, replay_step_order_diverges}`

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

#[spec(
    fn(
        left: UnsupportedRecoveryState[@lsv, @lst, @lap, @lpa],
        right: UnsupportedRecoveryState[@rsv, @rst, @rap, @rpa],
        union: UnsupportedRecoveryState[@usv, @ust, @uap, @upa],
    ) -> bool[usv == (lsv || rsv) && ust == (lst || rst) && uap == (lap || rap) && upa == (lpa || rpa)]
)]
fn union_matches_flags_surface(
    left: UnsupportedRecoveryState,
    right: UnsupportedRecoveryState,
    union: UnsupportedRecoveryState,
) -> bool {
    union.slot_values == (left.slot_values || right.slot_values)
        && union.slot_taint == (left.slot_taint || right.slot_taint)
        && union.action_payloads == (left.action_payloads || right.action_payloads)
        && union.pending_actions == (left.pending_actions || right.pending_actions)
}

#[spec(fn() -> bool[true])]
fn proof_supported_constant_has_no_unsupported_field() -> bool {
    is_fully_supported(supported_recovery_state())
}

#[spec(fn() -> bool[true])]
fn proof_union_function_matches_flag_or_semantics() -> bool {
    let left = UnsupportedRecoveryState {
        slot_values: true,
        slot_taint: false,
        action_payloads: true,
        pending_actions: false,
    };
    let right = UnsupportedRecoveryState {
        slot_values: false,
        slot_taint: true,
        action_payloads: false,
        pending_actions: false,
    };
    let union = union_recovery_state(left, right);
    union_matches_flags_surface(
        UnsupportedRecoveryState {
            slot_values: true,
            slot_taint: false,
            action_payloads: true,
            pending_actions: false,
        },
        UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: true,
            action_payloads: false,
            pending_actions: false,
        },
        union,
    )
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

#[should_fail]
#[spec(fn() -> bool[true])]
fn negative_union_mismatch_is_rejected() -> bool {
    union_matches_flags_surface(
        supported_recovery_state(),
        UnsupportedRecoveryState {
            slot_values: true,
            slot_taint: false,
            action_payloads: false,
            pending_actions: false,
        },
        supported_recovery_state(),
    )
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

#[spec(fn(tracker: ActionReplayTrackerSurface[@completed, @failed]) -> bool[completed || failed])]
fn is_resolved_surface(tracker: ActionReplayTrackerSurface) -> bool {
    tracker.completed || tracker.failed
}

#[spec(fn() -> bool[true])]
fn proof_tracker_resolution_after_completion_is_monotone() -> bool {
    let tracker = mark_completed_surface(new_tracker_surface());
    is_resolved_surface(tracker)
}

#[spec(fn() -> bool[true])]
fn proof_tracker_resolution_after_failure_is_monotone() -> bool {
    let tracker = mark_failed_surface(new_tracker_surface());
    is_resolved_surface(tracker)
}

#[spec(fn(tracker: ActionReplayTrackerSurface[@completed, @failed]) -> bool[completed || failed])]
fn proof_tracker_resolution_exactly_matches_public_membership(
    tracker: ActionReplayTrackerSurface,
) -> bool {
    is_resolved_surface(tracker)
}

#[should_fail]
#[spec(fn() -> bool[true])]
fn negative_new_tracker_is_not_resolved() -> bool {
    is_resolved_surface(new_tracker_surface())
}

// VFR-R2-FLUX-002 / INV-003.
#[refined_by(present: bool)]
enum DimensionObservation {
    #[variant((u16{v: v < 65535}) -> DimensionObservation[true])]
    Present(u16),
    #[variant(DimensionObservation[false])]
    Absent,
}

#[spec(fn(max_index: u16{max_index < 65535}) -> u16{count: count > 0})]
fn recovery_dimension_count_from_present_index(max_index: u16) -> u16 {
    max_index + 1
}

#[spec(fn(observed: DimensionObservation[@present]) -> u16{count: if present { count > 0 } else { count == 0 }})]
fn recovery_dimension_count_from_observation(observed: DimensionObservation) -> u16 {
    match observed {
        DimensionObservation::Present(max_index) => max_index + 1,
        DimensionObservation::Absent => 0,
    }
}

#[spec(fn(observed: DimensionObservation[@present], count: u16) -> bool[if present { count > 0 } else { count == 0 }])]
fn recovery_observed_dimension_is_positive_surface(
    observed: DimensionObservation,
    count: u16,
) -> bool {
    match observed {
        DimensionObservation::Present(_) => count > 0,
        DimensionObservation::Absent => count == 0,
    }
}

#[spec(fn(step_count: u16, slot_count: u16) -> bool[step_count > 0 && slot_count > 0])]
fn recovery_seed_dimensions_positive_surface(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}

#[spec(fn(max_index: u16{max_index < 65535}) -> bool[true])]
fn proof_observed_dimension_count_is_positive(max_index: u16) -> bool {
    recovery_dimension_count_from_present_index(max_index) > 0
}

#[spec(fn(max_index: u16{max_index < 65535}) -> bool[true])]
fn proof_present_observation_count_matches_presence(max_index: u16) -> bool {
    let count = recovery_dimension_count_from_present_index(max_index);
    recovery_observed_dimension_is_positive_surface(DimensionObservation::Present(max_index), count)
}

#[spec(fn() -> bool[true])]
fn proof_absent_observation_count_matches_presence() -> bool {
    recovery_observed_dimension_is_positive_surface(DimensionObservation::Absent, 0)
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

#[should_fail]
#[spec(fn() -> bool[true])]
fn negative_absent_observation_cannot_have_positive_count() -> bool {
    recovery_observed_dimension_is_positive_surface(DimensionObservation::Absent, 1)
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

#[spec(fn(step_count: u16, slot_count: u16) -> bool[step_count > 0 && slot_count > 0])]
fn hydrate_dimensions_positive_surface(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}

#[spec(fn(events_len: usize) -> bool[events_len > 0])]
fn hydrate_events_preconditions_surface(events_len: usize) -> bool {
    events_len > 0
}

#[spec(fn(events_len: usize{events_len > 0}) -> bool[true])]
fn proof_hydrate_events_nonempty_precondition(events_len: usize) -> bool {
    hydrate_events_preconditions_surface(events_len)
}

#[spec(fn(step_count: u16{step_count > 0}, slot_count: u16{slot_count > 0}) -> bool[true])]
fn proof_hydrate_dimensions_positive(step_count: u16, slot_count: u16) -> bool {
    hydrate_dimensions_positive_surface(step_count, slot_count)
}

#[should_fail]
#[spec(fn() -> bool[true])]
fn negative_hydrate_snapshot_tail_missing_evidence_rejected() -> bool {
    hydrate_snapshot_tail_preconditions_surface(HydrateSnapshotTailSurface {
        run_matches: true,
        seq_after_snapshot: true,
        has_evidence: false,
    })
}

#[should_fail]
#[spec(fn() -> bool[true])]
fn negative_empty_events_precondition_rejected() -> bool {
    hydrate_events_preconditions_surface(0)
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

#[should_fail]
#[spec(fn() -> bool[true])]
fn negative_state_effect_without_stale_is_not_stale_state_effect() -> bool {
    replay_event_is_stale_state_effect_surface(ReplayEventSurface {
        is_state_effect: true,
        is_stale: false,
    })
}

#[should_fail]
#[spec(fn() -> bool[true])]
fn negative_increasing_step_order_does_not_diverge() -> bool {
    replay_step_order_diverges_surface(1, 2)
}
