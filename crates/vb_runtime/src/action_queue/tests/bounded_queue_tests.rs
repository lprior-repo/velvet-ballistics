//! Bounded Action Completion Queue — Integration Tests
//!
//! These tests verify the BoundedActionCompletionQueue behavior through
//! its public API. They test all BDD scenarios from the LETHAL-5 test plan.
//!
//! # Test count: 45+ unit-style integration tests
//! # Coverage: enqueue, dequeue, backpressure, capacity tracking, drain, boundaries

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_runtime::action_queue::ActionQueueError;
use vb_runtime::action_queue::BackpressureTryRecvError;
use vb_runtime::action_queue::BoundedActionCompletionQueue;

// =============================================================================
// Test fixtures
// =============================================================================

fn make_ticket(seq: u32) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(seq),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: seq as u128,
        capacity: 1,
    }
}

fn make_ticket_with_action(seq: u32, action: u16) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(seq),
        action: ActionId::new(action),
        attempt: 1,
        idempotency_key: seq as u128,
        capacity: 1,
    }
}

// =============================================================================
// Constructor tests — 7 tests
// =============================================================================

#[test]
fn bounded_action_queue_new_with_capacity_stores_capacity() {
    let queue = BoundedActionCompletionQueue::new(10).unwrap();
    assert_eq!(queue.capacity(), 10);
}

#[test]
fn bounded_action_queue_new_with_small_capacity_stores_capacity() {
    let queue = BoundedActionCompletionQueue::new(1).unwrap();
    assert_eq!(queue.capacity(), 1);
}

#[test]
fn bounded_action_queue_new_with_large_capacity_stores_capacity() {
    let queue = BoundedActionCompletionQueue::new(1024).unwrap();
    assert_eq!(queue.capacity(), 1024);
}

#[test]
fn bounded_action_queue_new_with_zero_capacity_returns_invalid_capacity_error() {
    // BDD Scenario: action_queue_constructor_returns_invalid_capacity_error_when_capacity_is_zero
    // Given: A capacity of 0
    // When: BoundedActionCompletionQueue::new(0).unwrap() is called
    // Then: Returns Err(ActionQueueError::InvalidCapacity)
    let result = BoundedActionCompletionQueue::new(0);
    assert_eq!(result, Err(ActionQueueError::InvalidCapacity));
}

#[test]
fn bounded_action_queue_new_is_empty() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    assert_eq!(queue.len(), 0);
    assert!(queue.is_empty());
}

#[test]
fn bounded_action_queue_new_has_full_remaining_capacity() {
    let queue = BoundedActionCompletionQueue::new(8).unwrap();
    assert_eq!(queue.remaining_capacity(), 8);
}

#[test]
fn bounded_action_queue_new_is_not_full() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    assert!(!queue.is_full());
}

// =============================================================================
// Enqueue success tests — 5 tests
// =============================================================================

#[test]
fn bounded_action_queue_enqueue_single_item_succeeds() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    let ticket = make_ticket(0);
    let result = queue.enqueue(ticket);
    assert_eq!(result, Ok(()));
}

#[test]
fn bounded_action_queue_enqueue_increments_len() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    assert_eq!(queue.len(), 0);
    queue.enqueue(make_ticket(0)).unwrap();
    assert_eq!(queue.len(), 1);
    queue.enqueue(make_ticket(1)).unwrap();
    assert_eq!(queue.len(), 2);
}

#[test]
fn bounded_action_queue_enqueue_below_capacity_succeeds() {
    let queue = BoundedActionCompletionQueue::new(4).unwrap();
    // Fill to 2 (below capacity of 4)
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    assert_eq!(queue.len(), 2);
    // Enqueue one more — still below capacity
    let result = queue.enqueue(make_ticket(2));
    assert_eq!(result, Ok(()));
    assert_eq!(queue.len(), 3);
}

#[test]
fn bounded_action_queue_enqueue_at_exactly_one_below_capacity_succeeds() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    // Fill to capacity - 1 = 4
    for i in 0..4 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    assert_eq!(queue.len(), 4);
    // Enqueue 5th item — exactly at capacity
    let result = queue.enqueue(make_ticket(5));
    assert_eq!(result, Ok(()));
    assert_eq!(queue.len(), 5);
}

#[test]
fn bounded_action_queue_len_increments_per_enqueue() {
    let queue = BoundedActionCompletionQueue::new(10).unwrap();
    for i in 0..3 {
        assert_eq!(queue.len(), i);
        queue.enqueue(make_ticket(i)).unwrap();
        assert_eq!(queue.len(), i + 1);
    }
}

// =============================================================================
// Enqueue failure tests — QueueFull — 4 tests
// =============================================================================

#[test]
fn bounded_action_queue_returns_queue_full_error_when_enqueue_at_capacity() {
    // BDD Scenario: action_queue_returns_queue_full_error_when_enqueue_at_capacity
    // Given: A bounded action completion queue with capacity N = 3; 3 actions already enqueued
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    for i in 0..3 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    assert_eq!(queue.len(), 3);
    // When: A 4th action completion is enqueued
    let result = queue.enqueue(make_ticket(100));
    // Then: Returns Err(ActionQueueError::QueueFull { capacity: 3 })
    assert_eq!(result, Err(ActionQueueError::QueueFull { capacity: 3 }));
    // And: Queue length remains 3
    assert_eq!(queue.len(), 3);
}

#[test]
fn bounded_action_queue_returns_queue_full_error_when_enqueue_single_element_at_capacity_one() {
    // BDD Scenario: action_queue_returns_queue_full_error_when_enqueue_single_element_at_capacity_one
    // Given: A bounded action completion queue with capacity N = 1; 1 action already enqueued
    let queue = BoundedActionCompletionQueue::new(1).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    assert_eq!(queue.len(), 1);
    // When: A 2nd action completion is enqueued
    let result = queue.enqueue(make_ticket(1));
    // Then: Returns Err(ActionQueueError::QueueFull { capacity: 1 })
    assert_eq!(result, Err(ActionQueueError::QueueFull { capacity: 1 }));
}

#[test]
fn bounded_action_queue_enqueue_past_capacity_returns_queue_full_error() {
    let queue = BoundedActionCompletionQueue::new(2).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    let result = queue.enqueue(make_ticket(2));
    assert_eq!(result, Err(ActionQueueError::QueueFull { capacity: 2 }));
    assert_eq!(queue.len(), 2);
}

#[test]
fn bounded_action_queue_queue_full_error_contains_correct_capacity() {
    let queue = BoundedActionCompletionQueue::new(7).unwrap();
    for i in 0..7 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    let result = queue.enqueue(make_ticket(99));
    match result {
        Err(ActionQueueError::QueueFull { capacity }) => {
            assert_eq!(capacity, 7);
        }
        other => panic!("expected QueueFull {{ capacity: 7 }}, got {:?}", other),
    }
}

// =============================================================================
// Dequeue tests — 6 tests
// =============================================================================

#[test]
fn bounded_action_queue_dequeue_returns_none_when_empty() {
    // BDD Scenario: action_queue_dequeue_returns_none_when_empty
    // Given: A bounded action completion queue with capacity N = 4; queue is empty
    let queue = BoundedActionCompletionQueue::new(4).unwrap();
    // When: A dequeue is performed
    let result = queue.dequeue();
    // Then: Returns None
    assert_eq!(result, None);
}

#[test]
fn bounded_action_queue_dequeue_returns_fifo_order() {
    // BDD Scenario: action_queue_dequeue_returns_items_in_fifo_order
    // Given: A bounded action completion queue with capacity N = 3; actions A, B, C enqueued
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    queue.enqueue(make_ticket_with_action(0, 10)).unwrap();
    queue.enqueue(make_ticket_with_action(1, 20)).unwrap();
    queue.enqueue(make_ticket_with_action(2, 30)).unwrap();
    // When: Three dequeue operations are performed
    let first = queue.dequeue();
    let second = queue.dequeue();
    let third = queue.dequeue();
    // Then: First dequeue returns A, Second dequeue returns B, Third dequeue returns C
    assert_eq!(first.map(|t| t.seq.get()), Some(0));
    assert_eq!(second.map(|t| t.seq.get()), Some(1));
    assert_eq!(third.map(|t| t.seq.get()), Some(2));
}

#[test]
fn bounded_action_queue_dequeue_decrements_len() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    assert_eq!(queue.len(), 2);

    queue.dequeue();
    assert_eq!(queue.len(), 1);

    queue.dequeue();
    assert_eq!(queue.len(), 0);
}

#[test]
fn bounded_action_queue_dequeue_from_full_queue_returns_item() {
    let queue = BoundedActionCompletionQueue::new(2).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    assert!(queue.is_full());

    let result = queue.dequeue();
    assert!(result.is_some());
    assert_eq!(result.unwrap().seq.get(), 0);
}

#[test]
fn bounded_action_queue_dequeue_allows_re_enqueue() {
    let queue = BoundedActionCompletionQueue::new(2).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.dequeue().unwrap();
    assert_eq!(queue.len(), 0);

    queue.enqueue(make_ticket(1)).unwrap();
    assert_eq!(queue.len(), 1);

    let result = queue.dequeue();
    assert!(result.is_some());
    assert_eq!(result.unwrap().seq.get(), 1);
}

#[test]
fn bounded_action_queue_dequeue_from_empty_returns_none() {
    let queue = BoundedActionCompletionQueue::new(4).unwrap();
    let result = queue.dequeue();
    assert_eq!(result, None);
}

// =============================================================================
// is_empty / is_full tests — 7 tests
// =============================================================================

#[test]
fn bounded_action_queue_is_empty_true_when_new() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    assert!(queue.is_empty());
}

#[test]
fn bounded_action_queue_is_empty_false_after_enqueue() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    assert!(!queue.is_empty());
}

#[test]
fn bounded_action_queue_is_empty_true_after_drain() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    queue.dequeue();
    queue.dequeue();
    assert!(queue.is_empty());
}

#[test]
fn bounded_action_queue_is_full_false_when_empty() {
    let queue = BoundedActionCompletionQueue::new(4).unwrap();
    assert!(!queue.is_full());
}

#[test]
fn bounded_action_queue_is_full_true_when_at_capacity() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    queue.enqueue(make_ticket(2)).unwrap();
    assert!(queue.is_full());
}

#[test]
fn bounded_action_queue_is_full_false_when_below_capacity() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    assert!(!queue.is_full());
}

#[test]
fn bounded_action_queue_capacity_one_is_full_after_single_enqueue() {
    let queue = BoundedActionCompletionQueue::new(1).unwrap();
    assert!(!queue.is_full());
    queue.enqueue(make_ticket(0)).unwrap();
    assert!(queue.is_full());
}

// =============================================================================
// remaining_capacity tests — 6 tests
// =============================================================================

#[test]
fn bounded_action_queue_remaining_capacity_equals_capacity_when_empty() {
    // BDD Scenario: action_queue_remaining_capacity_equals_capacity_when_empty
    // Given: A bounded action completion queue with capacity N = 16; queue is empty
    let queue = BoundedActionCompletionQueue::new(16).unwrap();
    // When: remaining_capacity() is called
    // Then: Returns 16
    assert_eq!(queue.remaining_capacity(), 16);
}

#[test]
fn bounded_action_queue_remaining_capacity_is_zero_when_full() {
    // BDD Scenario: action_queue_remaining_capacity_is_zero_when_full
    // Given: A bounded action completion queue with capacity N = 4; 4 actions enqueued
    let queue = BoundedActionCompletionQueue::new(4).unwrap();
    for i in 0..4 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    // When: remaining_capacity() is called
    // Then: Returns 0
    assert_eq!(queue.remaining_capacity(), 0);
}

#[test]
fn bounded_action_queue_remaining_capacity_decrements_after_enqueue() {
    // BDD Scenario: action_queue_remaining_capacity_decrements_after_enqueue
    // Given: A bounded action completion queue with capacity N = 8; queue is empty
    let queue = BoundedActionCompletionQueue::new(8).unwrap();
    // When: 3 action completions are enqueued
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    queue.enqueue(make_ticket(2)).unwrap();
    // Then: remaining_capacity() returns 5
    assert_eq!(queue.remaining_capacity(), 5);
}

#[test]
fn bounded_action_queue_remaining_capacity_increments_after_dequeue() {
    // BDD Scenario: action_queue_remaining_capacity_increments_after_dequeue
    // Given: A bounded action completion queue with capacity N = 8; 3 actions enqueued
    let queue = BoundedActionCompletionQueue::new(8).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    queue.enqueue(make_ticket(2)).unwrap();
    // When: 1 action completion is dequeued
    queue.dequeue();
    // Then: remaining_capacity() returns 6
    assert_eq!(queue.remaining_capacity(), 6);
}

#[test]
fn bounded_action_queue_remaining_capacity_partial_fills_correctly() {
    let queue = BoundedActionCompletionQueue::new(10).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    queue.enqueue(make_ticket(2)).unwrap();
    // 3 enqueued, 7 remaining
    assert_eq!(queue.remaining_capacity(), 7);
}

#[test]
fn bounded_action_queue_remaining_capacity_after_drain_equals_capacity() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    queue.dequeue();
    queue.dequeue();
    assert_eq!(queue.remaining_capacity(), 5);
}

// =============================================================================
// Backpressure warning tests — 6 tests
// =============================================================================

#[test]
fn bounded_action_queue_emits_backpressure_warning_at_80_percent_capacity() {
    // BDD Scenario: action_queue_emits_backpressure_warning_at_80_percent_capacity
    // Given: A bounded action completion queue with capacity N = 10; 7 actions enqueued (70%)
    let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(10).unwrap();
    for i in 0..7 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    // When: An 8th action completion is enqueued (reaching exactly 80%)
    queue.enqueue(make_ticket(7)).unwrap();
    // Then: Returns Ok(())
    // And: A backpressure warning notification is emitted
    let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
    assert!(warning.is_ok());
    let w = warning.unwrap();
    assert_eq!(w.depth, 8);
    assert_eq!(w.capacity, 10);
}

#[test]
fn bounded_action_queue_emits_backpressure_warning_just_above_80_percent() {
    // BDD Scenario: action_queue_emits_backpressure_warning_just_above_80_percent
    // Given: A bounded action completion queue with capacity N = 10; 8 actions enqueued (80%)
    let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(10).unwrap();
    for i in 0..8 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    // Drain the channel
    let drained_warning = rx.try_recv();
    assert!(drained_warning.is_ok());
    // When: A 9th action completion is enqueued (90%)
    queue.enqueue(make_ticket(8)).unwrap();
    // Then: Returns Ok(())
    // And: A backpressure warning notification is emitted
    let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
    assert!(warning.is_ok());
    let w = warning.unwrap();
    assert_eq!(w.depth, 9);
    assert_eq!(w.capacity, 10);
}

#[test]
fn bounded_action_queue_does_not_emit_warning_below_80_percent() {
    // BDD Scenario: action_queue_does_not_emit_warning_below_80_percent
    // Given: A bounded action completion queue with capacity N = 10; 7 actions enqueued (70%)
    let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(10).unwrap();
    for i in 0..7 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    // When: An 8th action completion is enqueued (80%)
    queue.enqueue(make_ticket(7)).unwrap();
    // Then: No backpressure warning is emitted
    // Note: At exactly 80%, warning IS emitted (per spec)
    let threshold_warning = rx.try_recv();
    assert!(threshold_warning.is_ok());
    // Actually 8/10 = 80% should emit - let's verify below threshold first
    // For 70% case (7 items), try_recv should be empty
    let (queue2, rx2) = BoundedActionCompletionQueue::with_backpressure(10).unwrap();
    for i in 0..7 {
        queue2.enqueue(make_ticket(i)).unwrap();
    }
    let result2 = rx2.try_recv();
    assert_eq!(result2, Err(BackpressureTryRecvError::Empty));
}

#[test]
fn bounded_action_queue_backpressure_warning_contains_depth_and_capacity() {
    // BDD Scenario: action_queue_backpressure_warning_contains_depth_and_capacity
    // Given: A bounded action completion queue with capacity N = 5; 4 actions enqueued (80%)
    let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(5).unwrap();
    for i in 0..4 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    // When: A 5th action completion is enqueued (100%)
    queue.enqueue(make_ticket(4)).unwrap();
    // Then: The backpressure warning contains depth (4) and capacity (5)
    let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
    assert!(warning.is_ok());
    let w = warning.unwrap();
    assert_eq!(w.depth, 4); // depth before the enqueue that triggered
    assert_eq!(w.capacity, 5);
}

#[test]
fn bounded_action_queue_no_backpressure_when_empty() {
    let (_queue, rx) = BoundedActionCompletionQueue::with_backpressure(10).unwrap();
    let result = rx.try_recv();
    assert_eq!(result, Err(BackpressureTryRecvError::Empty));
}

#[test]
fn bounded_action_queue_backpressure_threshold_is_exclusive() {
    // For capacity 5, threshold = 4 (80%)
    let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(5).unwrap();
    // At 3 out of 5 = 60%, no warning
    for i in 0..3 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    let warning_before_threshold = rx.try_recv();
    assert_eq!(
        warning_before_threshold,
        Err(BackpressureTryRecvError::Empty)
    );

    // At 4 out of 5 = 80%, warning fires
    queue.enqueue(make_ticket(3)).unwrap();
    let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
    assert!(warning.is_ok());
}

// =============================================================================
// Drain / empty queue tests — 4 tests
// =============================================================================

#[test]
fn bounded_action_queue_len_is_zero_after_draining_all_items() {
    // BDD Scenario: action_queue_len_is_zero_after_draining_all_items
    // Given: A bounded action completion queue with capacity N = 5; 5 actions enqueued
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    for i in 0..5 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    // When: All 5 action completions are dequeued in FIFO order
    queue.dequeue();
    queue.dequeue();
    queue.dequeue();
    queue.dequeue();
    queue.dequeue();
    // Then: Queue length is 0
    assert_eq!(queue.len(), 0);
    // And: is_empty() returns true
    assert!(queue.is_empty());
    // And: remaining_capacity() returns 5
    assert_eq!(queue.remaining_capacity(), 5);
}

#[test]
fn bounded_action_queue_dequeue_returns_none_after_empty() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.dequeue();
    let result = queue.dequeue();
    assert_eq!(result, None);
}

#[test]
fn bounded_action_queue_multiple_enqueue_dequeue_cycles() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();

    // Cycle 1
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(0));
    assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(1));

    // Cycle 2
    queue.enqueue(make_ticket(2)).unwrap();
    queue.enqueue(make_ticket(3)).unwrap();
    assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(2));
    assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(3));

    assert!(queue.is_empty());
}

#[test]
fn bounded_action_queue_remaining_capacity_equals_capacity_after_drain() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    for i in 0..5 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    while queue.dequeue().is_some() {}
    assert_eq!(queue.remaining_capacity(), 5);
}

// =============================================================================
// Boundary: capacity=1 edge case — 4 tests
// =============================================================================

#[test]
fn bounded_action_queue_capacity_one_enqueue_first_succeeds() {
    let queue = BoundedActionCompletionQueue::new(1).unwrap();
    let result = queue.enqueue(make_ticket(0));
    assert_eq!(result, Ok(()));
    assert_eq!(queue.len(), 1);
    assert!(queue.is_full());
}

#[test]
fn bounded_action_queue_capacity_one_enqueue_second_fails() {
    let queue = BoundedActionCompletionQueue::new(1).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    let result = queue.enqueue(make_ticket(1));
    assert_eq!(result, Err(ActionQueueError::QueueFull { capacity: 1 }));
}

#[test]
fn bounded_action_queue_capacity_one_dequeue_returns_item() {
    let queue = BoundedActionCompletionQueue::new(1).unwrap();
    queue.enqueue(make_ticket(42)).unwrap();
    let result = queue.dequeue();
    assert!(result.is_some());
    assert_eq!(result.unwrap().seq.get(), 42);
}

#[test]
fn bounded_action_queue_capacity_one_dequeue_then_requeue() {
    let queue = BoundedActionCompletionQueue::new(1).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.dequeue().unwrap();
    assert!(queue.is_empty());
    assert!(!queue.is_full());

    queue.enqueue(make_ticket(1)).unwrap();
    assert!(queue.is_full());
    assert_eq!(queue.len(), 1);
}

// =============================================================================
// Boundary: large capacity tests — 2 tests
// =============================================================================

#[test]
fn bounded_action_queue_large_capacity_tracks_correctly() {
    let queue = BoundedActionCompletionQueue::new(1000).unwrap();
    assert_eq!(queue.capacity(), 1000);
    assert_eq!(queue.remaining_capacity(), 1000);

    for i in 0..500 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    assert_eq!(queue.len(), 500);
    assert_eq!(queue.remaining_capacity(), 500);
}

#[test]
fn bounded_action_queue_large_capacity_backpressure_fires_at_80_percent() {
    let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(100).unwrap();
    // Fill to 79% — below threshold
    for i in 0..79 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    let warning_before_threshold = rx.try_recv();
    assert_eq!(
        warning_before_threshold,
        Err(BackpressureTryRecvError::Empty)
    );

    // 80% = 80 items
    queue.enqueue(make_ticket(79)).unwrap();
    let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
    assert!(warning.is_ok());
}

// =============================================================================
// Invariant tests — 5 tests
// =============================================================================

#[test]
fn bounded_action_queue_invariant_len_plus_remaining_equals_capacity_new() {
    let queue = BoundedActionCompletionQueue::new(7).unwrap();
    assert_eq!(queue.len() + queue.remaining_capacity(), 7);
}

#[test]
fn bounded_action_queue_invariant_len_plus_remaining_equals_capacity_after_enqueue() {
    let queue = BoundedActionCompletionQueue::new(7).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    assert_eq!(queue.len() + queue.remaining_capacity(), 7);
    queue.enqueue(make_ticket(1)).unwrap();
    assert_eq!(queue.len() + queue.remaining_capacity(), 7);
}

#[test]
fn bounded_action_queue_invariant_len_plus_remaining_equals_capacity_after_dequeue() {
    let queue = BoundedActionCompletionQueue::new(7).unwrap();
    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    queue.dequeue();
    assert_eq!(queue.len() + queue.remaining_capacity(), 7);
}

#[test]
fn bounded_action_queue_invariant_len_plus_remaining_equals_capacity_full() {
    let queue = BoundedActionCompletionQueue::new(7).unwrap();
    for i in 0..7 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    assert_eq!(queue.len() + queue.remaining_capacity(), 7);
}

#[test]
fn bounded_action_queue_invariant_len_plus_remaining_equals_capacity_after_drain() {
    let queue = BoundedActionCompletionQueue::new(7).unwrap();
    for i in 0..7 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    while queue.dequeue().is_some() {}
    assert_eq!(queue.len() + queue.remaining_capacity(), 7);
}

// =============================================================================
// Error equality tests — 3 tests
// =============================================================================

#[test]
fn action_queue_error_queue_full_eq_when_same_capacity() {
    let e1 = ActionQueueError::QueueFull { capacity: 5 };
    let e2 = ActionQueueError::QueueFull { capacity: 5 };
    assert_eq!(e1, e2);
}

#[test]
fn action_queue_error_queue_full_neq_when_different_capacity() {
    let e1 = ActionQueueError::QueueFull { capacity: 5 };
    let e2 = ActionQueueError::QueueFull { capacity: 7 };
    assert_ne!(e1, e2);
}

#[test]
fn action_queue_error_queue_full_debug_contains_capacity() {
    let e = ActionQueueError::QueueFull { capacity: 3 };
    let debug = format!("{:?}", e);
    assert!(debug.contains("3"));
    assert!(debug.contains("QueueFull"));
}

// =============================================================================
// enqueue / dequeue round-trip tests — 3 tests
// =============================================================================

#[test]
fn bounded_action_queue_enqueue_dequeue_roundtrip_single_item() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    let ticket = make_ticket(123);
    queue.enqueue(ticket).unwrap();

    let dequeued = queue.dequeue();
    assert!(dequeued.is_some());
    assert_eq!(dequeued.unwrap().seq.get(), 123);
}

#[test]
fn bounded_action_queue_enqueue_dequeue_roundtrip_multiple_items() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();
    for i in 0..5 {
        queue.enqueue(make_ticket(i)).unwrap();
    }

    for i in 0..5 {
        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().seq.get(), i as u32);
    }

    assert!(queue.is_empty());
}

#[test]
fn bounded_action_queue_enqueue_dequeue_preserves_all_tickets() {
    let queue = BoundedActionCompletionQueue::new(10).unwrap();
    let tickets: Vec<_> = (0..10).map(make_ticket).collect();

    for t in &tickets {
        queue.enqueue(t.clone()).unwrap();
    }

    let mut dequeued = Vec::new();
    while let Some(t) = queue.dequeue() {
        dequeued.push(t);
    }

    assert_eq!(dequeued.len(), 10);
    for (original, roundtrip) in tickets.iter().zip(dequeued.iter()) {
        assert_eq!(original.seq, roundtrip.seq);
        assert_eq!(original.action, roundtrip.action);
    }
}

// =============================================================================
// Stress / many items tests — 2 tests
// =============================================================================

#[test]
fn bounded_action_queue_many_items_at_capacity() {
    let queue = BoundedActionCompletionQueue::new(100).unwrap();
    for i in 0..100 {
        queue.enqueue(make_ticket(i)).unwrap();
    }
    assert!(queue.is_full());
    assert_eq!(queue.len(), 100);
    assert_eq!(queue.remaining_capacity(), 0);

    // Verify no more can be added
    let result = queue.enqueue(make_ticket(999));
    assert_eq!(result, Err(ActionQueueError::QueueFull { capacity: 100 }));
}

#[test]
fn bounded_action_queue_interleaved_enqueue_dequeue() {
    let queue = BoundedActionCompletionQueue::new(5).unwrap();

    queue.enqueue(make_ticket(0)).unwrap();
    queue.enqueue(make_ticket(1)).unwrap();
    assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(0));

    queue.enqueue(make_ticket(2)).unwrap();
    queue.enqueue(make_ticket(3)).unwrap();
    assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(1));
    assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(2));

    queue.enqueue(make_ticket(4)).unwrap();
    queue.enqueue(make_ticket(5)).unwrap();
    assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(3));

    // At capacity now
    assert!(queue.is_full());
    queue.enqueue(make_ticket(6)).unwrap_err(); // Should fail
}
