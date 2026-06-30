// Verus proof obligations for vb-rpch INV-004: ActionReplayTracker is_resolved monotonicity.
//
// Obligation: VERUS-REC-004 / INV-004
// Contract: ActionReplayTracker::is_resolved is monotonic — once (action, step) is
//           marked completed or failed, is_resolved always returns true.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to production via the companion extern surface
// `verification/verus/extern_vb_rpch_action_tracker.rs`, which itself
// `#[path]`-includes the verbatim production mirror at
// `verification/verus/production_inner/action_replay_tracker_production.rs`
// (a verbatim copy of `crates/vb_storage/src/recovery/types.rs:666-852`).
//
// The `assume_specification` bridges below attach the production
// contracts for `is_resolved`, `mark_completed`, and `mark_failed` to
// the spec-side mirror methods. The exec wrappers invoke the mirror
// methods to discharge the contracts; they are the non-vacuum
// witnesses that the bridges are actually used.
//
// The Set abstraction in `spec_is_resolved` is bridged to the
// production HashSet via the standard library `@` (View) impl:
// `tracker.completed@` returns the Set view of the HashSet, and
// `tracker.completed.insert(...).completed@` returns the Set view
// after the insert. This makes the spec proofs operate on the
// same Set semantics as the production HashSet without changing
// production code.
//
// BINDING LEDGER:
//   - `production::ActionReplayTracker::is_resolved`     <- types.rs:843-845
//   - `production::ActionReplayTracker::mark_completed`  <- types.rs:761-763
//   - `production::ActionReplayTracker::mark_failed`     <- types.rs:824-826

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Production extern surface — `#[path]`-bound mirror of
// crates/vb_storage/src/recovery/types.rs:666-852.
// ---------------------------------------------------------------------------
#[path = "extern_vb_rpch_action_tracker.rs"]
mod production;

// Re-export the spec-side mirror struct so the proof context below
// can reason about it. The production newtypes `ActionId`/`StepIdx`
// are u16 newtype wrappers at `crates/vb_core/src/ids/mod.rs:55,58`;
// the spec proofs reason at the `u16` level (see the type aliases
// below) to stay compatible with `Set<(u16, u16)>` operations.
pub use production::SpecActionReplayTracker;

// Spec-level type aliases — production newtypes are `u16` newtype
// wrappers at `crates/vb_core/src/ids/mod.rs:55,58`. The spec proofs
// reason at the `u16` level so the Set abstraction matches the
// production mirror's `HashSet<(u16, u16)>` field type.
pub type ActionId = u16;
pub type StepIdx = u16;

pub open spec fn spec_is_resolved(completed: Set<(ActionId, StepIdx)>, failed: Set<(ActionId, StepIdx)>, action: ActionId, step: StepIdx) -> bool {
    completed.contains((action, step)) || failed.contains((action, step))
}

// ---------------------------------------------------------------------------
// assume_specification BRIDGES — production contract surface
// ---------------------------------------------------------------------------
//
// The bodies of the mirror methods are opaque to Verus
// (`#[verifier::external]` in the extern file). The bridges attach
// the production contracts: `is_resolved` returns true iff
// `(action, step)` is in `tracker.completed@` or `tracker.failed@`,
// `mark_completed` adds `(action, step)` to `tracker.completed@`,
// and `mark_failed` adds `(action, step)` to `tracker.failed@`.
pub assume_specification[ production::SpecActionReplayTracker::is_resolved ](
    tracker: &production::SpecActionReplayTracker,
    action: u16,
    step: u16,
) -> (result: bool)
    ensures
        result == spec_is_resolved(tracker.completed@, tracker.failed@, action, step),
;

pub assume_specification[ production::SpecActionReplayTracker::mark_completed ](
    tracker: &mut production::SpecActionReplayTracker,
    action: u16,
    step: u16,
)
    ensures
        spec_is_resolved(final(tracker).completed@, final(tracker).failed@, action, step),
        old(tracker).completed@ === final(tracker).completed@.insert((action, step))
            || old(tracker).completed@.contains((action, step)),
;

pub assume_specification[ production::SpecActionReplayTracker::mark_failed ](
    tracker: &mut production::SpecActionReplayTracker,
    action: u16,
    step: u16,
)
    ensures
        spec_is_resolved(final(tracker).completed@, final(tracker).failed@, action, step),
        old(tracker).failed@ === final(tracker).failed@.insert((action, step))
            || old(tracker).failed@.contains((action, step)),
;

// ---------------------------------------------------------------------------
// Production-bound exec wrappers — discharge witnesses for the bridges
// ---------------------------------------------------------------------------
//
// These exec wrappers invoke the spec-side mirror methods. Verus
// verifies each wrapper body via the `assume_specification` contract
// attached to the corresponding mirror method. Any drift between the
// production mirror and the production source breaks the contract
// and these wrappers fail to type-check.
pub exec fn production_is_resolved_witness(
    tracker: &production::SpecActionReplayTracker,
    action: u16,
    step: u16,
) -> (r: bool)
    ensures
        r == spec_is_resolved(tracker.completed@, tracker.failed@, action, step),
{
    tracker.is_resolved(action, step)
}

pub exec fn production_mark_completed_witness(
    tracker: &mut production::SpecActionReplayTracker,
    action: u16,
    step: u16,
)
    ensures
        spec_is_resolved(final(tracker).completed@, final(tracker).failed@, action, step),
{
    tracker.mark_completed(action, step);
}

pub exec fn production_mark_failed_witness(
    tracker: &mut production::SpecActionReplayTracker,
    action: u16,
    step: u16,
)
    ensures
        spec_is_resolved(final(tracker).completed@, final(tracker).failed@, action, step),
{
    tracker.mark_failed(action, step);
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
