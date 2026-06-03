//! Type definitions for bounded action completion queue.

use std::collections::VecDeque;
use vb_core::action::ActionTicket;

/// Maximum accepted action completion queue capacity.
pub const MAX_ACTION_COMPLETION_QUEUE_CAPACITY: usize = 65_536;

/// Parsed, non-zero, bounded action completion queue capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionQueueCapacity(pub(crate) usize);

impl ActionQueueCapacity {
    /// Returns the capacity as a primitive for allocation and reporting.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
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

/// Backpressure warning emitted when queue reaches 80% capacity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpressureWarning {
    /// Current depth (items in queue) at time of warning.
    pub depth: usize,
    /// Fixed capacity of the queue.
    pub capacity: usize,
}

/// Thread-safe bounded action completion queue.
///
/// Tracks action completion tickets with a fixed capacity bound.
/// Emits backpressure warnings when the queue reaches 80% capacity.
#[derive(Debug)]
pub struct BoundedActionCompletionQueue {
    pub(crate) inner: std::sync::Mutex<Inner>,
    pub(crate) capacity: ActionQueueCapacity,
    pub(crate) backpressure_tx: Option<std::sync::mpsc::SyncSender<BackpressureWarning>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Inner {
    pub(crate) items: VecDeque<ActionTicket>,
}
