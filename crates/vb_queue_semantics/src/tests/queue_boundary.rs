//! Boundary tests for queue-state transition semantics.
//!
//! These tests exercise the verified gaps around warning thresholds,
//! saturating arithmetic at the usize boundary, capacity=1 lifecycle,
//! empty-dequeue discipline, full-queue enqueue rejection, and shard-tick
//! consumption.

use crate::{
    EnqueueDecision, PopTransition, QueueState, ShardTickTransition,
    action_dequeue_transition, action_enqueue_transition, remaining_capacity,
    shard_tick_transition, warning_payload,
};

// ─── Gap 1: warning_payload when depth > capacity ──────────────────────────────
/// warning_payload returns None when depth exceeds capacity.
/// The guard `depth >= threshold && depth <= capacity` requires both conditions;
/// when `depth > capacity` the second fails regardless of threshold.
///
/// capacity=1 -> threshold=1 (1*8/10=0, clamped to 1)
/// depth=2 satisfies depth >= threshold but fails depth <= capacity.
#[test]
fn warning_payload_none_when_depth_exceeds_capacity_one() {
    // capacity=1, threshold(1)=1, depth=2: 2>=1 passes, 2<=1 fails -> None
    assert_eq!(warning_payload(1, 2), None);
}

/// warning_payload returns None when depth exceeds capacity for a larger gap.
///
/// capacity=5 -> threshold(5)=4 (5*8/10=4)
/// depth=10: 10>=4 passes, 10<=5 fails -> None
#[test]
fn warning_payload_none_when_depth_exceeds_capacity_large_gap() {
    assert_eq!(warning_payload(5, 10), None);
}

/// warning_payload returns None when depth is far above capacity.
///
/// capacity=3 -> threshold(3)=2 (3*8/10=2)
/// depth=5: 5>=2 passes, 5<=3 fails -> None
#[test]
fn warning_payload_none_when_depth_exceeds_capacity_three() {
    assert_eq!(warning_payload(3, 5), None);
}

/// warning_payload returns None when capacity is zero regardless of depth.
/// threshold(0)=0, depth=0: 0>=0 passes, 0<=0 passes, but we still expect None
/// because a zero-capacity queue is invalid and the guard semantics are
/// designed to only produce warnings for real bounded queues.
#[test]
fn warning_payload_none_when_capacity_zero() {
    assert_eq!(warning_payload(0, 0), None);
}

// ─── Gap 2: remaining_capacity at usize::MAX ───────────────────────────────────
/// remaining_capacity(usize::MAX, usize::MAX) must be exactly 0, not usize::MAX.
/// The function uses saturating_sub, which returns 0 when len >= capacity even
/// at the usize boundary.
#[test]
fn remaining_capacity_usize_max_eq_is_zero() {
    assert_eq!(
        remaining_capacity(usize::MAX, usize::MAX),
        0,
        "saturating_sub of equal usize::MAX values must be 0, not usize::MAX"
    );
}

/// remaining_capacity at usize::MAX with one item consumed must be usize::MAX - 1,
/// not saturating to zero. saturating_sub correctly decrements from the maximum.
#[test]
fn remaining_capacity_usize_max_minus_one() {
    assert_eq!(
        remaining_capacity(usize::MAX, 1),
        usize::MAX - 1,
        "remaining capacity should be usize::MAX - 1 when one slot is used"
    );
}

/// remaining_capacity(usize::MAX, usize::MAX - 1) must be 1.
/// The single available slot is correctly reported at the boundary.
#[test]
fn remaining_capacity_usize_max_minus_one_len() {
    assert_eq!(
        remaining_capacity(usize::MAX, usize::MAX - 1),
        1,
        "one slot remaining when len == usize::MAX - 1"
    );
}

// ─── Gap 3: QueueState capacity=1 lifecycle ───────────────────────────────────
/// Full lifecycle for a minimum-valid-capacity queue (capacity=1):
/// empty -> enqueue(accepted) -> full -> enqueue(rejected) ->
/// dequeue(item=42) -> empty -> enqueue(accepted) -> full ->
/// dequeue(item=7) -> empty.
#[test]
fn queue_state_capacity_one_full_lifecycle() {
    // Start: empty
    let state = empty_state(1);
    assert!(state.is_empty());
    assert!(!state.is_full());
    assert_eq!(state.len(), 0);
    assert_eq!(state.capacity(), 1);

    // First enqueue: accepted
    let (state, decision) = action_enqueue_transition(state, 42);
    assert_eq!(decision, EnqueueDecision::Accepted);
    assert_eq!(state.len(), 1);
    assert!(state.is_full());
    assert!(state.is_empty() == false);

    // Second enqueue: rejected, state unchanged
    let (state, decision) = action_enqueue_transition(state, 99);
    assert_eq!(decision, EnqueueDecision::QueueFull { capacity: 1 });
    assert_eq!(state.len(), 1, "rejected enqueue must not mutate the queue");

    // Dequeue: returns item 42, queue is now empty
    let PopTransition::Popped { state, item } = action_dequeue_transition(state) else {
        panic!("expected Popped at capacity=1 with one item");
    };
    assert_eq!(item, 42);
    assert!(state.is_empty());
    assert!(!state.is_full());
    assert_eq!(state.len(), 0);

    // Re-enqueue after dequeue: accepted again
    let (state, decision) = action_enqueue_transition(state, 7);
    assert_eq!(decision, EnqueueDecision::Accepted);
    assert_eq!(state.len(), 1);
    assert!(state.is_full());

    // Dequeue the second item
    let PopTransition::Popped { item, .. } = action_dequeue_transition(state) else {
        panic!("expected Popped, got Empty");
    };
    assert_eq!(item, 7);
}

/// Capacity=1 queue: after dequeue recovers empty state, enqueue should
/// succeed with any new item -- the item value does not matter.
#[test]
fn queue_state_capacity_one_enqueue_after_dequeue_accepts_any_item() {
    let state = empty_state(1);
    let (state, _) = action_enqueue_transition(state, 100);
    let PopTransition::Popped { state, .. } = action_dequeue_transition(state) else {
        panic!("expected Popped");
    };
    let (state, decision) = action_enqueue_transition(state, 200);
    assert_eq!(decision, EnqueueDecision::Accepted);
    assert_eq!(state.len(), 1);
}

// ─── Gap 4: action_dequeue_transition on empty queue ──────────────────────────
/// Dequeue from an empty capacity=1 queue returns Empty and preserves the
/// original empty state (capacity and empty length).
#[test]
fn action_dequeue_transition_empty_capacity_one() {
    let PopTransition::Empty { state } = action_dequeue_transition(empty_state(1)) else {
        panic!("expected Empty, got Popped");
    };
    assert!(state.is_empty());
    assert_eq!(state.capacity(), 1);
    assert_eq!(state.len(), 0);
}

/// Dequeue from an empty larger queue (capacity=10) also returns Empty
/// and preserves all state attributes.
#[test]
fn action_dequeue_transition_empty_larger_queue() {
    let PopTransition::Empty { state } = action_dequeue_transition(empty_state(10)) else {
        panic!("expected Empty, got Popped");
    };
    assert!(state.is_empty());
    assert_eq!(state.capacity(), 10);
    assert_eq!(state.len(), 0);
}

/// Dequeue on empty queue must never panic -- the function returns a PopTransition
/// rather than an Option or Result.
#[test]
fn action_dequeue_transition_empty_does_not_panic() {
    // This assertion is the test itself: if the function panics the test fails.
    let PopTransition::Empty { .. } = action_dequeue_transition(empty_state(1)) else {
        panic!("expected Empty transition");
    };
    let PopTransition::Empty { .. } = action_dequeue_transition(empty_state(1)) else {
        panic!("expected Empty transition on second call");
    };
    if let PopTransition::Empty { .. } = action_dequeue_transition(empty_state(1)) {} else {
        panic!("expected Empty transition on third call");
    }
    // Multiple consecutive dequeues on the same empty state must all return Empty.
}

// ─── Gap 5: action_enqueue_transition with full queue ─────────────────────────
/// Enqueue on a full capacity=1 queue returns QueueFull and preserves the
/// original ticket in the queue.
#[test]
fn action_enqueue_transition_full_capacity_one_preserves_item() {
    let full = state_with_items(1, &[42]);
    let (state, decision) = action_enqueue_transition(full, 99);
    assert_eq!(decision, EnqueueDecision::QueueFull { capacity: 1 });
    assert_eq!(state.len(), 1, "rejected enqueue must not mutate the queue");
    let items: std::collections::VecDeque<u8> = state.into_vec_deque();
    assert_eq!(items.into_iter().collect::<Vec<_>>(), vec![42]);
}

/// Enqueue on a full capacity=2 queue returns QueueFull and preserves both items.
#[test]
fn action_enqueue_transition_full_capacity_two_preserves_both_items() {
    let full = state_with_items(2, &[1, 2]);
    let (state, decision) = action_enqueue_transition(full, 3);
    assert_eq!(decision, EnqueueDecision::QueueFull { capacity: 2 });
    assert_eq!(state.len(), 2);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    assert_eq!(items, vec![1, 2]);
}

/// EnqueueDecision returned by action_enqueue_transition on a full queue
/// carries the correct capacity value (not some other value).
#[test]
fn action_enqueue_transition_full_carries_correct_capacity() {
    let full = state_with_items(7, &[]);
    // Fill it up to capacity 7
    let mut state = full;
    for i in 0..7 {
        let (s, decision) = action_enqueue_transition(state, i);
        assert_eq!(decision, EnqueueDecision::Accepted);
        state = s;
    }
    assert!(state.is_full());
    let (_state, decision) = action_enqueue_transition(state, 200);
    match decision {
        EnqueueDecision::QueueFull { capacity } => assert_eq!(capacity, 7),
        EnqueueDecision::Accepted => panic!("expected rejection on full queue"),
    }
}

// ─── Gap 6: shard_tick_transition with consumed items ─────────────────────────
/// Shard tick on a queue with one item consumes that item and returns ConsumedOne.
#[test]
fn shard_tick_transition_consumes_single_item() {
    let state = state_with_items(2, &[42]);
    match shard_tick_transition(state) {
        ShardTickTransition::ConsumedOne { command, state: s } => {
            assert_eq!(command, 42);
            assert!(s.is_empty());
            assert_eq!(s.len(), 0);
        }
        other => panic!("expected ConsumedOne, got {other:?}"),
    }
}

/// Shard tick on a queue with multiple items consumes only the old front,
/// preserves the tail in order.
#[test]
fn shard_tick_transition_preserves_tail_order() {
    let state = state_with_items(5, &[10, 20, 30]);
    match shard_tick_transition(state) {
        ShardTickTransition::ConsumedOne { command, state: s } => {
            assert_eq!(command, 10);
            assert_eq!(s.len(), 2);
            assert!(!s.is_empty());
            let items: Vec<u8> = s.into_vec_deque().into_iter().collect();
            assert_eq!(items, vec![20, 30]);
        }
        other => panic!("expected ConsumedOne, got {other:?}"),
    }
}

/// Shard tick on an empty queue returns Empty (no command consumed).
#[test]
fn shard_tick_transition_empty_returns_empty() {
    match shard_tick_transition(empty_state(3)) {
        ShardTickTransition::Empty { state } => {
            assert!(state.is_empty());
            assert_eq!(state.capacity(), 3);
            assert_eq!(state.len(), 0);
        }
        other => panic!("expected Empty, got {other:?}"),
    }
}

/// Multiple shard ticks on a filled queue drain items in FIFO order.
#[test]
fn shard_tick_transition_multiple_ticks_fifo_order() {
    let state = state_with_items(4, &[1, 2, 3]);

    // Tick 1: consumes 1
    let ShardTickTransition::ConsumedOne { command: c1, state: s1 } = shard_tick_transition(state)
    else {
        panic!("expected ConsumedOne on tick 1");
    };
    assert_eq!(c1, 1);
    assert_eq!(s1.len(), 2);

    // Tick 2: consumes 2
    let ShardTickTransition::ConsumedOne { command: c2, state: s2 } = shard_tick_transition(s1)
    else {
        panic!("expected ConsumedOne on tick 2");
    };
    assert_eq!(c2, 2);
    assert_eq!(s2.len(), 1);

    // Tick 3: consumes 3
    let ShardTickTransition::ConsumedOne { command: c3, state: s3 } = shard_tick_transition(s2)
    else {
        panic!("expected ConsumedOne on tick 3");
    };
    assert_eq!(c3, 3);
    assert_eq!(s3.len(), 0);

    // Tick 4: queue is empty, returns Empty
    match shard_tick_transition(s3) {
        ShardTickTransition::Empty { .. } => {}
        other => panic!("expected Empty on tick 4, got {other:?}"),
    }
}

// ─── Helper constructors ──────────────────────────────────────────────────────

fn empty_state(capacity: usize) -> QueueState<u8> {
    match QueueState::new(capacity, capacity) {
        Ok(state) => state,
        Err(reason) => panic!("valid test capacity rejected: {reason:?}"),
    }
}

fn state_with_items(capacity: usize, items: &[u8]) -> QueueState<u8> {
    let queue: std::collections::VecDeque<u8> = items.to_vec().into_iter().collect();
    match QueueState::from_vec_deque(capacity, capacity, queue) {
        Ok(state) => state,
        Err(reason) => panic!("valid test queue rejected: {reason:?}"),
    }
}
