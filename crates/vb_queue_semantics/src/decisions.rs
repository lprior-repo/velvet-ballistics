//! Decision types for queue transitions.
//!
//! Defines [`EnqueueDecision`], [`PopTransition`], and [`PopDecision`] — the
//! pure-enumeration outcomes used by all state-transition functions.


use super::state::QueueState;

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
