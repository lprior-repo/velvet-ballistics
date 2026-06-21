#![forbid(unsafe_code)]
//! Atomic-ish counters for runtime observability.

use core::sync::atomic::{AtomicU64, Ordering};

/// Shard-level counters for runs submitted, completed, failed, and steps executed.
#[derive(Debug)]
pub struct ShardCounters {
    runs_submitted: AtomicU64,
    runs_completed: AtomicU64,
    runs_failed: AtomicU64,
    runs_cancelled: AtomicU64,
    runs_killed: AtomicU64,
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
            runs_cancelled: AtomicU64::new(0),
            runs_killed: AtomicU64::new(0),
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

    /// Increments the runs-cancelled counter (RQ-W0-17: previously conflated with `inc_failed`).
    pub fn inc_cancelled(&self) {
        self.runs_cancelled.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the runs-killed counter (RQ-W0-17: previously conflated with `inc_failed`).
    pub fn inc_killed(&self) {
        self.runs_killed.fetch_add(1, Ordering::Relaxed);
    }

    /// Adds to the steps-executed counter.
    pub fn add_steps(&self, count: u64) {
        self.steps_executed.fetch_add(count, Ordering::Relaxed);
    }
}

#[cfg(test)]
#[path = "counters/tests.rs"]
mod tests;

/// Snapshot of all shard counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterSnapshot {
    /// Total runs submitted.
    pub runs_submitted: u64,
    /// Total runs completed successfully.
    pub runs_completed: u64,
    /// Total runs failed during deterministic execution.
    pub runs_failed: u64,
    /// Total runs cancelled via `ShardCommand::Cancel` (RQ-W0-17).
    pub runs_cancelled: u64,
    /// Total runs killed via `ShardCommand::Kill` (RQ-W0-17).
    pub runs_killed: u64,
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
            runs_cancelled: self.runs_cancelled.load(Ordering::Relaxed),
            runs_killed: self.runs_killed.load(Ordering::Relaxed),
            steps_executed: self.steps_executed.load(Ordering::Relaxed),
        }
    }
}

/// Per-shard metrics snapshot for observability.
#[derive(Debug, Clone)]
pub struct ShardMetricsSnapshot {
    /// Shard index in the runtime's shard vector.
    pub shard_id: u32,
    /// Number of active runs on this shard.
    pub active_runs: u32,
    /// Commands waiting in the ready queue.
    pub command_queue_depth: u32,
    /// Remaining free slots in the command queue.
    pub command_queue_remaining: u32,
    /// Number of pending timers.
    pub pending_timers: u32,
    /// Free frames in the frame pool.
    pub frame_pool_free: u32,
    /// Total capacity of the frame pool.
    pub frame_pool_total: u32,
    /// Trace ring fill percentage (0.0 - 100.0).
    pub trace_ring_fill_pct: f32,
    /// Counter snapshot.
    pub counters: CounterSnapshot,
}

/// Aggregate runtime metrics snapshot.
#[derive(Debug, Clone)]
pub struct RuntimeMetricsSnapshot {
    /// Per-shard metrics.
    pub shards: Vec<ShardMetricsSnapshot>,
    /// Total active runs across all shards.
    pub runs_active: u32,
    /// Total pending timers across all shards.
    pub runs_waiting: u32,
    /// Total runs failed across all shards.
    pub runs_failed_total: u64,
    /// Total runs finished across all shards.
    pub runs_finished_total: u64,
    /// Total runs cancelled across all shards (RQ-W0-17).
    pub runs_cancelled_total: u64,
    /// Total runs killed across all shards (RQ-W0-17).
    pub runs_killed_total: u64,
    /// Total steps executed across all shards.
    pub steps_total: u64,
}
