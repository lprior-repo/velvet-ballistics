#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

/// VFR-R2-VERUS-007 / POST-009.
/// Bridge model for State-11 production proof surfaces in replay/core.rs:
/// replay_attempt_or_default, replay_attempt_is_current,
/// replay_attempt_is_stale, replay_event_has_state_effect,
/// replay_event_is_stale_state_effect, and replay_step_order_diverges.
pub type ActionId = int;
pub type StepIdx = int;

pub struct ReplayEvent {
    pub has_attempt: bool,
    pub attempt: int,
    pub state_effect: bool,
    pub action: ActionId,
    pub step: StepIdx,
    pub completed: bool,
    pub failed: bool,
    pub non_idempotent: bool,
    pub step_order_ok: bool,
}

pub struct ReplayState {
    pub completed: Set<(ActionId, StepIdx)>,
    pub failed: Set<(ActionId, StepIdx)>,
    pub has_last_step: bool,
    pub last_step: StepIdx,
    pub diverged: bool,
    pub blocked: bool,
}

pub open spec fn production_replay_attempt_or_default(has_attempt: bool, attempt: int) -> int {
    if has_attempt { attempt } else { 1 }
}

pub open spec fn production_replay_attempt_is_current(has_attempt: bool, attempt: int, max_attempt: int) -> bool {
    production_replay_attempt_or_default(has_attempt, attempt) >= max_attempt
}

pub open spec fn production_replay_attempt_is_stale(has_attempt: bool, attempt: int, max_attempt: int) -> bool {
    production_replay_attempt_or_default(has_attempt, attempt) < max_attempt
}

pub open spec fn production_replay_event_has_state_effect(event: ReplayEvent) -> bool {
    event.state_effect
}

pub open spec fn production_replay_event_is_stale_state_effect(event: ReplayEvent, max_attempt: int) -> bool {
    production_replay_event_has_state_effect(event) && production_replay_attempt_is_stale(event.has_attempt, event.attempt, max_attempt)
}

pub open spec fn production_replay_step_order_diverges(has_previous: bool, previous: StepIdx, current: StepIdx) -> bool {
    has_previous && current < previous
}

pub open spec fn replay_step(state: ReplayState, event: ReplayEvent, max_attempt: int) -> ReplayState {
    if production_replay_attempt_is_stale(event.has_attempt, event.attempt, max_attempt) {
        state
    } else if !event.step_order_ok {
        ReplayState { diverged: true, ..state }
    } else if event.completed {
        ReplayState { completed: state.completed.insert((event.action, event.step)), ..state }
    } else if event.failed {
        ReplayState { failed: state.failed.insert((event.action, event.step)), ..state }
    } else {
        state
    }
}

pub open spec fn replay_from(events: Seq<ReplayEvent>, max_attempt: int, state: ReplayState, index: int) -> ReplayState
    decreases events.len() - index when 0 <= index <= events.len()
{
    if index < events.len() {
        replay_from(events, max_attempt, replay_step(state, events[index], max_attempt), index + 1)
    } else {
        state
    }
}

pub open spec fn tracker_resolved(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, key: (ActionId, StepIdx)) -> bool {
    completed.contains(key) || failed.contains(key)
}

pub open spec fn non_idempotent_blocked(already_resolved: bool, non_idempotent: bool) -> bool {
    already_resolved && non_idempotent
}

pub open spec fn step_order_diverges(events: Seq<ReplayEvent>) -> bool {
    exists|i: int| 0 <= i < events.len() && !events[i].step_order_ok
}

pub proof fn proof_stale_replay_step_is_noop(state: ReplayState, event: ReplayEvent, max_attempt: int)
    requires production_replay_attempt_is_stale(event.has_attempt, event.attempt, max_attempt),
    ensures replay_step(state, event, max_attempt) == state,
{}

pub proof fn proof_all_stale_replay_prefix_is_noop(events: Seq<ReplayEvent>, max_attempt: int, state: ReplayState, index: int)
    requires
        0 <= index <= events.len(),
        forall|i: int| index <= i < events.len() ==> production_replay_attempt_is_stale(events[i].has_attempt, events[i].attempt, max_attempt),
    ensures replay_from(events, max_attempt, state, index) == state,
    decreases events.len() - index,
{
    if index < events.len() {
        proof_stale_replay_step_is_noop(state, events[index], max_attempt);
        proof_all_stale_replay_prefix_is_noop(events, max_attempt, state, index + 1);
    }
}

pub proof fn proof_stale_attempt_filter_preserved(events: Seq<ReplayEvent>, max_attempt: int, state: ReplayState)
    requires forall|i: int| 0 <= i < events.len() ==> production_replay_attempt_is_stale(events[i].has_attempt, events[i].attempt, max_attempt),
    ensures replay_from(events, max_attempt, state, 0) == state,
{
    proof_all_stale_replay_prefix_is_noop(events, max_attempt, state, 0);
}

pub proof fn proof_stale_state_effect_predicate_matches_surfaces(event: ReplayEvent, max_attempt: int)
    ensures production_replay_event_is_stale_state_effect(event, max_attempt) == (event.state_effect && production_replay_attempt_is_stale(event.has_attempt, event.attempt, max_attempt)),
{}

pub proof fn proof_completed_or_failed_marks_resolved(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, key: (ActionId, StepIdx))
    ensures
        tracker_resolved(completed.insert(key), failed, key),
        tracker_resolved(completed, failed.insert(key), key),
{}

pub proof fn proof_resolved_non_idempotent_is_blocked()
    ensures non_idempotent_blocked(true, true), !non_idempotent_blocked(false, true), !non_idempotent_blocked(true, false),
{}

pub proof fn proof_step_order_divergence_detected(events: Seq<ReplayEvent>, bad_index: int)
    requires 0 <= bad_index < events.len(), !events[bad_index].step_order_ok,
    ensures step_order_diverges(events),
{}

pub proof fn proof_production_step_order_divergence_detected(previous: StepIdx, current: StepIdx)
    requires current < previous,
    ensures production_replay_step_order_diverges(true, previous, current),
{}

}
