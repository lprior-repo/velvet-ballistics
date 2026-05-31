//! PS-008 Verus proof: Capacity admission (POB-vb-fzgdn-033)
//! Production binding: crates/vb_runtime/src/shard timer registry admission paths
//!
//! Models: bounded timer registry with max capacity. Admission returns typed
//! capacity error when full and leaves state unchanged on rejection.

use vstd::prelude::*;

verus! {

/// Bounded timer registry model.
pub struct TimerRegistry {
    pub count: usize,
    pub max_capacity: usize,
}

impl TimerRegistry {
    /// Attempt to insert a timer. Returns new registry with incremented count
    /// or same registry if at capacity.
    pub closed spec fn try_insert_spec(self) -> (Self, bool) {
        if self.count < self.max_capacity {
            (TimerRegistry { count: self.count + 1, max_capacity: self.max_capacity }, true)
        } else {
            (self, false)
        }
    }

    /// Remove a timer (unconditional, for fire/cancel).
    pub closed spec fn remove_spec(self) -> Self {
        if self.count > 0 {
            TimerRegistry { count: (self.count - 1) as usize, max_capacity: self.max_capacity }
        } else {
            self
        }
    }
}

/// Theorem: Insert into non-full registry succeeds and increments count.
proof fn test_insert_non_full_succeeds()
    ensures
        forall |r: TimerRegistry| r.count < r.max_capacity ==>
            r.try_insert_spec().1
            && r.try_insert_spec().0.count == r.count + 1,
{
    assert forall |r: TimerRegistry| r.count < r.max_capacity ==>
        r.try_insert_spec().1
        && r.try_insert_spec().0.count == r.count + 1 by {
        if r.count < r.max_capacity {
            let (next, ok) = r.try_insert_spec();
            assert(ok);
            assert(next.count == r.count + 1);
        }
    };
}

/// Theorem: Insert into full registry fails and leaves count unchanged.
proof fn test_insert_full_fails()
    ensures
        forall |r: TimerRegistry| r.count >= r.max_capacity ==>
            !r.try_insert_spec().1
            && r.try_insert_spec().0.count == r.count,
{
    assert forall |r: TimerRegistry| r.count >= r.max_capacity ==>
        !r.try_insert_spec().1
        && r.try_insert_spec().0.count == r.count by {
        if r.count >= r.max_capacity {
            let (next, ok) = r.try_insert_spec();
            assert(!ok);
            assert(next.count == r.count);
        }
    };
}

/// Theorem: Remove from non-empty registry decrements count.
proof fn test_remove_decrements()
    ensures
        forall |r: TimerRegistry| r.count > 0 ==>
            r.remove_spec().count == r.count - 1,
{
    assert forall |r: TimerRegistry| r.count > 0 ==>
        r.remove_spec().count == r.count - 1 by {
        if r.count > 0 {
            assert(r.remove_spec().count == r.count - 1);
        }
    };
}

/// Theorem: Remove from empty registry stays empty.
proof fn test_remove_empty_stays_empty()
    ensures TimerRegistry { count: 0, max_capacity: 10 }.remove_spec().count == 0,
{
    assert(TimerRegistry { count: 0, max_capacity: 10 }.remove_spec().count == 0) by (compute);
}

} // verus!
