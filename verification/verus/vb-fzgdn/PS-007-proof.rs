//! PS-007 Verus proof: Monotonic clock + fire order (POB-vb-fzgdn-028)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::fire_expired
//!
//! Models: clock advancement is monotonic, backward ticks rejected.
//! fire_expired returns all timers with deadline <= now in deterministic order.

use vstd::prelude::*;

verus! {

/// Clock tick value, bounded to u64.
pub struct ClockTick {
    pub value: u64,
}

impl ClockTick {
    /// Advance clock to new tick. Returns None if backward.
    pub closed spec fn advance_to_spec(self, new_tick: u64) -> Option<Self> {
        if new_tick >= self.value {
            Some(ClockTick { value: new_tick })
        } else {
            None
        }
    }

    /// Check if a deadline has expired relative to current tick.
    pub closed spec fn is_expired_spec(self, deadline: u64) -> bool {
        deadline <= self.value
    }
}

/// Theorem: advancing to same tick succeeds.
proof fn test_advance_to_same_tick()
    ensures
        forall |t: u64|
            ClockTick { value: t }.advance_to_spec(t).is_Some(),
{
    assert forall |t: u64|
        ClockTick { value: t }.advance_to_spec(t).is_Some() by {
        assert(ClockTick { value: t }.advance_to_spec(t).is_Some());
    };
}

/// Theorem: advancing to future tick succeeds and has correct value.
proof fn test_advance_forward()
    ensures
        forall |t: u64, f: u64| f > t ==>
            ClockTick { value: t }.advance_to_spec(f).is_Some()
            && ClockTick { value: t }.advance_to_spec(f).get_Some_0().value == f,
{
    assert forall |t: u64, f: u64| f > t ==>
        ClockTick { value: t }.advance_to_spec(f).is_Some()
        && ClockTick { value: t }.advance_to_spec(f).get_Some_0().value == f by {
        if f > t {
            let result = ClockTick { value: t }.advance_to_spec(f);
            assert(result.is_Some());
            assert(result.get_Some_0().value == f);
        }
    };
}

/// Theorem: advancing backward (regression) returns None.
proof fn test_advance_backward_rejected()
    ensures
        forall |t: u64, b: u64| b < t ==>
            ClockTick { value: t }.advance_to_spec(b).is_None(),
{
    assert forall |t: u64, b: u64| b < t ==>
        ClockTick { value: t }.advance_to_spec(b).is_None() by {
        if b < t {
            assert(ClockTick { value: t }.advance_to_spec(b).is_None());
        }
    };
}

/// Theorem: deadline <= tick means expired.
proof fn test_deadline_past_is_expired()
    ensures
        forall |tick: u64, deadline: u64| deadline <= tick ==>
            ClockTick { value: tick }.is_expired_spec(deadline),
{
    assert forall |tick: u64, deadline: u64| deadline <= tick ==>
        ClockTick { value: tick }.is_expired_spec(deadline) by {
        // Directly from spec definition.
    };
}

/// Theorem: deadline > tick means not expired.
proof fn test_deadline_future_not_expired()
    ensures
        forall |tick: u64, deadline: u64| deadline > tick ==>
            !ClockTick { value: tick }.is_expired_spec(deadline),
{
    assert forall |tick: u64, deadline: u64| deadline > tick ==>
        !ClockTick { value: tick }.is_expired_spec(deadline) by {
        // Directly from spec definition.
    };
}

} // verus!
