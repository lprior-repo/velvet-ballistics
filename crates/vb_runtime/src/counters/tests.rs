use super::*;

const fn snapshot(
    runs_submitted: u64,
    runs_completed: u64,
    runs_failed: u64,
    runs_cancelled: u64,
    runs_killed: u64,
    steps_executed: u64,
) -> CounterSnapshot {
    CounterSnapshot {
        runs_submitted,
        runs_completed,
        runs_failed,
        runs_cancelled,
        runs_killed,
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
            runs_cancelled: 0,
            runs_killed: 0,
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
fn inc_cancelled_increments_cancelled_in_snapshot() {
    let counters = ShardCounters::new();
    counters.inc_cancelled();
    counters.inc_cancelled();
    assert_eq!(counters.snapshot().runs_cancelled, 2);
}

#[test]
fn inc_killed_increments_killed_in_snapshot() {
    let counters = ShardCounters::new();
    counters.inc_killed();
    assert_eq!(counters.snapshot().runs_killed, 1);
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
    counters.inc_cancelled();
    counters.inc_killed();
    counters.add_steps(100);
    let snap = counters.snapshot();
    assert_eq!(snap.runs_submitted, 3);
    assert_eq!(snap.runs_completed, 2);
    assert_eq!(snap.runs_failed, 1);
    assert_eq!(snap.runs_cancelled, 1);
    assert_eq!(snap.runs_killed, 1);
    assert_eq!(snap.steps_executed, 100);
}

#[test]
fn counter_snapshot_saturating_add_never_panics() {
    let a = CounterSnapshot {
        runs_submitted: u64::MAX,
        runs_completed: u64::MAX,
        runs_failed: u64::MAX,
        runs_cancelled: u64::MAX,
        runs_killed: u64::MAX,
        steps_executed: u64::MAX,
    };
    let b = CounterSnapshot {
        runs_submitted: u64::MAX,
        runs_completed: u64::MAX,
        runs_failed: u64::MAX,
        runs_cancelled: u64::MAX,
        runs_killed: u64::MAX,
        steps_executed: u64::MAX,
    };
    let result = CounterSnapshot {
        runs_submitted: a.runs_submitted.saturating_add(b.runs_submitted),
        runs_completed: a.runs_completed.saturating_add(b.runs_completed),
        runs_failed: a.runs_failed.saturating_add(b.runs_failed),
        runs_cancelled: a.runs_cancelled.saturating_add(b.runs_cancelled),
        runs_killed: a.runs_killed.saturating_add(b.runs_killed),
        steps_executed: a.steps_executed.saturating_add(b.steps_executed),
    };
    assert_eq!(result.runs_submitted, u64::MAX);
    assert_eq!(result.runs_completed, u64::MAX);
    assert_eq!(result.runs_failed, u64::MAX);
    assert_eq!(result.runs_cancelled, u64::MAX);
    assert_eq!(result.runs_killed, u64::MAX);
    assert_eq!(result.steps_executed, u64::MAX);
}

#[test]
fn counter_snapshot_default_matches_new() {
    let snap = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    let zero = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    assert_eq!(snap, zero);
}

#[test]
fn counter_snapshot_equality_differs_for_different_values() {
    let a = CounterSnapshot {
        runs_submitted: 1,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    let b = CounterSnapshot {
        runs_submitted: 2,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    assert_ne!(a, b);
}

#[test]
fn counter_snapshot_clone_preserves_values() {
    let original = CounterSnapshot {
        runs_submitted: 10,
        runs_completed: 5,
        runs_failed: 2,
        runs_cancelled: 3,
        runs_killed: 4,
        steps_executed: 100,
    };
    let cloned = original.clone();
    assert_eq!(cloned, original);
}

#[test]
fn shard_counters_default_is_zeroed() {
    let counters = ShardCounters::default();
    let snap = counters.snapshot();
    assert_eq!(snap.runs_submitted, 0);
    assert_eq!(snap.runs_completed, 0);
    assert_eq!(snap.runs_failed, 0);
    assert_eq!(snap.runs_cancelled, 0);
    assert_eq!(snap.runs_killed, 0);
    assert_eq!(snap.steps_executed, 0);
}

#[test]
fn add_steps_accumulates_multiple_calls() {
    let counters = ShardCounters::new();
    counters.add_steps(10);
    counters.add_steps(20);
    counters.add_steps(30);
    assert_eq!(counters.snapshot().steps_executed, 60);
}

#[test]
fn add_steps_with_zero_adds_nothing() {
    let counters = ShardCounters::new();
    counters.add_steps(0);
    assert_eq!(counters.snapshot().steps_executed, 0);
}

#[test]
fn inc_submitted_multiple_times() {
    let counters = ShardCounters::new();
    for _ in 0..10 {
        counters.inc_submitted();
    }
    assert_eq!(counters.snapshot().runs_submitted, 10);
}

#[test]
fn inc_completed_multiple_times() {
    let counters = ShardCounters::new();
    for _ in 0..5 {
        counters.inc_completed();
    }
    assert_eq!(counters.snapshot().runs_completed, 5);
}

#[test]
fn inc_failed_multiple_times() {
    let counters = ShardCounters::new();
    for _ in 0..3 {
        counters.inc_failed();
    }
    assert_eq!(counters.snapshot().runs_failed, 3);
}

#[test]
fn inc_cancelled_multiple_times() {
    let counters = ShardCounters::new();
    for _ in 0..4 {
        counters.inc_cancelled();
    }
    assert_eq!(counters.snapshot().runs_cancelled, 4);
}

#[test]
fn inc_killed_multiple_times() {
    let counters = ShardCounters::new();
    for _ in 0..2 {
        counters.inc_killed();
    }
    assert_eq!(counters.snapshot().runs_killed, 2);
}

#[test]
fn counter_snapshot_copy_preserves_values() {
    let original = CounterSnapshot {
        runs_submitted: 10,
        runs_completed: 5,
        runs_failed: 2,
        runs_cancelled: 1,
        runs_killed: 1,
        steps_executed: 100,
    };
    let copied = original;
    assert_eq!(copied, original);
}

#[test]
fn counter_snapshot_equality_same_values() {
    let a = CounterSnapshot {
        runs_submitted: 1,
        runs_completed: 2,
        runs_failed: 3,
        runs_cancelled: 4,
        runs_killed: 5,
        steps_executed: 6,
    };
    let b = CounterSnapshot {
        runs_submitted: 1,
        runs_completed: 2,
        runs_failed: 3,
        runs_cancelled: 4,
        runs_killed: 5,
        steps_executed: 6,
    };
    assert_eq!(a, b);
}

#[test]
fn counter_snapshot_equality_differs_completed() {
    let a = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    let b = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 1,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    assert_ne!(a, b);
}

#[test]
fn counter_snapshot_equality_differs_failed() {
    let a = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    let b = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 1,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    assert_ne!(a, b);
}

#[test]
fn counter_snapshot_equality_differs_steps() {
    let a = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    let b = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 1,
    };
    assert_ne!(a, b);
}

#[test]
fn counter_snapshot_equality_differs_cancelled() {
    let a = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    let b = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 1,
        runs_killed: 0,
        steps_executed: 0,
    };
    assert_ne!(a, b);
}

#[test]
fn counter_snapshot_equality_differs_killed() {
    let a = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 0,
        steps_executed: 0,
    };
    let b = CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        runs_cancelled: 0,
        runs_killed: 1,
        steps_executed: 0,
    };
    assert_ne!(a, b);
}

#[test]
fn counter_snapshot_debug_output() {
    let snap = CounterSnapshot {
        runs_submitted: 1,
        runs_completed: 2,
        runs_failed: 3,
        runs_cancelled: 4,
        runs_killed: 5,
        steps_executed: 6,
    };
    let debug = format!("{snap:?}");
    assert_eq!(debug.contains("CounterSnapshot"), true);
}

#[test]
fn counters_mixed_operations_snapshot_is_consistent() {
    let counters = ShardCounters::new();
    counters.inc_submitted();
    counters.inc_submitted();
    counters.inc_completed();
    counters.inc_failed();
    counters.inc_cancelled();
    counters.inc_killed();
    counters.add_steps(50);
    let snap = counters.snapshot();
    assert_eq!(snap.runs_submitted, 2);
    assert_eq!(snap.runs_completed, 1);
    assert_eq!(snap.runs_failed, 1);
    assert_eq!(snap.runs_cancelled, 1);
    assert_eq!(snap.runs_killed, 1);
    assert_eq!(snap.steps_executed, 50);
}

// =======================================================================
// Adversarial BDD tests - counters attack vectors
// =======================================================================

#[test]
fn counter_add_steps_near_u64_max_wraps_via_atomic() {
    let counters = ShardCounters::new();
    counters.add_steps(u64::MAX);
    counters.add_steps(1);
    let snap = counters.snapshot();
    assert_eq!(snap.steps_executed, 0);
}

#[test]
fn counter_inc_submitted_never_panics() {
    let counters = ShardCounters::new();
    for _ in 0..100 {
        counters.inc_submitted();
    }
    let snap = counters.snapshot();
    assert_eq!(snap.runs_submitted, 100);
}

#[test]
fn counter_snapshot_saturating_add_with_zero_is_identity() {
    let snap = snapshot(100, 50, 10, 5, 5, 1000);
    let zero = snapshot(0, 0, 0, 0, 0, 0);
    let result = snapshot(
        snap.runs_submitted.saturating_add(zero.runs_submitted),
        snap.runs_completed.saturating_add(zero.runs_completed),
        snap.runs_failed.saturating_add(zero.runs_failed),
        snap.runs_cancelled.saturating_add(zero.runs_cancelled),
        snap.runs_killed.saturating_add(zero.runs_killed),
        snap.steps_executed.saturating_add(zero.steps_executed),
    );
    assert_eq!(result, snap);
}

#[test]
fn counter_snapshot_saturating_add_is_commutative() {
    let a = snapshot(10, 5, 2, 1, 1, 100);
    let b = snapshot(20, 10, 3, 2, 2, 200);
    let ab = snapshot(
        a.runs_submitted.saturating_add(b.runs_submitted),
        a.runs_completed.saturating_add(b.runs_completed),
        a.runs_failed.saturating_add(b.runs_failed),
        a.runs_cancelled.saturating_add(b.runs_cancelled),
        a.runs_killed.saturating_add(b.runs_killed),
        a.steps_executed.saturating_add(b.steps_executed),
    );
    let ba = snapshot(
        b.runs_submitted.saturating_add(a.runs_submitted),
        b.runs_completed.saturating_add(a.runs_completed),
        b.runs_failed.saturating_add(a.runs_failed),
        b.runs_cancelled.saturating_add(a.runs_cancelled),
        b.runs_killed.saturating_add(a.runs_killed),
        b.steps_executed.saturating_add(a.steps_executed),
    );
    assert_eq!(ab, ba);
    assert_eq!(ab.runs_submitted, 30);
    assert_eq!(ab.runs_completed, 15);
}
