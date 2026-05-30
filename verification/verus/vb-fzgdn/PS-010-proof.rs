//! PS-010 Verus proof: Atomic timer fire + state preservation (POB-vb-fzgdn-042)
//! Production binding: crates/vb_runtime/src/shard/lifecycle/chunk_002.rs Shard::handle_timer
//!
//! Models: a valid timer fire atomically removes pending state and enqueues
//! delayed action. If enqueue fails, pending state is restored (preserved).
//! No partial mutation: either both succeed or both fail.

use vstd::prelude::*;

verus! {

/// Shard timer state model: pending timer present or absent.
pub struct PendingTimerState {
    pub present: bool,
}

/// Command queue model with bounded capacity.
pub struct CommandQueueModel {
    pub count: usize,
    pub capacity: usize,
}

/// Spec for atomic fire: removes pending timer and enqueues iff queue has room.
pub closed spec fn atomic_fire_spec(
    pending: PendingTimerState,
    queue: CommandQueueModel,
) -> (PendingTimerState, CommandQueueModel, bool) {
    if pending.present && queue.count < queue.capacity {
        (
            PendingTimerState { present: false },
            CommandQueueModel { count: queue.count + 1, capacity: queue.capacity },
            true,
        )
    } else {
        (pending, queue, false)
    }
}

/// Theorem: When both conditions met, timer removed and command enqueued.
proof fn test_fire_succeeds_when_room()
    ensures
        forall |q: CommandQueueModel| q.count < q.capacity ==>
            atomic_fire_spec(PendingTimerState { present: true }, q).2,
{
    assert forall |q: CommandQueueModel| q.count < q.capacity ==>
        atomic_fire_spec(PendingTimerState { present: true }, q).2 by {
        if q.count < q.capacity {
            let (_, _, ok) = atomic_fire_spec(PendingTimerState { present: true }, q);
            assert(ok);
        }
    };
}

/// Theorem: When queue is full, state preserved unchanged.
proof fn test_fire_preserves_when_full()
    ensures
        forall |q: CommandQueueModel| q.count >= q.capacity ==>
            atomic_fire_spec(PendingTimerState { present: true }, q) == (PendingTimerState { present: true }, q, false),
{
    assert forall |q: CommandQueueModel| q.count >= q.capacity ==>
        atomic_fire_spec(PendingTimerState { present: true }, q) == (PendingTimerState { present: true }, q, false) by {
        if q.count >= q.capacity {
            let result = atomic_fire_spec(PendingTimerState { present: true }, q);
            assert(result.0.present == true);
            assert(result.2 == false);
        }
    };
}

/// Theorem: When no pending timer, fire is no-op (preserves state).
proof fn test_fire_noop_when_no_pending()
    ensures
        forall |q: CommandQueueModel|
            atomic_fire_spec(PendingTimerState { present: false }, q) == (PendingTimerState { present: false }, q, false),
{
    assert forall |q: CommandQueueModel|
        atomic_fire_spec(PendingTimerState { present: false }, q) == (PendingTimerState { present: false }, q, false) by {
        let result = atomic_fire_spec(PendingTimerState { present: false }, q);
        assert(result.0.present == false);
        assert(result.2 == false);
    };
}

/// Theorem: Successful fire produces consistent post-state.
proof fn test_fire_post_state_consistent()
    ensures
        forall |q: CommandQueueModel| q.count < q.capacity ==>
        {
            let (pending, queue, ok) = atomic_fire_spec(PendingTimerState { present: true }, q);
            ok ==> !pending.present && queue.count == q.count + 1
        },
{
    assert forall |q: CommandQueueModel| q.count < q.capacity ==>
    {
        let (pending, queue, ok) = atomic_fire_spec(PendingTimerState { present: true }, q);
        ok ==> !pending.present && queue.count == q.count + 1
    } by {
        if q.count < q.capacity {
            let (pending, queue, ok) = atomic_fire_spec(PendingTimerState { present: true }, q);
            assert(!pending.present);
            assert(queue.count == q.count + 1);
        }
    };
}

} // verus!
