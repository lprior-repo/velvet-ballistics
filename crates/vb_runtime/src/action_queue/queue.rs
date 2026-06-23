//! Queue implementation for bounded action completion queue.

use std::sync::Arc;

use crossbeam_queue::ArrayQueue;
use vb_core::action::ActionTicket;

use super::types::{
    ActionQueueCapacity, ActionQueueError, BackpressureReceiver, BackpressureSender,
    BackpressureWarning, BoundedActionCompletionQueue, MAX_ACTION_COMPLETION_QUEUE_CAPACITY,
};

impl BoundedActionCompletionQueue {
    /// Creates a new bounded queue with the given capacity.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero
    /// or exceeds [`MAX_ACTION_COMPLETION_QUEUE_CAPACITY`].
    pub fn new(capacity: usize) -> Result<Self, ActionQueueError> {
        let capacity = parse_capacity(capacity)?;
        Ok(Self {
            inner: ArrayQueue::new(capacity.get()),
            capacity,
            backpressure_tx: None,
        })
    }

    /// Creates a new bounded queue with backpressure notification channel.
    ///
    /// The backpressure channel is a lock-free bounded MPMC ring buffer
    /// (`crossbeam_queue::ArrayQueue<BackpressureWarning>`) shared between
    /// the queue and the returned [`BackpressureReceiver`]. This satisfies
    /// master spec §50 (lock-free MPMC primitives on the runtime hot path)
    /// and the Holzman-Rust rule against `std::sync::mpsc` use.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero
    /// or exceeds [`MAX_ACTION_COMPLETION_QUEUE_CAPACITY`].
    pub fn with_backpressure(
        capacity: usize,
    ) -> Result<(Self, BackpressureReceiver), ActionQueueError> {
        let capacity = parse_capacity(capacity)?;
        let bp_queue: Arc<ArrayQueue<BackpressureWarning>> =
            Arc::new(ArrayQueue::new(capacity.get()));
        let tx = BackpressureSender {
            queue: Arc::clone(&bp_queue),
        };
        let rx = BackpressureReceiver { queue: bp_queue };
        Ok((
            Self {
                inner: ArrayQueue::new(capacity.get()),
                capacity,
                backpressure_tx: Some(tx),
            },
            rx,
        ))
    }

    /// Pushes a ticket into the queue.
    ///
    /// Returns `Ok(())` if the item was enqueued.
    /// Returns `Err(ActionQueueError::QueueFull)` if the queue is at capacity.
    ///
    /// Emits a backpressure warning if the queue reaches or exceeds 80% capacity
    /// after the enqueue. The warning is non-blocking: a full backpressure
    /// channel silently drops the new warning so producer enqueue is never
    /// stalled by a slow consumer.
    pub fn enqueue(&self, ticket: ActionTicket) -> Result<(), ActionQueueError> {
        self.inner
            .push(ticket)
            .map_err(|_| ActionQueueError::QueueFull {
                capacity: self.capacity,
            })?;
        let depth = self.inner.len();
        let threshold = backpressure_threshold(self.capacity);
        if depth >= threshold
            && let Some(ref tx) = self.backpressure_tx
        {
            let warning = BackpressureWarning {
                depth,
                capacity: self.capacity.get(),
            };
            // The warning channel is best-effort by contract: enqueue must not
            // stall when the backpressure receiver falls behind. The send
            // result is still observed explicitly so fallible status is not
            // silently discarded.
            let _warning_was_dropped = tx.try_send(warning).is_err();
        }
        Ok(())
    }

    /// Dequeues an action ticket in FIFO order.
    ///
    /// Returns `Some(ticket)` if an item was available.
    /// Returns `None` if the queue is empty.
    #[must_use]
    pub fn dequeue(&self) -> Option<ActionTicket> {
        self.inner.pop()
    }

    /// Returns the number of items currently in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the queue contains no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// Returns the number of slots available for new items.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.get().saturating_sub(self.inner.len())
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
    let cap = capacity.get();
    match cap.checked_mul(8) {
        Some(scaled) => {
            // ceil(scaled / 10) = (scaled + 9) / 10, fully checked. The doc
            // contract is "80 percent capacity"; ceiling reaches the documented
            // threshold for capacities that are not multiples of 5 (e.g. 9 -> 8
            // rather than floor -> 7).
            match scaled
                .checked_add(9)
                .and_then(|biased| biased.checked_div(10))
            {
                Some(threshold) => threshold.max(1),
                // Unreachable for any capacity where checked_mul(8) succeeded
                // (adding 9 cannot overflow usize for a realistic queue), but
                // fail safe by falling back to the raw capacity.
                None => cap,
            }
        }
        None => cap,
    }
}

#[cfg(test)]
mod tests {
    use super::backpressure_threshold;
    use super::types::ActionQueueCapacity;

    fn capacity(value: usize) -> ActionQueueCapacity {
        // The tuple field is pub(crate), and these tests run in-crate. The
        // production constructor (`parse_capacity`) rejects 0 and oversized
        // values; the values used here are all valid.
        ActionQueueCapacity(value)
    }

    // RP-019 regression: backpressure_threshold must reach the documented 80%
    // mark via ceiling division, not floor. The pre-fix floor implementation
    // produced threshold=7 for capacity=9 (only 77.7%), below the documented
    // 80% contract.
    #[test]
    fn backpressure_threshold_uses_ceiling_to_reach_80_percent() {
        // capacity 9: ceil(9*8/10) = ceil(7.2) = 8 (was 7 with floor)
        assert_eq!(backpressure_threshold(capacity(9)), 8);
        // capacity 2: ceil(2*8/10) = ceil(1.6) = 2 (was 1 with floor)
        assert_eq!(backpressure_threshold(capacity(2)), 2);
        // capacity 5: ceil(5*8/10) = ceil(4.0) = 4 (unchanged)
        assert_eq!(backpressure_threshold(capacity(5)), 4);
        // capacity 10: ceil(10*8/10) = ceil(8.0) = 8 (unchanged)
        assert_eq!(backpressure_threshold(capacity(10)), 8);
    }

    // The minimum threshold of 1 must still hold for capacity=1 (ceil(0.8)=1
    // anyway, but the .max(1) guard keeps capacity=1 stable).
    #[test]
    fn backpressure_threshold_minimum_is_one() {
        assert_eq!(backpressure_threshold(capacity(1)), 1);
    }
}
