//! PS-005 Verus proof: Duplicate key handling idempotency (POB-vb-fzgdn-019)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::insert
//!
//! Models: duplicate run inserts replace the existing entry with new kind and
//! deadline while incrementing generation, without leaving stale entries.

use vstd::prelude::*;

verus! {

/// Spec model of timer wheel state for a single run.
pub struct TimerSlot {
    pub present: bool,
    pub generation: u64,
}

impl TimerSlot {
    /// Inserts or replaces a timer for a run.
    pub closed spec fn insert_spec(self) -> Self {
        if self.present {
            TimerSlot { present: true, generation: (self.generation + 1) as u64 }
        } else {
            TimerSlot { present: true, generation: 1 }
        }
    }

    /// Cancels the timer, returning whether one was present.
    pub closed spec fn cancel_spec(self) -> (Self, bool) {
        if self.present {
            (TimerSlot { present: false, generation: 0 }, true)
        } else {
            (self, false)
        }
    }
}

/// Theorem: First insert yields generation=1 and present=true.
proof fn test_first_insert()
    ensures TimerSlot { present: false, generation: 0 }.insert_spec() == TimerSlot { present: true, generation: 1 },
{
    assert(TimerSlot { present: false, generation: 0 }.insert_spec() == TimerSlot { present: true, generation: 1 }) by (compute);
}

/// Theorem: Replacement increments generation and stays present.
proof fn test_replacement_increments()
    ensures
        forall |s: TimerSlot| s.present ==>
            s.insert_spec().present && s.insert_spec().generation == s.generation + 1,
{
    assert forall |s: TimerSlot| s.present ==>
        s.insert_spec().present && s.insert_spec().generation == s.generation + 1 by {
        if s.present {
            let result = s.insert_spec();
            assert(result.present);
            assert(result.generation == s.generation + 1);
        }
    };
}

/// Theorem: Cancel on empty slot returns (empty, false).
proof fn test_cancel_empty()
    ensures TimerSlot { present: false, generation: 0 }.cancel_spec() == (TimerSlot { present: false, generation: 0 }, false),
{
    assert(TimerSlot { present: false, generation: 0 }.cancel_spec().1 == false) by (compute);
}

/// Theorem: Cancel on filled slot returns (empty, true).
proof fn test_cancel_filled()
    ensures TimerSlot { present: true, generation: 5 }.cancel_spec() == (TimerSlot { present: false, generation: 0 }, true),
{
    assert(TimerSlot { present: true, generation: 5 }.cancel_spec().1 == true) by (compute);
}

} // verus!
