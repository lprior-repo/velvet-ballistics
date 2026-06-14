//! PS-001 Verus proof: TimerDeadline arithmetic safety (POB-vb-fzgdn-001)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel
//!                     crates/vb_runtime/src/shard/transitions.rs Shard::await_timer
//!
//! Models u64 addition with checked overflow semantics matching production
//! next_pending_timer_generation and TimerWheel::insert generation advancement.
//!
//! Trusted boundary: Instant is opaque; we model only the numeric u64 generation.

use vstd::prelude::*;

verus! {

/// Model of a timer generation value constrained to u64.
pub struct TimerGeneration {
    pub value: u64,
}

impl TimerGeneration {
    /// Creates a new timer generation value, bound by u64::MAX.
    pub closed spec fn new_spec(value: u64) -> Self {
        TimerGeneration { value }
    }

    /// Checked increment: returns Option-like modeling of checked_add(1).
    /// Production code: Shard::next_pending_timer_generation uses checked_add(1).
    pub closed spec fn checked_increment_spec(self) -> Option<Self> {
        if self.value < u64::MAX {
            Some(TimerGeneration { value: (self.value + 1u64) as u64 })
        } else {
            None
        }
    }
}

/// Verifies that checked_add on values < MAX succeeds with exact increment.
proof fn test_checked_add_within_bounds()
    ensures
        forall |g: TimerGeneration|
            g.value < u64::MAX ==>
            #[trigger] g.checked_increment_spec().is_Some()
            && g.checked_increment_spec().get_Some_0().value == g.value + 1,
{
    assert forall |g: TimerGeneration|
        g.value < u64::MAX implies
        #[trigger] g.checked_increment_spec().is_Some()
        && g.checked_increment_spec().get_Some_0().value == g.value + 1 by {
        if g.value < u64::MAX {
            assert(g.checked_increment_spec().is_Some());
            assert(g.checked_increment_spec().get_Some_0().value == g.value + 1);
        }
    };
}

/// Verifies that MAX generation + 1 returns None (GenerationExhausted).
proof fn test_checked_add_at_max()
    ensures
        (TimerGeneration { value: u64::MAX }).checked_increment_spec().is_None(),
{
    assert((TimerGeneration { value: u64::MAX }).checked_increment_spec().is_None());
}

/// Verifies monotonicity: increment always yields strictly greater value or None.
proof fn test_increment_monotonic()
    ensures
        forall |g: TimerGeneration|
            if g.checked_increment_spec().is_Some() {
                g.checked_increment_spec().get_Some_0().value > g.value
            },
{
    assert forall |g: TimerGeneration|
        if g.checked_increment_spec().is_Some() {
            g.checked_increment_spec().get_Some_0().value > g.value
        } by {
        if g.checked_increment_spec().is_Some() {
            let next = g.checked_increment_spec().get_Some_0();
            assert(next.value == g.value + 1);
            assert(next.value > g.value);
        }
    };
}

} // verus!
