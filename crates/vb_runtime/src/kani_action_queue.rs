#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for BoundedActionCompletionQueue verification.
//!
//! Coverage:
//! - `po-runtime-actionqueue-capacity-kani-01`: BoundedActionCompletionQueue::new capacity bounds
//! - `po-runtime-actionqueue-full-kani-01`: ActionQueue::enqueue returns QueueFull at capacity
//! - `po-runtime-actionqueue-fifo-kani-01`: ActionQueue::dequeue returns tickets in FIFO order

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

use crate::action_queue::{
    ActionQueueError, BoundedActionCompletionQueue, MAX_ACTION_COMPLETION_QUEUE_CAPACITY,
};

/// Constructs a valid ActionTicket for use in harnesses.
///
/// Uses kani::any() for fields to avoid GOD RULE hardcoded-shape violations.
/// All fields are bounded to valid non-zero/non-max values via kani::assume().
fn arbitrary_action_ticket(seq_val: u64) -> ActionTicket {
    let run_val: u64 = kani::any();
    kani::assume(run_val > 0 && run_val != u64::MAX);

    let step_val: u16 = kani::any();
    kani::assume(step_val != u16::MAX);

    let action_val: u16 = kani::any();
    kani::assume(action_val > 0);

    let attempt_val: u16 = kani::any();
    kani::assume(attempt_val > 0);

    ActionTicket {
        run: RunId::new(run_val),
        step: StepIdx::new(step_val),
        seq: SeqNo::new(seq_val),
        action: ActionId::new(action_val),
        attempt: attempt_val,
        idempotency_key: seq_val as u128,
        capacity: 1,
            ..Default::default()
    }
}

/// PO-runtime-actionqueue-capacity-kani-01:
/// BoundedActionCompletionQueue::new rejects capacity=0 and capacity > MAX.
#[kani::proof]
fn kani_action_queue_capacity() {
    let capacity: usize = kani::any();

    let result = BoundedActionCompletionQueue::new(capacity);

    if capacity == 0 {
        // Zero capacity must be rejected with InvalidCapacity::Zero
        kani::assert(matches!(
                result,
                Err(ActionQueueError::InvalidCapacity {
                    reason: crate::action_queue::InvalidActionQueueCapacity::Zero,
                    ..
                }), "assertion failed"),
            "capacity=0 must be rejected with InvalidCapacity::Zero",
        );
    } else if capacity > MAX_ACTION_COMPLETION_QUEUE_CAPACITY {
        // Above-max capacity must be rejected with InvalidCapacity::AboveMaximum
        kani::assert(matches!(
                result,
                Err(ActionQueueError::InvalidCapacity {
                    reason: crate::action_queue::InvalidActionQueueCapacity::AboveMaximum { maximum },
                    ..
                }) if maximum == MAX_ACTION_COMPLETION_QUEUE_CAPACITY, "assertion failed"),
            "capacity above MAX must be rejected with InvalidCapacity::AboveMaximum",
        );
    } else {
        // Valid capacity (1..=MAX) must succeed
        kani::assert(result.is_ok(, "assertion failed"), "valid capacity must be accepted");
        if let Ok(queue) = result {
            kani::assert(queue.capacity(, "assertion failed") == capacity,
                "queue reports the correct capacity",
            );
            kani::assert(queue.is_empty(, "assertion failed"), "new queue is empty");
        }
    }
}

/// PO-runtime-actionqueue-full-kani-01:
/// ActionQueue::enqueue returns QueueFull when queue reaches capacity.
#[kani::proof]
#[kani::unwind(5)]
fn kani_action_queue_full() {
    let capacity: usize = kani::any_where(|c| *c >= 2 && *c <= 16);
    let queue = BoundedActionCompletionQueue::new(capacity);
    kani::assume(queue.is_ok());
    let queue = match queue {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    // Fill the queue to capacity
    let mut i: usize = 0;
    while i < capacity {
        let ticket = arbitrary_action_ticket(i as u64);
        let enqueued = queue.enqueue(ticket);
        kani::assert(enqueued.is_ok(, "assertion failed"), "enqueue until capacity must succeed");
        i += 1;
    }

    // Queue is now full — next enqueue must return QueueFull
    let extra_ticket = arbitrary_action_ticket(u64::MAX);
    let result = queue.enqueue(extra_ticket);
    kani::assert(matches!(
            result,
            Err(ActionQueueError::QueueFull { .. }), "assertion failed"),
        "enqueue at capacity must return QueueFull",
    );

    // len must never exceed capacity
    kani::assert(queue.len(, "assertion failed") <= capacity,
        "queue len must not exceed capacity",
    );
}

/// PO-runtime-actionqueue-fifo-kani-01:
/// ActionQueue::dequeue returns tickets in FIFO order (push_back/pop_front).
#[kani::proof]
#[kani::unwind(6)]
fn kani_action_queue_fifo() {
    let capacity: usize = kani::any_where(|c| *c >= 4 && *c <= 8);
    let queue = BoundedActionCompletionQueue::new(capacity);
    kani::assume(queue.is_ok());
    let queue = match queue {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    // Enqueue 3 tickets with distinct seq values
    let ticket0 = arbitrary_action_ticket(0);
    let ticket1 = arbitrary_action_ticket(1);
    let ticket2 = arbitrary_action_ticket(2);

    match queue.enqueue(ticket0) {
        Ok(v) => { let _ = v; },
        Err(_) => {
            kani::assume(false);
            return;
        }
    }
    match queue.enqueue(ticket1) {
        Ok(v) => { let _ = v; },
        Err(_) => {
            kani::assume(false);
            return;
        }
    }
    match queue.enqueue(ticket2) {
        Ok(v) => { let _ = v; },
        Err(_) => {
            kani::assume(false);
            return;
        }
    }

    // Dequeue must return in FIFO order: 0, then 1, then 2
    let first = queue.dequeue();
    kani::assert(first.is_some(, "assertion failed"), "dequeue from non-empty queue returns Some");
    if let Some(t) = first {
        kani::assert(t.seq.get(, "assertion failed") == 0, "first dequeued ticket has seq=0");
    }

    let second = queue.dequeue();
    kani::assert(second.is_some(, "assertion failed"), "second dequeue returns Some");
    if let Some(t) = second {
        kani::assert(t.seq.get(, "assertion failed") == 1, "second dequeued ticket has seq=1");
    }

    let third = queue.dequeue();
    kani::assert(third.is_some(, "assertion failed"), "third dequeue returns Some");
    if let Some(t) = third {
        kani::assert(t.seq.get(, "assertion failed") == 2, "third dequeued ticket has seq=2");
    }

    let fourth = queue.dequeue();
    kani::assert(fourth.is_none(, "assertion failed"), "dequeue from empty queue returns None");
}
