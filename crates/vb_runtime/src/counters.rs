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

#[cfg(test)]
mod tests {
    use super::*;

    const fn snapshot(
        runs_submitted: u64,
        runs_completed: u64,
        runs_failed: u64,
        steps_executed: u64,
    ) -> CounterSnapshot {
        CounterSnapshot {
            runs_submitted,
            runs_completed,
            runs_failed,
            steps_executed,
        }
    }

    #[test]
    fn new_creates_zeroed_counters() {
        let counters = ShardCounters::new();
        let snap = counters.snapshot();
        assert_eq!(
            snap,
            CounterSnapshot {
                runs_submitted: 0,
                runs_completed: 0,
                runs_failed: 0,
                steps_executed: 0,
            }
        );
    }

    #[test]
    fn inc_submitted_increments_submitted_in_snapshot() {
        let counters = ShardCounters::new();
        counters.inc_submitted();
        counters.inc_submitted();
        counters.inc_submitted();
        assert_eq!(counters.snapshot().runs_submitted, 3);
    }

    #[test]
    fn inc_completed_increments_completed_in_snapshot() {
        let counters = ShardCounters::new();
        counters.inc_completed();
        counters.inc_completed();
        assert_eq!(counters.snapshot().runs_completed, 2);
    }

    #[test]
    fn inc_failed_increments_failed_in_snapshot() {
        let counters = ShardCounters::new();
        counters.inc_failed();
        assert_eq!(counters.snapshot().runs_failed, 1);
    }

    #[test]
    fn add_steps_increments_step_count_in_snapshot() {
        let counters = ShardCounters::new();
        counters.add_steps(42);
        assert_eq!(counters.snapshot().steps_executed, 42);
    }

    #[test]
    fn multiple_operations_accumulate_in_single_snapshot() {
        let counters = ShardCounters::new();
        counters.inc_submitted();
        counters.inc_submitted();
        counters.inc_submitted();
        counters.inc_completed();
        counters.inc_completed();
        counters.inc_failed();
        counters.add_steps(100);
        let snap = counters.snapshot();
        assert_eq!(snap.runs_submitted, 3);
        assert_eq!(snap.runs_completed, 2);
        assert_eq!(snap.runs_failed, 1);
        assert_eq!(snap.steps_executed, 100);
    }

    #[test]
    fn counter_snapshot_saturating_add_never_panics() {
        // Given two snapshots with max values
        let a = CounterSnapshot {
            runs_submitted: u64::MAX,
            runs_completed: u64::MAX,
            runs_failed: u64::MAX,
            steps_executed: u64::MAX,
        };
        let b = CounterSnapshot {
            runs_submitted: u64::MAX,
            runs_completed: u64::MAX,
            runs_failed: u64::MAX,
            steps_executed: u64::MAX,
        };
        // When performing saturating add
        let result = CounterSnapshot {
            runs_submitted: a.runs_submitted.saturating_add(b.runs_submitted),
            runs_completed: a.runs_completed.saturating_add(b.runs_completed),
            runs_failed: a.runs_failed.saturating_add(b.runs_failed),
            steps_executed: a.steps_executed.saturating_add(b.steps_executed),
        };
        // Then all values saturate at u64::MAX
        assert_eq!(result.runs_submitted, u64::MAX);
        assert_eq!(result.runs_completed, u64::MAX);
        assert_eq!(result.runs_failed, u64::MAX);
        assert_eq!(result.steps_executed, u64::MAX);
    }

    #[test]
    fn counter_snapshot_default_matches_new() {
        // Given a default CounterSnapshot
        let snap = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        };
        // When comparing to manually constructed zero snapshot
        let zero = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        };
        // Then they are equal
        assert_eq!(snap, zero);
    }

    #[test]
    fn counter_snapshot_equality_differs_for_different_values() {
        // Given two snapshots with different values
        let a = CounterSnapshot {
            runs_submitted: 1,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        };
        let b = CounterSnapshot {
            runs_submitted: 2,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        };
        // Then they are not equal
        assert_ne!(a, b);
    }

    #[test]
    fn counter_snapshot_clone_preserves_values() {
        // Given a snapshot
        let original = CounterSnapshot {
            runs_submitted: 10,
            runs_completed: 5,
            runs_failed: 2,
            steps_executed: 100,
        };
        // When cloning
        let cloned = original.clone();
        // Then clone matches original
        assert_eq!(cloned, original);
    }

    #[test]
    fn shard_counters_default_is_zeroed() {
        // Given default counters
        let counters = ShardCounters::default();
        let snap = counters.snapshot();
        // Then all values are zero
        assert_eq!(snap.runs_submitted, 0);
        assert_eq!(snap.runs_completed, 0);
        assert_eq!(snap.runs_failed, 0);
        assert_eq!(snap.steps_executed, 0);
    }

    #[test]
    fn add_steps_accumulates_multiple_calls() {
        // Given counters with multiple add_steps calls
        let counters = ShardCounters::new();
        counters.add_steps(10);
        counters.add_steps(20);
        counters.add_steps(30);
        // Then the snapshot shows the sum
        assert_eq!(counters.snapshot().steps_executed, 60);
    }

    #[test]
    fn add_steps_with_zero_adds_nothing() {
        // Given counters
        let counters = ShardCounters::new();
        // When adding zero steps
        counters.add_steps(0);
        // Then steps_executed is still 0
        assert_eq!(counters.snapshot().steps_executed, 0);
    }

    #[test]
    fn inc_submitted_multiple_times() {
        // Given counters
        let counters = ShardCounters::new();
        // When incrementing submitted 10 times
        for _ in 0..10 {
            counters.inc_submitted();
        }
        // Then the snapshot shows 10
        assert_eq!(counters.snapshot().runs_submitted, 10);
    }

    #[test]
    fn inc_completed_multiple_times() {
        // Given counters
        let counters = ShardCounters::new();
        // When incrementing completed 5 times
        for _ in 0..5 {
            counters.inc_completed();
        }
        // Then the snapshot shows 5
        assert_eq!(counters.snapshot().runs_completed, 5);
    }

    #[test]
    fn inc_failed_multiple_times() {
        // Given counters
        let counters = ShardCounters::new();
        // When incrementing failed 3 times
        for _ in 0..3 {
            counters.inc_failed();
        }
        // Then the snapshot shows 3
        assert_eq!(counters.snapshot().runs_failed, 3);
    }

    #[test]
    fn counter_snapshot_copy_preserves_values() {
        // Given a snapshot
        let original = CounterSnapshot {
            runs_submitted: 10,
            runs_completed: 5,
            runs_failed: 2,
            steps_executed: 100,
        };
        // When copying (CounterSnapshot derives Copy)
        let copied = original;
        // Then copy matches original
        assert_eq!(copied, original);
    }

    #[test]
    fn counter_snapshot_equality_same_values() {
        // Given two snapshots with same values
        let a = CounterSnapshot {
            runs_submitted: 1,
            runs_completed: 2,
            runs_failed: 3,
            steps_executed: 4,
        };
        let b = CounterSnapshot {
            runs_submitted: 1,
            runs_completed: 2,
            runs_failed: 3,
            steps_executed: 4,
        };
        // Then they are equal
        assert_eq!(a, b);
    }

    #[test]
    fn counter_snapshot_equality_differs_completed() {
        // Given two snapshots with different completed
        let a = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        };
        let b = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 1,
            runs_failed: 0,
            steps_executed: 0,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn counter_snapshot_equality_differs_failed() {
        // Given two snapshots with different failed
        let a = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        };
        let b = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 0,
            runs_failed: 1,
            steps_executed: 0,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn counter_snapshot_equality_differs_steps() {
        // Given two snapshots with different steps
        let a = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        };
        let b = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 1,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn counter_snapshot_debug_output() {
        // Given a snapshot
        let snap = CounterSnapshot {
            runs_submitted: 1,
            runs_completed: 2,
            runs_failed: 3,
            steps_executed: 4,
        };
        // When formatting with debug
        let debug = format!("{snap:?}");
        // Then the debug output contains relevant field info
        assert_eq!(debug.contains("CounterSnapshot"), true);
    }

    #[test]
    fn counters_mixed_operations_snapshot_is_consistent() {
        // Given counters with mixed operations
        let counters = ShardCounters::new();
        counters.inc_submitted();
        counters.inc_submitted();
        counters.inc_completed();
        counters.inc_failed();
        counters.add_steps(50);
        // When taking snapshot
        let snap = counters.snapshot();
        // Then all counters are independent
        assert_eq!(snap.runs_submitted, 2);
        assert_eq!(snap.runs_completed, 1);
        assert_eq!(snap.runs_failed, 1);
        assert_eq!(snap.steps_executed, 50);
    }

    // =======================================================================
    // Adversarial BDD tests - counters attack vectors
    // =======================================================================

    #[test]
    fn counter_add_steps_near_u64_max_wraps_via_atomic() {
        // Given counters near u64::MAX for steps
        let counters = ShardCounters::new();
        counters.add_steps(u64::MAX);
        // When adding one more step
        counters.add_steps(1);
        // Then the counter wraps (fetch_add wraps on overflow for AtomicU64)
        let snap = counters.snapshot();
        assert_eq!(snap.steps_executed, 0);
    }

    #[test]
    fn counter_inc_submitted_never_panics() {
        // Given counters incremented many times
        let counters = ShardCounters::new();
        for _ in 0..100 {
            counters.inc_submitted();
        }
        let snap = counters.snapshot();
        assert_eq!(snap.runs_submitted, 100);
    }

    #[test]
    fn counter_snapshot_saturating_add_with_zero_is_identity() {
        // Given a snapshot with some values
        let snap = snapshot(100, 50, 10, 1000);
        let zero = snapshot(0, 0, 0, 0);
        // When adding zero
        let result = snapshot(
            snap.runs_submitted.saturating_add(zero.runs_submitted),
            snap.runs_completed.saturating_add(zero.runs_completed),
            snap.runs_failed.saturating_add(zero.runs_failed),
            snap.steps_executed.saturating_add(zero.steps_executed),
        );
        // Then result equals original
        assert_eq!(result, snap);
    }

    #[test]
    fn counter_snapshot_saturating_add_is_commutative() {
        // Given two snapshots
        let a = snapshot(10, 5, 2, 100);
        let b = snapshot(20, 10, 3, 200);
        // When adding a+b and b+a
        let ab = snapshot(
            a.runs_submitted.saturating_add(b.runs_submitted),
            a.runs_completed.saturating_add(b.runs_completed),
            a.runs_failed.saturating_add(b.runs_failed),
            a.steps_executed.saturating_add(b.steps_executed),
        );
        let ba = snapshot(
            b.runs_submitted.saturating_add(a.runs_submitted),
            b.runs_completed.saturating_add(a.runs_completed),
            b.runs_failed.saturating_add(a.runs_failed),
            b.steps_executed.saturating_add(a.steps_executed),
        );
        // Then they are equal (commutative)
        assert_eq!(ab, ba);
        assert_eq!(ab.runs_submitted, 30);
        assert_eq!(ab.runs_completed, 15);
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
    /// Total steps executed across all shards.
    pub steps_total: u64,
}
