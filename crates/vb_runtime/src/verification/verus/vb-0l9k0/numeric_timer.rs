//! Verus specification and proof for Numeric Timer Seam — vb-0l9k0.
//!
//! Production bindings:
//! - `spec_timer_tick_elapsed` → `shard/timer.rs:76-78`
//! - `spec_timer_deadline_from_tick_and_duration` → `shard/timer.rs:132-134`
//! - `spec_timer_deadline_is_past` → `shard/timer.rs:138-140`

use vstd::prelude::*;

verus! {

    // ===========================================================================
    // Spec: timer tick elapsed check
    //
    // Production binding: shard/timer.rs:76-78
    //
    //   pub fn has_elapsed(self, deadline: TimerDeadline) -> bool {
    //       self.0 >= deadline.get()
    //   }
    // ===========================================================================

    pub closed spec fn spec_timer_tick_elapsed(current_tick: nat, deadline: u64) -> bool {
        current_tick >= deadline as nat
    }

    // ===========================================================================
    // Proof: timer tick at deadline is elapsed
    // ===========================================================================

    pub proof fn proof_timer_tick_at_deadline_is_elapsed(deadline: u64)
        ensures
            spec_timer_tick_elapsed(deadline as nat, deadline),
    {
        assert(spec_timer_tick_elapsed(deadline as nat, deadline));
    }

    // ===========================================================================
    // Proof: timer tick past deadline is elapsed
    // ===========================================================================

    pub proof fn proof_timer_tick_past_deadline_is_elapsed(deadline: u64, extra: nat)
        requires
            extra > 0,
        ensures
            spec_timer_tick_elapsed((deadline as nat) + extra, deadline),
    {
        assert(spec_timer_tick_elapsed((deadline as nat) + extra, deadline));
    }

    // ===========================================================================
    // Proof: timer tick before deadline is NOT elapsed
    // ===========================================================================

    pub proof fn proof_timer_tick_before_deadline_not_elapsed(deadline: u64, before: u64)
        requires
            before < deadline,
        ensures
            !spec_timer_tick_elapsed(before as nat, deadline),
    {
        assert(!spec_timer_tick_elapsed(before as nat, deadline));
    }

    // ===========================================================================
    // Spec: timer deadline from tick and duration
    //
    // Production binding: shard/timer.rs:132-134
    //
    //   pub fn from_tick_and_duration(tick: TimerTick, duration: TimerDuration) -> Option<Self> {
    //       tick.get().checked_add(duration.get()).map(Self)
    //   }
    // ===========================================================================

    pub closed spec fn spec_timer_deadline_from_tick_and_duration(
        tick: u64,
        duration: u64,
    ) -> Option<u64> {
        tick.checked_add(duration)
    }

    // ===========================================================================
    // Proof: deadline from tick and duration is Some when no overflow
    // ===========================================================================

    pub proof fn proof_deadline_from_tick_no_overflow(
        tick: u64,
        duration: u64,
    )
        requires
            tick <= u64::MAX - duration,
        ensures
            spec_timer_deadline_from_tick_and_duration(tick, duration).is_some(),
    {
        assert(spec_timer_deadline_from_tick_and_duration(tick, duration).is_some());
    }

    // ===========================================================================
    // Proof: deadline from tick and duration is None on overflow
    // ===========================================================================

    pub proof fn proof_deadline_from_tick_overflow(
        tick: u64,
        duration: u64,
    )
        requires
            tick > u64::MAX - duration,
        ensures
            spec_timer_deadline_from_tick_and_duration(tick, duration).is_none(),
    {
        assert(spec_timer_deadline_from_tick_and_duration(tick, duration).is_none());
    }

    // ===========================================================================
    // Spec: timer deadline is past
    //
    // Production binding: shard/timer.rs:138-140
    //
    //   pub fn is_past(self, current: TimerTick) -> bool {
    //       current.has_elapsed(self)
    //   }
    // ===========================================================================

    pub closed spec fn spec_timer_deadline_is_past(current_tick: u64, deadline: u64) -> bool {
        spec_timer_tick_elapsed(current_tick as nat, deadline)
    }

    // ===========================================================================
    // Theorem: timer deadline past is equivalent to tick elapsed
    // ===========================================================================

    pub proof fn theorem_timer_deadline_past_equivalence(
        current_tick: u64,
        deadline: u64,
    )
        ensures
            spec_timer_deadline_is_past(current_tick, deadline) == spec_timer_tick_elapsed(current_tick as nat, deadline),
    {
        assert(spec_timer_deadline_is_past(current_tick, deadline) == spec_timer_tick_elapsed(current_tick as nat, deadline));
    }

} // verus!
