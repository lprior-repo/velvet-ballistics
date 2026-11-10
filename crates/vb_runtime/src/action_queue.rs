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

/// Maximum accepted action completion queue capacity.
pub const MAX_ACTION_COMPLETION_QUEUE_CAPACITY: usize = 65_536;

/// Parsed, non-zero, bounded action completion queue capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionQueueCapacity(usize);

impl ActionQueueCapacity {
    /// Returns the capacity as a primitive for allocation and reporting.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }

    /// Creates a new [`ActionQueueCapacity`] without validating the input.
    ///
    /// Use this constructor ONLY when the caller has already verified that
    /// `1 <= capacity <= MAX_ACTION_COMPLETION_QUEUE_CAPACITY`. The naming
    /// convention `new_unchecked` makes the trust boundary explicit at the
    /// call site. Constructing an `ActionQueueCapacity` with `capacity == 0`
    /// or above the maximum violates the type's invariants and may cause
    /// `BoundedActionCompletionQueue` operations to behave incorrectly.
    ///
    /// For runtime/derived input where the value is not statically known,
    /// use the fallible [`BoundedActionCompletionQueue::new`] and propagate
    /// the error.
    #[must_use]
    pub const fn new_unchecked(capacity: usize) -> Self {
        Self(capacity)
    }
}

/// Reason an action completion queue capacity was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidActionQueueCapacity {
    /// Capacity must be at least one.
    Zero,
    /// Capacity is above the maximum allowed bound.
    AboveMaximum {
        /// Maximum accepted capacity.
        maximum: usize,
    },
}

/// Errors returned by bounded action completion queue operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionQueueError {
    /// Queue has reached its bounded capacity; no more items can be enqueued.
    QueueFull {
        /// The fixed capacity of this queue.
        capacity: ActionQueueCapacity,
    },
    /// Constructor received an invalid capacity.
    InvalidCapacity {
        /// Capacity requested by the caller.
        requested: usize,
        /// Typed rejection reason.
        reason: InvalidActionQueueCapacity,
    },
}

/// Thread-safe bounded action completion queue.
///
/// Tracks action completion tickets with a fixed capacity bound.
/// Emits backpressure warnings when the queue reaches at least 80% capacity.
#[derive(Debug)]
pub struct BoundedActionCompletionQueue {
    inner: std::sync::Mutex<Inner>,
    capacity: ActionQueueCapacity,
    backpressure_tx: Option<SyncSender<BackpressureWarning>>,
}

#[derive(Debug, Clone)]
struct Inner {
    items: VecDeque<ActionTicket>,
}

/// Outcome of attempting to send a backpressure warning notification.
///
/// Returned by
/// [`BoundedActionCompletionQueue::enqueue_with_backpressure_warning_outcome`]
/// so callers can distinguish a delivered warning from a dropped one instead
/// of discarding the `try_send` result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackpressureWarningOutcome {
    /// The warning was delivered to the notification channel.
    Delivered,
    /// No warning was sent: below the backpressure threshold, no channel is
    /// configured, or the enqueue itself failed.
    NotSent,
    /// The notification channel was full; the warning was dropped.
    ChannelFull,
    /// The notification channel receiver was dropped; the warning was dropped.
    ChannelDisconnected,
}

/// Backpressure warning emitted when queue reaches 80% capacity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpressureWarning {
    /// Current depth (items in queue) at time of warning.
    pub depth: usize,
    /// Fixed capacity of the queue.
    pub capacity: usize,
}

include!("action_queue/queue.rs");

include!("action_queue/unit_tests.rs");

#[cfg(test)]
#[path = "action_queue/tests/bounded_queue_tests.rs"]
mod bounded_queue_tests;
