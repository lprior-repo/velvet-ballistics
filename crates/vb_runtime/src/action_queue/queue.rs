//! Queue implementation for bounded action completion queue.

use std::sync::mpsc::{Receiver, TrySendError};
use vb_core::action::ActionTicket;

use super::types::{
    ActionQueueCapacity, ActionQueueError, BackpressureWarning, BoundedActionCompletionQueue,
    Inner, MAX_ACTION_COMPLETION_QUEUE_CAPACITY,
};

impl BoundedActionCompletionQueue {
    /// Creates a new bounded queue with the given capacity.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero
    /// or exceeds [`MAX_ACTION_COMPLETION_QUEUE_CAPACITY`].
    pub fn new(capacity: usize) -> Result<Self, ActionQueueError> {
        let capacity = parse_capacity(capacity)?;
        Ok(Self {
            inner: std::sync::Mutex::new(Inner {
                items: std::collections::VecDeque::with_capacity(capacity.get()),
            }),
            capacity,
            backpressure_tx: None,
        })
    }

    /// Creates a new bounded queue with backpressure notification channel.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero
    /// or exceeds [`MAX_ACTION_COMPLETION_QUEUE_CAPACITY`].
    pub fn with_backpressure(
        capacity: usize,
    ) -> Result<(Self, Receiver<BackpressureWarning>), ActionQueueError> {
        let capacity = parse_capacity(capacity)?;
        let (tx, rx) = std::sync::mpsc::sync_channel(capacity.get());
        Ok((
            Self {
                inner: std::sync::Mutex::new(Inner {
                    items: std::collections::VecDeque::with_capacity(capacity.get()),
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
        if inner.items.len() >= self.capacity.get() {
            return Err(ActionQueueError::QueueFull {
                capacity: self.capacity,
            });
        }

        inner.items.push_back(ticket);

        let depth = inner.items.len();
        let threshold = backpressure_threshold(self.capacity);
        if depth >= threshold
            && let Some(ref tx) = self.backpressure_tx
        {
            match tx.try_send(BackpressureWarning {
                depth,
                capacity: self.capacity.get(),
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
        self.len() >= self.capacity.get()
    }

    /// Returns the number of slots available for new items.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.get().saturating_sub(self.len())
    }

    /// Returns the fixed capacity of this queue.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }
}

fn parse_capacity(capacity: usize) -> Result<ActionQueueCapacity, ActionQueueError> {
    if capacity == 0 {
        return Err(ActionQueueError::InvalidCapacity {
            requested: capacity,
            reason: super::types::InvalidActionQueueCapacity::Zero,
        });
    }

    if capacity > MAX_ACTION_COMPLETION_QUEUE_CAPACITY {
        return Err(ActionQueueError::InvalidCapacity {
            requested: capacity,
            reason: super::types::InvalidActionQueueCapacity::AboveMaximum {
                maximum: MAX_ACTION_COMPLETION_QUEUE_CAPACITY,
            },
        });
    }

    Ok(ActionQueueCapacity(capacity))
}

fn backpressure_threshold(capacity: ActionQueueCapacity) -> usize {
    match capacity
        .get()
        .checked_mul(8)
        .and_then(|scaled| scaled.checked_div(10))
    {
        Some(threshold) => threshold.max(1),
        None => capacity.get(),
    }
}
