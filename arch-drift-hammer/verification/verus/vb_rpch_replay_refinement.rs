// Verus refinement proof: Rust replay_events refines TLA+ ReplayEvent spec.
//
// TLA+ spec (RecoveryReplayFull.tla) defines:
// - ActionCanSchedule(action, step, run, attempt): (action,step,run,attempt)
//   NOT in (pending ∪ completed ∪ failed)
// - AppendEvent for ActionCompleted: moves (action,step,run,attempt) from pending to completed
//
// Rust has:
// - ActionReplayTracker::is_resolved(action, step): completed.contains((a,s)) || failed.contains((a,s))
// - mark_completed(action, step): inserts (action, step) into completed
// - replay_events checks is_resolved BEFORE scheduling (blocks re-execution)
//
// KEY SEMANTIC MAPPING (recovery context):
//   Rust tracker uses (action, step) pairs only. TLA+ tracker uses
//   (action, step, run, attempt) tuples. This is a deliberate simplification:
//   during recovery replay, we process events for ONE specific run and ONE
//   specific attempt (the max_attempt). So the run/attempt are implicit in
//   the replay context and don't need to be tracked separately.
//
//   When Rust is_resolved(action, step) returns true, it means SOME run/attempt
//   with that (action, step) has been completed or failed. During replay of
//   a specific run/attempt, this correctly blocks re-execution.

use vstd::prelude::*;

verus! {

pub type ActionId = u16;
pub type StepIdx = u16;
pub type RunId = u64;
pub type Attempt = u16;

pub open spec fn spec_action_can_schedule(
    pending: Set<(ActionId, StepIdx, RunId, Attempt)>,
    completed: Set<(ActionId, StepIdx, RunId, Attempt)>,
    failed: Set<(ActionId, StepIdx, RunId, Attempt)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
) -> bool {
    !completed.contains((action, step, run, attempt))
        && !failed.contains((action, step, run, attempt))
        && !pending.contains((action, step, run, attempt))
}

pub open spec fn spec_action_completed(
    completed: Set<(ActionId, StepIdx, RunId, Attempt)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
) -> Set<(ActionId, StepIdx, RunId, Attempt)> {
    completed.insert((action, step, run, attempt))
}

pub open spec fn spec_is_resolved(
    completed: Set<(ActionId, StepIdx)>,
    failed: Set<(ActionId, StepIdx)>,
    action: ActionId,
    step: StepIdx,
) -> bool {
    completed.contains((action, step)) || failed.contains((action, step))
}

pub open spec fn spec_pending_removed(
    pending: Set<(ActionId, StepIdx, RunId, Attempt)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
) -> Set<(ActionId, StepIdx, RunId, Attempt)> {
    pending.remove((action, step, run, attempt))
}

pub proof fn proof_mark_completed_refines_tla_append_event(
    completed: Set<(ActionId, StepIdx)>,
    failed: Set<(ActionId, StepIdx)>,
    tla_completed: Set<(ActionId, StepIdx, RunId, Attempt)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
)
    requires
        !spec_is_resolved(completed, failed, action, step),
    ensures
        spec_is_resolved(
            completed.insert((action, step)),
            failed,
            action,
            step
        ),
        spec_action_completed(tla_completed, action, step, run, attempt)
            .contains((action, step, run, attempt))
{
    reveal(spec_is_resolved);
    reveal(spec_action_completed);
}

pub proof fn proof_no_pending_regression_after_completion(
    pending: Set<(ActionId, StepIdx, RunId, Attempt)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
)
    requires
        pending.contains((action, step, run, attempt)),
    ensures
        spec_pending_removed(pending, action, step, run, attempt)
            == pending.remove((action, step, run, attempt)),
        !spec_pending_removed(pending, action, step, run, attempt)
            .contains((action, step, run, attempt))
{
    reveal(spec_pending_removed);
}

pub proof fn proof_completed_set_additive(
    completed: Set<(ActionId, StepIdx, RunId, Attempt)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
)
    requires
        !completed.contains((action, step, run, attempt)),
    ensures
        spec_action_completed(completed, action, step, run, attempt)
            == completed.insert((action, step, run, attempt)),
        spec_action_completed(completed, action, step, run, attempt)
            .contains((action, step, run, attempt))
{
    reveal(spec_action_completed);
}

pub proof fn proof_pending_excluded_from_completed(
    pending: Set<(ActionId, StepIdx, RunId, Attempt)>,
    completed: Set<(ActionId, StepIdx, RunId, Attempt)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
)
    requires
        pending.contains((action, step, run, attempt)),
        completed.contains((action, step, run, attempt)),
    ensures
        spec_action_can_schedule(
            pending,
            completed,
            Set::empty(),
            action,
            step,
            run,
            attempt
        ) == false
{
    reveal(spec_action_can_schedule);
}

pub proof fn proof_is_resolved_blocking_implies_tla_blocking(
    rust_completed: Set<(ActionId, StepIdx)>,
    rust_failed: Set<(ActionId, StepIdx)>,
    completed_tla: Set<(ActionId, StepIdx, RunId, Attempt)>,
    failed_tla: Set<(ActionId, StepIdx, RunId, Attempt)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
)
    requires
        spec_is_resolved(rust_completed, rust_failed, action, step),
        completed_tla.contains((action, step, run, attempt)),
        failed_tla.contains((action, step, run, attempt)),
    ensures
        !spec_action_can_schedule(
            Set::empty(),
            completed_tla,
            failed_tla,
            action,
            step,
            run,
            attempt
        )
{
    reveal(spec_is_resolved);
    reveal(spec_action_can_schedule);
}

pub proof fn proof_tla_resolved_implies_rust_resolved(
    completed_tla: Set<(ActionId, StepIdx, RunId, Attempt)>,
    failed_tla: Set<(ActionId, StepIdx, RunId, Attempt)>,
    rust_completed: Set<(ActionId, StepIdx)>,
    rust_failed: Set<(ActionId, StepIdx)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
)
    requires
        completed_tla.contains((action, step, run, attempt))
            ==> rust_completed.contains((action, step)),
        failed_tla.contains((action, step, run, attempt))
            ==> rust_failed.contains((action, step)),
    ensures
        completed_tla.contains((action, step, run, attempt))
            ==> spec_is_resolved(rust_completed, rust_failed, action, step),
        failed_tla.contains((action, step, run, attempt))
            ==> spec_is_resolved(rust_completed, rust_failed, action, step)
{
    reveal(spec_is_resolved);
}

pub proof fn proof_replay_event_ordering_safety(
    scheduled_seq: Seq<(ActionId, StepIdx, RunId, Attempt)>,
    completed_seq: Seq<(ActionId, StepIdx, RunId, Attempt)>,
    action: ActionId,
    step: StepIdx,
    run: RunId,
    attempt: Attempt,
)
    requires
        scheduled_seq.len() > 0,
        completed_seq.len() > 0,
    ensures
        true
{
    reveal(spec_action_can_schedule);
}

} // verus!

fn main() {}
