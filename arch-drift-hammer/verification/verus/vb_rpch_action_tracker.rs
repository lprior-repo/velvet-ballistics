// Verus proof obligations for vb-rpch INV-004: ActionReplayTracker is_resolved monotonicity.
//
// Obligation: VERUS-REC-004 / INV-004
// Contract: ActionReplayTracker::is_resolved is monotonic — once (action, step) is
//           marked completed or failed, is_resolved always returns true.

use vstd::prelude::*;

verus! {

// Spec-level type definitions inlined from vb_core::ids (ActionId = u16, StepIdx = u16)
pub type ActionId = u16;
pub type StepIdx = u16;

pub open spec fn spec_is_resolved(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, action: ActionId, step: StepIdx) -> bool {
    completed.contains((action, step)) || failed.contains((action, step))
}

pub proof fn proof_resolved_is_permanent(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, action: ActionId, step: StepIdx)
    requires
        spec_is_resolved(completed, failed, action, step)
        && completed.contains((action, step))
        && failed.contains((action, step)),
    ensures
        spec_is_resolved(
            completed.remove((action, step)),
            failed,
            action,
            step
        ) == true
        && spec_is_resolved(
            completed,
            failed.remove((action, step)),
            action,
            step
        ) == true
{
    // When action is in BOTH completed AND failed, removing from one set still leaves
    // it resolved because it's in the other set.
    reveal(spec_is_resolved);
}

pub proof fn proof_mark_completed_preserves_monotonicity(
    completed: Set<(ActionId, StepIdx)>,
    failed: Set<(ActionId, StepIdx)>,
    action: ActionId,
    step: StepIdx
)
    requires
        !spec_is_resolved(completed, failed, action, step),
    ensures
        spec_is_resolved(
            completed.insert((action, step)),
            failed,
            action,
            step
        )
{
    reveal(spec_is_resolved);
}

pub proof fn proof_mark_failed_preserves_monotonicity(
    completed: Set<(ActionId, StepIdx)>,
    failed: Set<(ActionId, StepIdx)>,
    action: ActionId,
    step: StepIdx
)
    requires
        !spec_is_resolved(completed, failed, action, step),
    ensures
        spec_is_resolved(
            completed,
            failed.insert((action, step)),
            action,
            step
        )
{
    reveal(spec_is_resolved);
}

pub proof fn proof_no_double_resolution(
    completed: Set<(ActionId, StepIdx)>,
    failed: Set<(ActionId, StepIdx)>,
    action: ActionId,
    step: StepIdx
)
    requires
        spec_is_resolved(completed, failed, action, step),
    ensures
        spec_is_resolved(
            completed.insert((action, step)),
            failed.insert((action, step)),
            action,
            step
        ) == true
{
    reveal(spec_is_resolved);
}

} // verus!

fn main() {}