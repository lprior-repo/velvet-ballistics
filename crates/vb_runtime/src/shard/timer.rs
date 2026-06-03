#![forbid(unsafe_code)]
//! Timer types for deterministic execution control.

use std::time::Instant;

use vb_core::ids::{ActionId, StepIdx};

// ============================================================================
// Pending Timer (wall-clock based)
// ============================================================================

/// Kind of pending timer (wall-clock based).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PendingTimerKind {
    Wait,
    Ask,
}

/// A pending timer for wall-clock based waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTimer {
    pub step: StepIdx,
    pub kind: PendingTimerKind,
    pub generation: u64,
    pub deadline: Instant,
}

impl PendingTimer {
    #[must_use]
    pub fn matches_authority(
        self,
        generation: u64,
        deadline: Instant,
        kind: PendingTimerKind,
    ) -> bool {
        self.generation == generation && self.deadline == deadline && self.kind == kind
    }
}

// ============================================================================
// Numeric Timer Seam Types (logical time based)
// ============================================================================

/// A monotonically increasing timer tick value, counting logical time units.
///
/// Wraps a `u64` to provide type safety and checked arithmetic for deterministic
/// clock control. One tick represents one logical time unit in the deterministic
/// timer seam, operating alongside the existing wall-clock `Instant`-based timers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerTick(u64);

impl TimerTick {
    /// Creates a new timer tick at the given value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the inner `u64` value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Advances the tick by a duration, returning the resulting tick.
    ///
    /// Returns `None` on overflow.
    #[must_use]
    pub fn checked_add(self, duration: TimerDuration) -> Option<Self> {
        self.0.checked_add(duration.get()).map(Self)
    }

    /// Returns `true` if this tick is at or past the given deadline.
    #[must_use]
    pub fn has_elapsed(self, deadline: TimerDeadline) -> bool {
        self.0 >= deadline.get()
    }
}

/// A timer duration measured in ticks, representing a span of logical time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerDuration(u64);

impl TimerDuration {
    /// Creates a new duration with the given number of ticks.
    #[must_use]
    pub const fn new(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Returns the inner `u64` value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the duration as a tick count.
    #[must_use]
    pub const fn as_ticks(self) -> u64 {
        self.0
    }

    /// Returns a zero-length duration.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }
}

/// An absolute deadline in ticks, representing when a timer expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerDeadline(u64);

impl TimerDeadline {
    /// Creates a new deadline at the given tick value.
    #[must_use]
    pub const fn new(tick: u64) -> Self {
        Self(tick)
    }

    /// Returns the inner `u64` value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Creates a deadline by adding a duration to a tick.
    ///
    /// Returns `None` on overflow.
    #[must_use]
    pub fn from_tick_and_duration(tick: TimerTick, duration: TimerDuration) -> Option<Self> {
        tick.get().checked_add(duration.get()).map(Self)
    }

    /// Returns `true` if this deadline has passed relative to the given tick.
    #[must_use]
    pub fn is_past(self, current: TimerTick) -> bool {
        current.has_elapsed(self)
    }
}

/// Kind of timer managed by the numeric timer seam.
///
/// Used alongside the existing `PendingTimerKind` to provide a richer
/// timer taxonomy for deterministic execution control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimerKind {
    /// Retry timer — combined wait/ask semantics for deterministic execution.
    Retry,
    /// Delayed action bound to a specific action identifier.
    DelayedAction(ActionId),
}
