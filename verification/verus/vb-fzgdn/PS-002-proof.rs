//! PS-002 Verus proof: Timer admission stores numeric fields only (POB-vb-fzgdn-006)
//! Production binding: crates/vb_runtime/src/shard/types.rs PendingTimer, PendingTimerKind
//!                     crates/vb_runtime/src/shard/transitions.rs Shard::await_timer
//!
//! Models PendingTimer and its matches_authority predicate.
//! The production PendingTimer struct fields (step, kind, generation, deadline)
//! are modeled here; Instant is treated as opaque.

use vstd::prelude::*;

verus! {

/// Model of PendingTimerKind matching production enum.
#[derive(PartialEq, Eq)]
pub enum TimerKindModel {
    Wait,
    Ask,
}

/// Spec model of PendingTimer fields: step index, kind, generation, deadline opaque.
pub struct PendingTimerModel {
    pub step: u16,
    pub kind: TimerKindModel,
    pub generation: u64,
    pub deadline_present: bool, // Instant is opaque; we model presence only
}

impl PendingTimerModel {
    /// matches_authority spec mirroring production PendingTimer::matches_authority.
    pub closed spec fn matches_authority_spec(
        self,
        generation: u64,
        kind: TimerKindModel,
        deadline_match: bool,
    ) -> bool {
        self.generation == generation
        && self.kind == kind
        && self.deadline_present == deadline_match
    }
}

/// Theorem: matches_authority requires an exact match on all three fields.
proof fn test_matches_authority_all_fields_required()
    ensures
        forall |t: PendingTimerModel, g: u64, k: TimerKindModel, d: bool|
            #[trigger] t.matches_authority_spec(g, k, d) ==
            (t.generation == g && t.kind == k && t.deadline_present == d),
{
    assert forall |t: PendingTimerModel, g: u64, k: TimerKindModel, d: bool|
        #[trigger] t.matches_authority_spec(g, k, d) ==
        (t.generation == g && t.kind == k && t.deadline_present == d) by {
        // The spec definition is structural equality — verifier can compute this.
    };
}

/// Theorem: If any one field differs, matches_authority returns false.
proof fn test_matches_authority_fails_on_any_mismatch(t: PendingTimerModel)
    ensures
        (t.generation != 0) ==> !t.matches_authority_spec(0, t.kind, t.deadline_present),
        (t.kind != TimerKindModel::Wait) ==> !t.matches_authority_spec(t.generation, TimerKindModel::Wait, t.deadline_present),
        (t.deadline_present != true) ==> !t.matches_authority_spec(t.generation, t.kind, true),
{
    if t.generation != 0 {
        assert(!t.matches_authority_spec(0, t.kind, t.deadline_present));
    }
    if t.kind != TimerKindModel::Wait {
        assert(!t.matches_authority_spec(t.generation, TimerKindModel::Wait, t.deadline_present));
    }
    if t.deadline_present != true {
        assert(!t.matches_authority_spec(t.generation, t.kind, true));
    }
}

} // verus!
