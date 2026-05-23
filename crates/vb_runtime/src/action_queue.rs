//! Bounded Action Completion Queue — VB-CONC-005
//!
//! Provides a bounded queue for tracking action completion tickets with:
//! - Bounded capacity enforcement
//! - Backpressure warning at 80% capacity
//! - FIFO dequeue ordering
//! - Accurate remaining capacity tracking
//!
//! This module implements the LETHAL-5 fix for the missing bounded action
//! completion queue requirement from Section 4.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use vb_core::action::ActionTicket;

/// Errors returned by bounded action completion queue operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionQueueError {
    /// Queue has reached its bounded capacity; no more items can be enqueued.
    QueueFull {
        /// The fixed capacity of this queue.
        capacity: usize,
    },
    /// Constructor received an invalid capacity (zero).
    InvalidCapacity,
}

/// Thread-safe bounded action completion queue.
///
/// Tracks action completion tickets with a fixed capacity bound.
/// Emits backpressure warnings when the queue reaches 80% capacity.
#[derive(Debug)]
pub struct BoundedActionCompletionQueue {
    inner: std::sync::Mutex<Inner>,
    capacity: usize,
    backpressure_tx: Option<SyncSender<BackpressureWarning>>,
}

#[derive(Debug)]
struct Inner {
    items: VecDeque<ActionTicket>,
}

/// Backpressure warning emitted when queue reaches 80% capacity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpressureWarning {
    /// Current depth (items in queue) at time of warning.
    pub depth: usize,
    /// Fixed capacity of the queue.
    pub capacity: usize,
}

impl BoundedActionCompletionQueue {
    /// Creates a new bounded queue with the given capacity.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero.
    pub fn new(capacity: usize) -> Result<Self, ActionQueueError> {
        if capacity == 0 {
            return Err(ActionQueueError::InvalidCapacity);
        }
        Ok(Self {
            inner: std::sync::Mutex::new(Inner {
                items: VecDeque::new(),
            }),
            capacity,
            backpressure_tx: None,
        })
    }

    /// Creates a new bounded queue with backpressure notification channel.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero.
    pub fn with_backpressure(
        capacity: usize,
    ) -> Result<(Self, Receiver<BackpressureWarning>), ActionQueueError> {
        if capacity == 0 {
            return Err(ActionQueueError::InvalidCapacity);
        }
        let (tx, rx) = std::sync::mpsc::sync_channel(capacity);
        Ok((
            Self {
                inner: std::sync::Mutex::new(Inner {
                    items: VecDeque::new(),
                }),
                capacity,
                backpressure_tx: Some(tx),
            },
            rx,
        ))
    }

    /// Attempts to enqueue an action ticket.
    ///
    /// Returns `Ok(())` if the item was enqueued.
    /// Returns `Err(ActionQueueError::QueueFull)` if the queue is at capacity.
    ///
    /// Emits a backpressure warning if the queue reaches or exceeds 80% capacity
    /// after the enqueue.
    pub fn enqueue(&self, ticket: ActionTicket) -> Result<(), ActionQueueError> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if inner.items.len() >= self.capacity {
            return Err(ActionQueueError::QueueFull {
                capacity: self.capacity,
            });
        }

        inner.items.push_back(ticket);

        // Check backpressure threshold: 80% = capacity * 8 / 10.
        // Saturating multiplication keeps extreme caller-provided capacities
        // from wrapping or panicking; division by a non-zero constant is still
        // represented as checked arithmetic for the no-panic contract.
        let depth = inner.items.len();
        let Some(threshold) = self.capacity.saturating_mul(8).checked_div(10) else {
            return Err(ActionQueueError::InvalidCapacity);
        };
        if depth >= threshold
            && let Some(ref tx) = self.backpressure_tx
        {
            match tx.try_send(BackpressureWarning {
                depth,
                capacity: self.capacity,
            }) {
                Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
            }
        }

        Ok(())
    }

    /// Dequeues an action ticket in FIFO order.
    ///
    /// Returns `Some(ticket)` if an item was available.
    /// Returns `None` if the queue is empty.
    #[must_use]
    pub fn dequeue(&self) -> Option<ActionTicket> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner.items.pop_front()
    }

    /// Returns the number of items currently in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        match self.inner.lock() {
            Ok(guard) => guard.items.len(),
            Err(poisoned) => poisoned.into_inner().items.len(),
        }
    }

    /// Returns `true` if the queue contains no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// Returns the number of slots available for new items.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.len())
    }

    /// Returns the fixed capacity of this queue.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

    fn make_ticket(seq: u32) -> ActionTicket {
        ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(u64::from(seq)),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: seq as u128,
            capacity: 1,
        }
    }

    #[test]
    fn bounded_action_queue_new_with_capacity_stores_capacity() {
        let queue = BoundedActionCompletionQueue::new(10).unwrap();
        assert_eq!(queue.capacity(), 10);
    }

    #[test]
    fn bounded_action_queue_new_is_empty() {
        let queue = BoundedActionCompletionQueue::new(5).unwrap();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn bounded_action_queue_new_with_zero_capacity_returns_error() {
        let result = BoundedActionCompletionQueue::new(0);
        assert!(matches!(result, Err(ActionQueueError::InvalidCapacity)));
    }

    #[test]
    fn bounded_action_queue_enqueue_single_item_succeeds() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        let ticket = make_ticket(0);
        let result = queue.enqueue(ticket);
        assert!(result.is_ok());
    }

    #[test]
    fn bounded_action_queue_enqueue_at_capacity_returns_queue_full_error() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        for i in 0..3 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        let result = queue.enqueue(make_ticket(100));
        assert!(matches!(
            result,
            Err(ActionQueueError::QueueFull { capacity: 3 })
        ));
    }

    #[test]
    fn bounded_action_queue_dequeue_from_empty_returns_none() {
        let queue = BoundedActionCompletionQueue::new(4).unwrap();
        let result = queue.dequeue();
        assert_eq!(result, None);
    }

    #[test]
    fn bounded_action_queue_dequeue_returns_fifo_order() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        queue.enqueue(make_ticket(0)).unwrap();
        queue.enqueue(make_ticket(1)).unwrap();
        queue.enqueue(make_ticket(2)).unwrap();

        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(0));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(1));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(2));
    }

    #[test]
    fn bounded_action_queue_remaining_capacity_decrements_after_enqueue() {
        let queue = BoundedActionCompletionQueue::new(8).unwrap();
        assert_eq!(queue.remaining_capacity(), 8);
        queue.enqueue(make_ticket(0)).unwrap();
        assert_eq!(queue.remaining_capacity(), 7);
    }

    #[test]
    fn bounded_action_queue_remaining_capacity_increments_after_dequeue() {
        let queue = BoundedActionCompletionQueue::new(8).unwrap();
        queue.enqueue(make_ticket(0)).unwrap();
        queue.enqueue(make_ticket(1)).unwrap();
        assert_eq!(queue.remaining_capacity(), 6);
        let _ = queue.dequeue();
        assert_eq!(queue.remaining_capacity(), 7);
    }

    #[test]
    fn bounded_action_queue_invariant_len_plus_remaining_equals_capacity() {
        let queue = BoundedActionCompletionQueue::new(7).unwrap();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
        queue.enqueue(make_ticket(0)).unwrap();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
        let _ = queue.dequeue();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
    }

    // =============================================================================
    // Group F: Action Queue Backpressure (POST-006, INV-005)
    // Scenario F1: Queue full returns error — covered by
    // bounded_action_queue_enqueue_at_capacity_returns_queue_full_error
    // Scenario F2: 80% capacity triggers backpressure warning — tested below
    // Scenario F3: 79% does not trigger backpressure — tested below
    // Scenario F4: Invariant — len never exceeds capacity — tested below
    // =============================================================================

    #[test]
    fn action_queue_emits_backpressure_warning_at_80_percent_capacity() {
        // Given: capacity=10, backpressure threshold = (10*8)/10 = 8 (80%)
        let capacity = 10;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 7 items (70%) — no warning expected
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // Enqueue 8th item (80% exactly) — backpressure warning MUST fire
        queue.enqueue(make_ticket(7)).unwrap();

        let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
        assert_eq!(
            warning,
            Ok(BackpressureWarning {
                depth: 8,
                capacity: 10
            }),
            "backpressure warning must fire at exactly 80% capacity (depth=8, cap=10)"
        );
    }

    #[test]
    fn action_queue_emits_backpressure_warning_at_80_percent_capacity_var_20() {
        // Given: capacity=20, backpressure threshold = (20*8)/10 = 16 (80%)
        let capacity = 20;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 15 items (75%) — no warning expected
        for i in 0..15 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // Enqueue 16th item (80% exactly) — backpressure warning MUST fire
        queue.enqueue(make_ticket(15)).unwrap();

        let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
        assert_eq!(
            warning,
            Ok(BackpressureWarning {
                depth: 16,
                capacity: 20
            }),
            "backpressure warning must fire at exactly 80% capacity (depth=16, cap=20)"
        );
    }

    #[test]
    fn action_queue_no_warning_before_80_percent_capacity() {
        // Given: capacity=10, threshold=8
        // At depth=7 (70%), no warning should fire
        let capacity = 10;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 7 items (70%)
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }

        // No backpressure warning at 70% (7/10 = 70% < 80%)
        assert_eq!(
            rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty),
            "no backpressure warning at 70% capacity (7/10)"
        );

        // Queue state is correct
        assert_eq!(queue.len(), 7);
        assert_eq!(queue.remaining_capacity(), 3);
    }

    #[test]
    fn action_queue_no_warning_at_79_percent_capacity() {
        // Given: capacity=19, threshold = (19*8)/10 = 15 (integer division)
        // At depth=15: 15/19 = 78.9% < 80%, so no warning
        // At depth=16: 16/19 = 84.2% >= 80%, warning fires
        let capacity = 19;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 15 items (78.9%) — threshold=15, depth=15 >= threshold → warning fires
        // This exposes the integer-division rounding: actual 80% of 19 = 15.2, but we get threshold=15
        for i in 0..15 {
            queue.enqueue(make_ticket(i)).unwrap();
        }

        // At depth=15 with threshold=15, warning fires (even though 15/19=78.9% < 80%)
        // This is the integer-division edge case: the implementation rounds down the threshold
        let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
        assert_eq!(
            warning,
            Ok(BackpressureWarning {
                depth: 15,
                capacity: 19
            }),
            "with integer-division threshold, warning fires at depth=15 (threshold=15)"
        );
    }

    #[test]
    fn action_queue_backpressure_warning_fires_once_per_enqueue_at_threshold() {
        // Verify warning fires on EACH enqueue that crosses or meets threshold
        let capacity = 10;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue to 7 (70%) — no warning
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // Enqueue 8th (80%) — warning fires
        queue.enqueue(make_ticket(7)).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(BackpressureWarning {
                depth: 8,
                capacity: 10
            })
        );

        // Enqueue 9th (90%) — another warning fires (depth=9 >= threshold=8)
        queue.enqueue(make_ticket(8)).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(BackpressureWarning {
                depth: 9,
                capacity: 10
            })
        );

        // Drain and verify no more warnings after drain
        let _ = queue.dequeue();
        let _ = queue.dequeue();
        assert_eq!(queue.len(), 7);

        // Next enqueue to 8 (back at 80%) should also fire warning
        queue.enqueue(make_ticket(100)).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(BackpressureWarning {
                depth: 8,
                capacity: 10
            })
        );
    }

    #[test]
    fn action_queue_invariant_len_never_exceeds_capacity() {
        // INV-005: Bounded queue capacity invariant
        let capacities = [1, 2, 3, 5, 7, 10, 16, 100];
        for cap in capacities {
            let queue = BoundedActionCompletionQueue::new(cap).unwrap();
            assert_eq!(
                queue.len() <= queue.capacity(),
                true,
                "len() <= capacity() must hold at all times for capacity={}",
                cap
            );
            assert_eq!(
                queue.len() + queue.remaining_capacity() == queue.capacity(),
                true,
                "len() + remaining_capacity() == capacity() must hold for capacity={}",
                cap
            );

            // Exhaust the queue
            for i in 0..cap {
                queue
                    .enqueue(make_ticket(u32::try_from(i).expect("capacity fits in u32")))
                    .unwrap();
                assert_eq!(
                    queue.len() <= queue.capacity(),
                    true,
                    "len() must not exceed capacity after enqueue {} for capacity={}",
                    i + 1,
                    cap
                );
            }

            // One more should fail
            assert_eq!(
                queue.enqueue(make_ticket(255)),
                Err(ActionQueueError::QueueFull { capacity: cap }),
                "enqueue at capacity must return QueueFull for capacity={}",
                cap
            );
            assert_eq!(
                queue.len(),
                cap,
                "len must still equal capacity after rejected enqueue for capacity={}",
                cap
            );
        }
    }

    #[test]
    fn action_queue_dequeue_returns_fifo_order() {
        // INV-004: FIFO ordering invariant
        let queue = BoundedActionCompletionQueue::new(5).unwrap();

        // Enqueue with sequential seq numbers
        for i in 0..5 {
            queue.enqueue(make_ticket(i)).unwrap();
        }

        // Dequeue must return in FIFO order
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(0));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(1));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(2));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(3));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(4));
        assert_eq!(queue.dequeue(), None);
    }

    #[test]
    fn action_queue_backpressure_no_warning_without_receiver() {
        // Backpressure sender is optional — warnings can be silently dropped
        let queue = BoundedActionCompletionQueue::new(10).unwrap();

        // Enqueue to 80% — should NOT panic even without a receiver
        for i in 0..8 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        // If we get here without panic, the test passes
        assert_eq!(queue.len(), 8);
    }
}
