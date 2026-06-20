#![forbid(unsafe_code)]
//! MEM-01 LRU ring tests (master §77.8 + RED-QUEEN-MASTER-ISSUE-REPORT.md).
//!
//! Tests per tier-a-6-014 contract:
//!   1. test_terminal_runs_lru_bounded_under_load
//!   2. test_terminal_runs_lru_evicts_oldest_after_capacity
//!   3. test_terminal_runs_lru_respects_ttl_seconds

use crate::RuntimeError;
use crate::shard::lru_ring::{DEFAULT_MAX_TERMINAL_RUNS, LruRing};
use crate::shard::timer::TimerTick;
use vb_core::ids::RunId;

// ── test_terminal_runs_lru_bounded_under_load ───────────────────────────────

#[test]
fn test_terminal_runs_lru_bounded_under_load() {
    let capacity = 4;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, u64::MAX);

    // Insert `capacity` distinct runs; every insert must succeed.
    for index in 0..capacity {
        let run = RunId::new(index as u64 + 1);
        let outcome = ring.insert(run, TimerTick::new(index as u64));
        assert!(
            outcome.is_ok(),
            "insert under capacity must succeed, got: {outcome:?}"
        );
    }
    assert_eq!(
        ring.len(),
        capacity,
        "ring must hold exactly `capacity` entries before saturation"
    );

    // The next insert under a strict (no-TTL-evictable) clock must refuse
    // and surface TerminalRunsLruFull. The counter must reflect the
    // refused insert so operators can observe the pressure.
    let overflowing = RunId::new(999);
    let outcome = ring.insert(overflowing, TimerTick::new(capacity as u64));
    assert!(
        matches!(outcome, Err(RuntimeError::TerminalRunsLruFull { .. })),
        "overflow insert must return TerminalRunsLruFull, got: {outcome:?}"
    );
    assert_eq!(
        ring.counters().capacity_overflows,
        1,
        "overflow counter must increment exactly once"
    );
    assert_eq!(
        ring.len(),
        capacity,
        "ring must remain bounded at capacity under load"
    );

    // The default-capacity helper exposes the bead's documented default.
    let default_ring: LruRing<RunId> = LruRing::new(DEFAULT_MAX_TERMINAL_RUNS, 86_400);
    assert_eq!(default_ring.capacity(), DEFAULT_MAX_TERMINAL_RUNS);
    assert_eq!(default_ring.ttl_ticks(), 86_400);
}

// ── test_terminal_runs_lru_evicts_oldest_after_capacity ────────────────────

#[test]
fn test_terminal_runs_lru_evicts_oldest_after_capacity() {
    let capacity = 3;
    let ttl = 100;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, ttl);

    // Fill the ring at t=0..3.
    for index in 0..capacity {
        let run = RunId::new(index as u64 + 1);
        ring.insert(run, TimerTick::new(index as u64))
            .expect("first wave under capacity must succeed");
    }

    // At t=capacity, an insert must refuse because nothing has expired yet.
    let overflow = RunId::new(99);
    let outcome = ring.insert(overflow, TimerTick::new(capacity as u64));
    assert!(
        matches!(outcome, Err(RuntimeError::TerminalRunsLruFull { .. })),
        "insert at t=capacity with no TTL-expired entry must fail, got: {outcome:?}"
    );

    // Advance past the TTL horizon and re-attempt. All three fills happened
    // at t=0..2 with TTL=100, so the entire wave is expired once the
    // logical clock reaches capacity + ttl + 1. The sweep removes them in
    // insertion order and the new insert succeeds.
    let later_tick = TimerTick::new((capacity as u64) + ttl + 1);
    let outcome = ring.insert(RunId::new(100), later_tick);
    assert!(
        outcome.is_ok(),
        "insert after TTL horizon must succeed by evicting oldest, got: {outcome:?}"
    );
    assert!(
        !ring.contains(&RunId::new(1)),
        "oldest entry must be evicted, run 1 should be gone"
    );
    assert!(
        !ring.contains(&RunId::new(2)),
        "second oldest entry must be evicted too (entire wave expired)"
    );
    assert!(
        ring.contains(&RunId::new(100)),
        "newly-inserted run must be present"
    );
    assert_eq!(
        ring.counters().expired_evictions,
        capacity as u64,
        "TTL sweep must evict every entry past the horizon (got {})",
        ring.counters().expired_evictions
    );
    assert_eq!(
        ring.len(),
        1,
        "ring must hold only the freshly-inserted entry after the TTL sweep"
    );

    // force_insert must grow the ring past capacity (legacy behavior
    // preserved) and increment capacity_overflows rather than Err.
    // Reset to a known state: clear, then refill to exactly capacity at a
    // tick where TTL will not trigger.
    ring.clear();
    let baseline_overflows = ring.counters().capacity_overflows;
    for index in 0..capacity {
        let run = RunId::new(300 + index as u64);
        ring.force_insert(run, TimerTick::new(later_tick.get() + 2));
    }
    assert_eq!(
        ring.len(),
        capacity,
        "ring must be at capacity before overflow test"
    );
    let overflow2 = RunId::new(200);
    ring.force_insert(overflow2, TimerTick::new(later_tick.get() + 3));
    assert!(
        ring.contains(&overflow2),
        "force_insert must grow the ring past capacity when needed"
    );
    assert!(
        ring.counters().capacity_overflows > baseline_overflows,
        "force_insert past capacity must increment capacity_overflows (baseline={}, after={})",
        baseline_overflows,
        ring.counters().capacity_overflows
    );
}

// ── test_terminal_runs_lru_respects_ttl_seconds ────────────────────────────

#[test]
fn test_terminal_runs_lru_respects_ttl_seconds() {
    let capacity = 4;
    // 86_400 ticks matches the bead's documented ttl_seconds default when
    // the runtime advances at 1 tick / second.
    let ttl = 86_400u64;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, ttl);

    let baseline = TimerTick::new(1_000);
    for index in 0..capacity {
        let run = RunId::new(index as u64 + 1);
        ring.insert(run, baseline).expect("first wave must succeed");
    }
    assert_eq!(
        ring.counters().expired_evictions,
        0,
        "no TTL sweep must have happened yet"
    );

    // Insert a new entry just inside the TTL horizon — no eviction.
    let inside_ttl = TimerTick::new(baseline.get() + ttl - 1);
    let outcome = ring.insert(RunId::new(999), inside_ttl);
    assert!(
        outcome.is_err(),
        "insert at exactly ttl-1 must refuse because all existing entries are within TTL"
    );
    assert_eq!(
        ring.counters().expired_evictions,
        0,
        "no expired_evictions should have occurred at ttl-1"
    );

    // Explicit sweep after the TTL horizon clears every entry.
    ring.sweep_expired(TimerTick::new(baseline.get() + ttl + 1));
    assert_eq!(
        ring.counters().expired_evictions,
        capacity as u64,
        "sweep must evict every TTL-expired entry (got {})",
        ring.counters().expired_evictions
    );
    assert!(
        ring.is_empty(),
        "ring must be empty after the TTL horizon sweeps all entries"
    );

    // A subsequent insert after the sweep must succeed and remain stable.
    let outcome = ring.insert(RunId::new(2_000), TimerTick::new(baseline.get() + ttl + 2));
    assert!(
        outcome.is_ok(),
        "insert after TTL sweep must succeed, got: {outcome:?}"
    );
    assert!(ring.contains(&RunId::new(2_000)));
}

// ── vb-xfu6m remove O(1) behaviour ──────────────────────────────────────────

#[test]
fn test_remove_present_item_drops_it() {
    let capacity = 8usize;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, u64::MAX);
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 1), TimerTick::new(offset as u64))
            .expect("fill");
    }
    assert_eq!(ring.len(), capacity);

    let target = RunId::new(3);
    let before_counters = ring.counters();
    ring.remove(&target);

    assert!(!ring.contains(&target), "removed item must not be present");
    assert_eq!(ring.len(), capacity - 1, "len must drop by exactly one");
    assert_eq!(
        ring.counters(),
        before_counters,
        "remove must not touch expired_evictions or capacity_overflows"
    );

    // The other items must still be present.
    for offset in 0..capacity {
        let run = RunId::new(offset as u64 + 1);
        if run != target {
            assert!(
                ring.contains(&run),
                "untouched sibling must remain present (run {run:?})"
            );
        }
    }
}

#[test]
fn test_remove_absent_item_is_no_op() {
    let capacity = 4usize;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, u64::MAX);
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 1), TimerTick::new(offset as u64))
            .expect("fill");
    }
    let absent = RunId::new(999);
    assert!(!ring.contains(&absent), "absent item must not be present");

    let before_counters = ring.counters();
    ring.remove(&absent);

    assert!(!ring.contains(&absent));
    assert_eq!(
        ring.len(),
        capacity,
        "remove on absent item must not change membership size"
    );
    assert_eq!(
        ring.counters(),
        before_counters,
        "remove on absent item must not change counters"
    );
}

#[test]
fn test_remove_on_empty_ring_is_no_op() {
    let mut ring: LruRing<RunId> = LruRing::new(4, u64::MAX);
    assert!(ring.is_empty());
    let before_counters = ring.counters();

    ring.remove(&RunId::new(1));

    assert!(ring.is_empty(), "empty ring must remain empty after remove");
    assert_eq!(
        ring.len(),
        0,
        "empty ring length must stay zero after remove"
    );
    assert_eq!(
        ring.counters(),
        before_counters,
        "remove on empty ring must not change counters"
    );
}

#[test]
fn test_remove_after_sweep_expired_of_same_item_is_no_op() {
    let capacity = 4usize;
    let ttl = 50u64;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, ttl);
    let baseline = TimerTick::new(100);
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 1), baseline)
            .expect("fill");
    }
    let before_sweep_evictions = ring.counters().expired_evictions;

    // Sweep at a tick well past the TTL horizon; every entry is evicted
    // by sweep_expired (the linked-list head walks forward until the
    // first non-expired node, which there is none of here).
    ring.sweep_expired(TimerTick::new(baseline.get() + ttl + 1));
    assert_eq!(
        ring.counters().expired_evictions,
        before_sweep_evictions + capacity as u64,
        "sweep must evict every TTL-expired entry"
    );
    assert!(ring.is_empty(), "ring must be empty after sweep");

    let before_counters = ring.counters();
    // Removing an item that sweep_expired already evicted must be a no-op:
    // the position map no longer has the entry, so remove short-circuits.
    ring.remove(&RunId::new(2));

    assert!(ring.is_empty(), "ring must still be empty");
    assert_eq!(
        ring.counters(),
        before_counters,
        "remove on already-evicted item must not change any counter"
    );
}

#[test]
fn test_remove_preserves_membership_and_order_invariants() {
    // Interleave inserts, removes, and a sweep to verify that the
    // position map stays consistent with the linked-list ordering.
    let capacity = 6usize;
    let ttl = 1_000u64;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, ttl);

    let baseline = TimerTick::new(0);
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 1), baseline)
            .expect("fill");
    }

    // Remove the second-oldest, the middle, and the newest — three
    // different positions in the linked list — and re-insert each so
    // the free list is exercised across both reused and fresh slots.
    ring.remove(&RunId::new(2));
    ring.remove(&RunId::new(4));
    ring.remove(&RunId::new(6));

    let survivors = [RunId::new(1), RunId::new(3), RunId::new(5)];
    for run in survivors {
        assert!(ring.contains(&run), "survivor {run:?} must remain present");
    }
    assert_eq!(ring.len(), survivors.len());

    // After re-insert, ring must be at capacity and every entry is
    // distinct from the survivors (ids 2, 4, 6 are not reused).
    ring.insert(RunId::new(2), TimerTick::new(500))
        .expect("re-insert");
    ring.insert(RunId::new(4), TimerTick::new(600))
        .expect("re-insert");
    ring.insert(RunId::new(6), TimerTick::new(700))
        .expect("re-insert");
    assert_eq!(
        ring.len(),
        capacity,
        "ring must reach capacity again after refill"
    );

    // Sweep everything (all entries are at ts <= 700 with ttl = 1000,
    // sweep at ts = 2000 evicts all).
    ring.sweep_expired(TimerTick::new(2_000));
    assert!(
        ring.is_empty(),
        "sweep at ts=2000 must evict every entry (ttl=1000)"
    );
    assert_eq!(
        ring.counters().expired_evictions,
        capacity as u64,
        "expired_evictions must reflect every sweep removal"
    );
}
