#![forbid(unsafe_code)]
#![cfg(test)]

use vb_core::action::{ActionTicket, ActionError, RunId, SeqNo, StepIdx, ActionId};
use vb_runtime::action::IdempotencyTracker;

fn make_ticket(idempotency_key: u128) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key,
        capacity: 3,
    }
}

#[test]
fn test_tracker_mark_completed_new_key_succeeds() {
    let tracker = IdempotencyTracker::new(10);
    let ticket = make_ticket(100);
    let result = tracker.mark_completed(ticket);
    assert!(result.is_ok(), "mark_completed on new key should succeed");
}

#[test]
fn test_tracker_is_completed_after_mark() {
    let tracker = IdempotencyTracker::new(10);
    let ticket = make_ticket(101);
    tracker.mark_completed(ticket).expect("mark_completed should succeed");
    assert!(tracker.is_completed(&ticket), "is_completed should be true after mark_completed");
}

#[test]
fn test_tracker_mark_completed_duplicate_key_fails() {
    let tracker = IdempotencyTracker::new(10);
    let ticket = make_ticket(102);
    tracker.mark_completed(ticket).expect("first mark_completed should succeed");
    let second = tracker.mark_completed(ticket);
    assert!(second.is_err(), "duplicate mark_completed should fail");
    assert_eq!(second.unwrap_err(), ActionError::CompletionAlreadyRecorded);
}

#[test]
fn test_tracker_at_capacity_evicts_oldest() {
    let capacity = 3;
    let tracker = IdempotencyTracker::new(capacity);
    let ticket0 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 3,
    };
    let ticket1 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(2),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 1,
        capacity: 3,
    };
    let ticket2 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(3),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 2,
        capacity: 3,
    };
    let ticket3 = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(4),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 3,
        capacity: 3,
    };
    tracker.mark_completed(ticket0).expect("ticket0 should succeed");
    tracker.mark_completed(ticket1).expect("ticket1 should succeed");
    tracker.mark_completed(ticket2).expect("ticket2 should succeed");
    assert_eq!(tracker.len(), 3, "tracker should be at capacity");
    tracker.mark_completed(ticket3).expect("ticket3 should succeed after eviction");
    assert!(!tracker.is_completed(&ticket0), "ticket0 should be evicted");
    assert!(tracker.is_completed(&ticket1), "ticket1 should still be present");
    assert!(tracker.is_completed(&ticket2), "ticket2 should still be present");
    assert!(tracker.is_completed(&ticket3), "ticket3 should be present");
}

#[test]
fn test_tracker_eviction_wraps_at_capacity() {
    let capacity = 2;
    let tracker = IdempotencyTracker::new(capacity);
    for i in 0..4 {
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(i + 1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: u128::from(i),
            capacity: 3,
        };
        tracker.mark_completed(ticket).expect("mark should succeed");
    }
    assert_eq!(tracker.len(), 2, "tracker should never exceed capacity");
}

#[test]
fn test_tracker_len_respects_capacity() {
    let capacity = 5;
    let tracker = IdempotencyTracker::new(capacity);
    for i in 0..10 {
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(i + 1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: u128::from(1000 + i),
            capacity: 3,
        };
        let _ = tracker.mark_completed(ticket);
    }
    assert!(tracker.len() <= capacity, "len should never exceed capacity");
}

#[test]
fn test_tracker_is_completed_false_for_unseen_key() {
    let tracker = IdempotencyTracker::new(10);
    let ticket = make_ticket(999);
    assert!(!tracker.is_completed(&ticket), "is_completed should be false for unseen key");
}
