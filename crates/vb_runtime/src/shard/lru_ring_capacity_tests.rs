#![forbid(unsafe_code)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test-only lint overrides; production code in lru_ring.rs is unaffected"
)]
//! MEM-01 LRU ring capacity/TTL tests (master §77.8 + RED-QUEEN-MASTER-ISSUE-REPORT.md).
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
    let mut ring: LruRing<RunId> = LruRing::try_new(capacity, u64::MAX).expect("non-zero test capacity");

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
    let default_ring: LruRing<RunId> = LruRing::try_new(DEFAULT_MAX_TERMINAL_RUNS, 86_400).expect("non-zero test capacity");
    assert_eq!(default_ring.capacity(), DEFAULT_MAX_TERMINAL_RUNS);
    assert_eq!(default_ring.ttl_ticks(), 86_400);
}

// ── test_terminal_runs_lru_evicts_oldest_after_capacity ────────────────────

#[test]
fn test_terminal_runs_lru_evicts_oldest_after_capacity() {
    let capacity = 3;
    let ttl = 100;
    let mut ring: LruRing<RunId> = LruRing::try_new(capacity, ttl).expect("non-zero test capacity");

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
    let mut ring: LruRing<RunId> = LruRing::try_new(capacity, ttl).expect("non-zero test capacity");

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
    ring.sweep_expired(TimerTick::new(baseline.get() + ttl + 1))
        .expect("sweep past ttl must succeed");
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

// ── BH-W0-S10 — re-insert of a present item preserves the original timestamp ─

#[test]
fn reinsert_preserves_original_timestamp() {
    let capacity = 4;
    let ttl = 100u64;
    let mut ring: LruRing<RunId> =
        LruRing::try_new(capacity, ttl).expect("non-zero test capacity");

    let run = RunId::new(42);
    let original_tick = TimerTick::new(1_000);

    // First insert records the timestamp.
    ring.insert(run, original_tick)
        .expect("first insert must succeed");

    // Re-insert the same item at a strictly later tick. The contract is
    // idempotent membership: the new `now` MUST NOT overwrite the
    // recorded insertion tick, otherwise TTL eviction would silently
    // extend the entry's lifetime and break the bounded-history
    // contract.
    let later_tick = TimerTick::new(original_tick.get() + 50);
    let outcome = ring.insert(run, later_tick);
    assert!(
        outcome.is_ok(),
        "re-insert of a present item must succeed idempotently, got: {outcome:?}"
    );
    assert_eq!(ring.len(), 1, "re-insert must not duplicate membership");

    // Sweep at `original_tick + ttl + 1`: this tick exceeds the
    // ORIGINAL timestamp's TTL horizon (1000 + 100 = 1100) but is
    // strictly less than the re-insert tick's TTL horizon
    // (1050 + 100 = 1150). If the timestamp had been overwritten to
    // `later_tick`, the entry would still be live at this sweep tick.
    let sweep_tick = TimerTick::new(original_tick.get() + ttl + 1);
    ring.sweep_expired(sweep_tick)
        .expect("sweep past original TTL must succeed");

    assert!(
        !ring.contains(&run),
        "entry must be evicted by the ORIGINAL timestamp's TTL horizon, \
         proving re-insert did not update the recorded timestamp"
    );
    assert_eq!(
        ring.counters().expired_evictions,
        1,
        "exactly one entry must have been evicted by the sweep"
    );
}
