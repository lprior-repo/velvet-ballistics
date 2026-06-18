//! Verus specification and proof for Pending Timer — vb-0l9k0.
//!
//! Production bindings:
//! - `spec_pending_timer_matches_authority` → `shard/timer.rs:31-38`

use vstd::prelude::*;

verus! {

    // ===========================================================================
    // Spec: pending timer matches authority
    //
    // Production binding: shard/timer.rs:31-38
    //
    //   pub fn matches_authority(
    //       self,
    //       generation: u64,
    //       deadline: Instant,
    //       kind: PendingTimerKind,
    //   ) -> bool {
    //       self.generation == generation && self.deadline == deadline && self.kind == kind
    //   }
    //
    // Simplified: we model Instant as u64 for spec purposes.
    // ===========================================================================

    pub closed spec fn spec_pending_timer_matches_authority(
        timer_generation: u64,
        timer_deadline: u64,
        timer_kind: u8, // 0=Wait, 1=Ask (simplified)
        authority_generation: u64,
        authority_deadline: u64,
        authority_kind: u8,
    ) -> bool {
        timer_generation == authority_generation
            && timer_deadline == authority_deadline
            && timer_kind == authority_kind
    }

    // ===========================================================================
    // Proof: matching authority when all fields equal
    // ===========================================================================

    pub proof fn proof_matches_authority_all_equal(
        generation: u64,
        deadline: u64,
        kind: u8,
    )
        ensures
            spec_pending_timer_matches_authority(
                generation, deadline, kind,
                generation, deadline, kind,
            ),
    {
        assert(spec_pending_timer_matches_authority(
            generation, deadline, kind,
            generation, deadline, kind,
        ));
    }

    // ===========================================================================
    // Proof: NOT matching when generation differs
    // ===========================================================================

    pub proof fn proof_not_matches_authority_generation_differs(
        timer_generation: u64,
        timer_deadline: u64,
        timer_kind: u8,
        authority_generation: u64,
        authority_deadline: u64,
        authority_kind: u8,
    )
        requires
            timer_generation != authority_generation,
        ensures
            !spec_pending_timer_matches_authority(
                timer_generation, timer_deadline, timer_kind,
                authority_generation, authority_deadline, authority_kind,
            ),
    {
        assert(!spec_pending_timer_matches_authority(
            timer_generation, timer_deadline, timer_kind,
            authority_generation, authority_deadline, authority_kind,
        ));
    }

    // ===========================================================================
    // Proof: NOT matching when deadline differs
    // ===========================================================================

    pub proof fn proof_not_matches_authority_deadline_differs(
        timer_generation: u64,
        timer_deadline: u64,
        timer_kind: u8,
        authority_generation: u64,
        authority_deadline: u64,
        authority_kind: u8,
    )
        requires
            timer_generation == authority_generation
                && timer_deadline != authority_deadline
                && timer_kind == authority_kind,
        ensures
            !spec_pending_timer_matches_authority(
                timer_generation, timer_deadline, timer_kind,
                authority_generation, authority_deadline, authority_kind,
            ),
    {
        assert(!spec_pending_timer_matches_authority(
            timer_generation, timer_deadline, timer_kind,
            authority_generation, authority_deadline, authority_kind,
        ));
    }

    // ===========================================================================
    // Proof: NOT matching when kind differs
    // ===========================================================================

    pub proof fn proof_not_matches_authority_kind_differs(
        timer_generation: u64,
        timer_deadline: u64,
        timer_kind: u8,
        authority_generation: u64,
        authority_deadline: u64,
        authority_kind: u8,
    )
        requires
            timer_generation == authority_generation
                && timer_deadline == authority_deadline
                && timer_kind != authority_kind,
        ensures
            !spec_pending_timer_matches_authority(
                timer_generation, timer_deadline, timer_kind,
                authority_generation, authority_deadline, authority_kind,
            ),
    {
        assert(!spec_pending_timer_matches_authority(
            timer_generation, timer_deadline, timer_kind,
            authority_generation, authority_deadline, authority_kind,
        ));
    }

    // ===========================================================================
    // Theorem: matches authority is a well-defined predicate
    // ===========================================================================

    pub proof fn theorem_matches_authority_predicate(
        timer_generation: u64,
        timer_deadline: u64,
        timer_kind: u8,
        authority_generation: u64,
        authority_deadline: u64,
        authority_kind: u8,
    )
        ensures
            spec_pending_timer_matches_authority(timer_generation, timer_deadline, timer_kind, authority_generation, authority_deadline, authority_kind) || !spec_pending_timer_matches_authority(timer_generation, timer_deadline, timer_kind, authority_generation, authority_deadline, authority_kind),
    {
        assert(spec_pending_timer_matches_authority(timer_generation, timer_deadline, timer_kind, authority_generation, authority_deadline, authority_kind) || !spec_pending_timer_matches_authority(timer_generation, timer_deadline, timer_kind, authority_generation, authority_deadline, authority_kind));
    }

} // verus!
