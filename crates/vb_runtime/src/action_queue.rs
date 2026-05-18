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

use vb_core::action::ActionTicket;
use std::collections::VecDeque;

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
    backpressure_tx: Option<std::sync::mpsc::Sender<BackpressureWarning>>,
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
    #[must_use]
    pub fn new(capacity: usize) -> Result<Self, ActionQueueError> {
        if capacity == 0 {
            return Err(ActionQueueError::InvalidCapacity);
        }
        Ok(Self {
            inner: std::sync::Mutex::new(Inner { items: VecDeque::new() }),
            capacity,
            backpressure_tx: None,
        })
    }

    /// Creates a new bounded queue with backpressure notification channel.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero.
    #[must_use]
    pub fn with_backpressure(
        capacity: usize,
    ) -> Result<(Self, std::sync::mpsc::Receiver<BackpressureWarning>), ActionQueueError> {
        if capacity == 0 {
            return Err(ActionQueueError::InvalidCapacity);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        Ok((
            Self {
                inner: std::sync::Mutex::new(Inner { items: VecDeque::new() }),
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

        inner.items.push(ticket);

        // Check backpressure threshold: 80% = capacity * 8 / 10
        let depth = inner.items.len();
        let threshold = (self.capacity * 8) / 10;
        if depth >= threshold {
            if let Some(ref tx) = self.backpressure_tx {
                let _ = tx.send(BackpressureWarning {
                    depth,
                    capacity: self.capacity,
                });
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
            seq: SeqNo::new(seq),
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
        assert_eq!(result, Err(ActionQueueError::InvalidCapacity));
    }

    #[test]
    fn bounded_action_queue_enqueue_single_item_succeeds() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        let ticket = make_ticket(0);
        let result = queue.enqueue(ticket);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn bounded_action_queue_enqueue_at_capacity_returns_queue_full_error() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        for i in 0..3 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        let result = queue.enqueue(make_ticket(100));
        assert_eq!(result, Err(ActionQueueError::QueueFull { capacity: 3 }));
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
        queue.dequeue();
        assert_eq!(queue.remaining_capacity(), 7);
    }

    #[test]
    fn bounded_action_queue_invariant_len_plus_remaining_equals_capacity() {
        let queue = BoundedActionCompletionQueue::new(7).unwrap();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
        queue.enqueue(make_ticket(0)).unwrap();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
        queue.dequeue();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
    }
}
