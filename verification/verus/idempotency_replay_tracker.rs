// Verus proof obligations for vb-qi37.5 replay tracker monotonicity.
//
// Obligation: VERUS-REPLAY-004.
// Standalone model: replay state is represented by resolved/scheduled flags for
// one action-step pair, abstracting durable journal decoding and Fjall storage.
// Exact verifier command: `verus verification/verus/idempotency_replay_tracker.rs`.

use vstd::prelude::*;

verus! {

pub open spec fn spec_replay_tracker_resolved(resolved: bool, scheduled: bool) -> bool {
    resolved ==> !scheduled
}

pub open spec fn spec_mark_resolved(resolved: bool, scheduled: bool) -> (bool, bool) {
    (true, false)
}

pub open spec fn spec_retry_allowed(resolved: bool, is_idempotent: bool) -> bool {
    !resolved || is_idempotent
}

pub proof fn proof_resolved_action_monotonic(resolved: bool, scheduled: bool)
    requires
        spec_replay_tracker_resolved(resolved, scheduled),
    ensures
        spec_replay_tracker_resolved(spec_mark_resolved(resolved, scheduled).0,
                                     spec_mark_resolved(resolved, scheduled).1),
{
}

pub proof fn proof_resolved_non_idempotent_not_rescheduled()
    ensures
        !spec_retry_allowed(true, false),
{
}

pub proof fn proof_unresolved_action_may_be_scheduled(is_idempotent: bool)
    ensures
        spec_retry_allowed(false, is_idempotent),
{
}

pub proof fn proof_resolved_idempotent_retry_is_only_collapsed_observation()
    ensures
        spec_retry_allowed(true, true),
{
}

fn main() {}

} // verus!
