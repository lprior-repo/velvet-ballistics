//! Warning payload and transport outcome types for queue transitions.
//!
//! Defines [`WarningPayload`], [`WarningTransition`], and [`WarningSendOutcome`] —
//! the types that model warning emission during enqueue/pop operations.

use crate::state::QueueState;

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

impl WarningSendOutcome {
    /// Returns true if the warning was successfully delivered.
    #[must_use]
    pub const fn is_delivered(&self) -> bool {
        matches!(self, Self::Delivered)
    }
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
