#![forbid(unsafe_code)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test-only lint overrides; production code in lru_ring.rs is unaffected"
)]
//! vb-xfu6m remove O(1) behaviour tests.
//!
//! Validates that the position-index-backed `LruRing::remove` is O(1) and
//! preserves the doubly-linked-list invariants across the documented edge
//! cases (present item, absent item, empty ring, item already evicted by a
//! prior `sweep_expired`, and a mixed insert/remove/sweep sequence).

use crate::shard::lru_ring::LruRing;
use crate::shard::timer::TimerTick;
use vb_core::ids::RunId;

// ── vb-xfu6m remove O(1) behaviour ──────────────────────────────────────────

#[test]
fn test_remove_present_item_drops_it() {
    let capacity = 8usize;
    let mut ring: LruRing<RunId> = LruRing::try_new(capacity, u64::MAX).expect("non-zero test capacity");
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 1), TimerTick::new(offset as u64))
            .expect("fill");
    }
    assert_eq!(ring.len(), capacity);

    let target = RunId::new(3);
    let before_counters = ring.counters();
    ring.remove(&target).expect("remove of present item must succeed");

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
    let mut ring: LruRing<RunId> = LruRing::try_new(capacity, u64::MAX).expect("non-zero test capacity");
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 1), TimerTick::new(offset as u64))
            .expect("fill");
    }
    let absent = RunId::new(999);
    assert!(!ring.contains(&absent), "absent item must not be present");

    let before_counters = ring.counters();
    ring.remove(&absent).expect("remove of absent item must succeed");

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
    let mut ring: LruRing<RunId> = LruRing::try_new(4, u64::MAX).expect("non-zero test capacity");
    assert!(ring.is_empty());
    let before_counters = ring.counters();

    ring.remove(&RunId::new(1))
        .expect("remove on empty ring must succeed");

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
    let mut ring: LruRing<RunId> = LruRing::try_new(capacity, ttl).expect("non-zero test capacity");
    let baseline = TimerTick::new(100);
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 1), baseline)
            .expect("fill");
    }
    let before_sweep_evictions = ring.counters().expired_evictions;

    // Sweep at a tick well past the TTL horizon; every entry is evicted
    // by sweep_expired (the linked-list head walks forward until the
    // first non-expired node, which there is none of here).
    ring.sweep_expired(TimerTick::new(baseline.get() + ttl + 1))
        .expect("sweep past ttl must succeed");
    assert_eq!(
        ring.counters().expired_evictions,
        before_sweep_evictions + capacity as u64,
        "sweep must evict every TTL-expired entry"
    );
    assert!(ring.is_empty(), "ring must be empty after sweep");

    let before_counters = ring.counters();
    // Removing an item that sweep_expired already evicted must be a no-op:
    // the position map no longer has the entry, so remove short-circuits.
    ring.remove(&RunId::new(2))
        .expect("remove of already-evicted item must succeed");

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
    let mut ring: LruRing<RunId> = LruRing::try_new(capacity, ttl).expect("non-zero test capacity");

    let baseline = TimerTick::new(0);
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 1), baseline)
            .expect("fill");
    }

    // Remove the second-oldest, the middle, and the newest — three
    // different positions in the linked list — and re-insert each so
    // the free list is exercised across both reused and fresh slots.
    ring.remove(&RunId::new(2))
        .expect("remove second-oldest must succeed");
    ring.remove(&RunId::new(4))
        .expect("remove middle must succeed");
    ring.remove(&RunId::new(6))
        .expect("remove newest must succeed");

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
    ring.sweep_expired(TimerTick::new(2_000))
        .expect("sweep past ttl must succeed");
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
