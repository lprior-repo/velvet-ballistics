//! QueueState domain aggregate and decision types.
//!
//! Defines the core queue-state invariant, import/rejection paths, and the
//! decision enums used by all pure-transition functions.

use std::collections::VecDeque;

use crate::capacity::{CapacityRejection, validate_capacity};

// ---- Core aggregate ----

/// Validated bounded queue state for proof-oriented transition checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueState<T> {
    pub(crate) capacity: usize,
    pub(crate) items: VecDeque<T>,
}

/// Rejection returned when an existing concrete queue cannot be imported into a
/// bounded semantic state without weakening invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueStateRejection<T> {
    /// Capacity validation failed; the original concrete queue is preserved.
    Capacity {
        /// Capacity rejection reason.
        reason: CapacityRejection,
        /// Original concrete queue.
        items: VecDeque<T>,
    },
    /// Existing depth is larger than the fixed capacity; the original concrete
    /// queue is preserved.
    OverCapacity {
        /// Fixed queue capacity.
        capacity: usize,
        /// Observed queue depth.
        len: usize,
        /// Original concrete queue.
        items: VecDeque<T>,
    },
}

impl<T> QueueStateRejection<T> {
    /// Returns the original concrete queue so production callers can fail closed
    /// without losing queued work.
    #[must_use]
    pub fn into_vec_deque(self) -> VecDeque<T> {
        match self {
            Self::Capacity { items, .. } | Self::OverCapacity { items, .. } => items,
        }
    }
}

impl<T> QueueState<T> {
    /// Creates an empty queue state after validating capacity.
    pub fn new(capacity: usize, maximum: usize) -> Result<Self, CapacityRejection> {
        validate_capacity(capacity, maximum)?;
        Ok(Self {
            capacity,
            items: VecDeque::with_capacity(capacity),
        })
    }

    /// Imports an existing concrete FIFO queue into the semantic state without
    /// copying elements. This is the production bridge for concrete queues whose
    /// internals are otherwise outside the verifier scope.
    pub fn from_vec_deque(
        capacity: usize,
        maximum: usize,
        items: VecDeque<T>,
    ) -> Result<Self, QueueStateRejection<T>> {
        if let Err(reason) = validate_capacity(capacity, maximum) {
            return Err(QueueStateRejection::Capacity { reason, items });
        }
        let len = items.len();
        if len > capacity {
            return Err(QueueStateRejection::OverCapacity {
                capacity,
                len,
                items,
            });
        }
        Ok(Self { capacity, items })
    }

    /// Returns the concrete FIFO storage after a semantic transition.
    #[must_use]
    pub fn into_vec_deque(self) -> VecDeque<T> {
        self.items
    }

    /// Returns the fixed queue capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the current queue depth.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true when the queue has no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns true when the queue depth is at or above capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        queue_is_full(self.capacity, self.items.len())
    }
}

// ---- Decision types ----

/// Admission decision for bounded enqueue transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueDecision {
    /// The item may be appended exactly once.
    Accepted,
    /// The item must be rejected and prior queue membership preserved.
    QueueFull { capacity: usize },
}

/// Outcome of a dequeue/pop transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopTransition<T> {
    /// Empty queues remain empty and return no item.
    Empty { state: QueueState<T> },
    /// Non-empty queues return the old front and retain the old tail.
    Popped { state: QueueState<T>, item: T },
}

/// Zero-allocation pop/tick decision for concrete queues whose element storage
/// cannot be moved into [`QueueState`] without violating their abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopDecision {
    /// No item may be consumed.
    Empty,
    /// Exactly one old-front item may be consumed by the concrete queue.
    PopFront,
}

// ---- Warning types ----

/// Advisory warning transport outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningSendOutcome {
    /// Warning transport accepted the payload.
    Delivered,
    /// Warning transport was bounded and full.
    Full,
    /// Warning receiver was disconnected.
    Disconnected,
}

/// Warning payload derived from post-enqueue queue state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarningPayload {
    /// Depth after the successful enqueue.
    pub depth: usize,
    /// Fixed queue capacity.
    pub capacity: usize,
}

/// Warning transition result. Queue membership is unchanged by this transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarningTransition<T> {
    /// Unchanged queue state.
    pub state: QueueState<T>,
    /// Transport outcome observed by production.
    pub outcome: WarningSendOutcome,
    /// Exact payload when warning threshold is reached.
    pub payload: Option<WarningPayload>,
}

// ---- Observation helpers (pure const fn) ----

/// Verus-shared helper route for full-queue decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len >= capacity]))]
pub const fn helper_queue_is_full(capacity: usize, len: usize) -> bool {
    len >= capacity
}

/// Returns remaining capacity using saturating arithmetic for observation only.
#[must_use]
pub const fn remaining_capacity(capacity: usize, len: usize) -> usize {
    capacity.saturating_sub(len)
}

/// Returns true when a queue of `len` is full for `capacity`.
#[must_use]
pub const fn queue_is_full(capacity: usize, len: usize) -> bool {
    helper_queue_is_full(capacity, len)
}

#[cfg(test)]
mod tests;
