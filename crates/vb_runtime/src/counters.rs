//! Atomic-ish counters for runtime observability.

use core::sync::atomic::{AtomicU64, Ordering};

/// Shard-level counters for runs submitted, completed, failed, and steps executed.
#[derive(Debug)]
pub struct ShardCounters {
    runs_submitted: AtomicU64,
    runs_completed: AtomicU64,
    runs_failed: AtomicU64,
    steps_executed: AtomicU64,
}

impl Default for ShardCounters {
    fn default() -> Self {
        Self::new()
    }
}

impl ShardCounters {
    /// Creates zeroed counters.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            runs_submitted: AtomicU64::new(0),
            runs_completed: AtomicU64::new(0),
            runs_failed: AtomicU64::new(0),
            steps_executed: AtomicU64::new(0),
        }
    }

    /// Increments the runs-submitted counter.
    pub fn inc_submitted(&self) {
        self.runs_submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the runs-completed counter.
    pub fn inc_completed(&self) {
        self.runs_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the runs-failed counter.
    pub fn inc_failed(&self) {
        self.runs_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Adds to the steps-executed counter.
    pub fn add_steps(&self, count: u64) {
        self.steps_executed.fetch_add(count, Ordering::Relaxed);
    }
}

/// Snapshot of all shard counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterSnapshot {
    /// Total runs submitted.
    pub runs_submitted: u64,
    /// Total runs completed successfully.
    pub runs_completed: u64,
    /// Total runs failed.
    pub runs_failed: u64,
    /// Total steps executed across all runs.
    pub steps_executed: u64,
}

impl ShardCounters {
    /// Drains a snapshot of all counters.
    pub fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            runs_submitted: self.runs_submitted.load(Ordering::Relaxed),
            runs_completed: self.runs_completed.load(Ordering::Relaxed),
            runs_failed: self.runs_failed.load(Ordering::Relaxed),
            steps_executed: self.steps_executed.load(Ordering::Relaxed),
        }
    }
}
