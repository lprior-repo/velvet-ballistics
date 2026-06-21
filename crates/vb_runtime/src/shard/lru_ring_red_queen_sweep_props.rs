#![forbid(unsafe_code)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test-only lint overrides; production code in lru_ring.rs is unaffected"
)]
//! Adversarial property tests for `LruRing::sweep_expired` and clear.
//!
//! These tests target the doubly-linked-list walk path that the
//! `sweep_expired` function exercises (advance head until the first
//! non-expired node, push every popped slot onto the free list), plus
//! the `clear` reset path. They cover both the simple TTL horizon sweep
//! and the more aggressive sweep-then-insert pattern that the
//! FINDING-006+012 push_tail debug_assert originally surfaced.

use crate::shard::lru_ring::LruRing;
use crate::shard::timer::TimerTick;
use vb_core::ids::RunId;

#[test]
fn lru_ring_property_sweep_expired_evicts_only_expired() {
    // Insert 5 items at t=0 (capacity 10, ttl 100), then sweep at t=50.
    // No items should be evicted (50 < 100).
    let mut ring: LruRing<RunId> = LruRing::try_new(10, 100).expect("non-zero test capacity");
    for i in 0..5 {
        ring.insert(RunId::new(i), TimerTick::new(0))
            .expect("first wave");
    }
    ring.sweep_expired(TimerTick::new(50))
        .expect("sweep before ttl must succeed");
    assert_eq!(ring.len(), 5, "no items expired at t=50 (ttl=100)");
    assert_eq!(ring.counters().expired_evictions, 0);

    // Now sweep at t=200. All 5 items should be evicted.
    ring.sweep_expired(TimerTick::new(200))
        .expect("sweep past ttl must succeed");
    assert!(ring.is_empty(), "all items expired at t=200 (ttl=100)");
    assert_eq!(ring.counters().expired_evictions, 5);
}

#[test]
fn lru_ring_property_clear_resets_state() {
    let mut ring: LruRing<RunId> = LruRing::try_new(8, u64::MAX).expect("non-zero test capacity");
    for i in 0..8 {
        ring.insert(RunId::new(i), TimerTick::new(0)).expect("fill");
    }
    ring.clear();
    assert!(ring.is_empty());
    assert_eq!(ring.len(), 0);

    // After clear, capacity and ttl remain unchanged.
    assert_eq!(ring.capacity(), 8);
    assert_eq!(ring.ttl_ticks(), u64::MAX);

    // Re-insert after clear works.
    for i in 0..8 {
        ring.insert(RunId::new(i + 100), TimerTick::new(0))
            .expect("re-fill");
    }
    assert_eq!(ring.len(), 8);
}

/// Adversarial test: aggressive TTL sweep pattern — rapidly advancing time
/// causes repeated eviction. The sweep_expired path also pushes slots onto
/// the free list. This stresses the push_tail debug_assert through a
/// different code path than remove().
#[test]
fn lru_ring_property_sweep_expired_then_insert_does_not_fire_invariant() {
    let mut ring: LruRing<RunId> = LruRing::try_new(8, 10).expect("non-zero test capacity");
    for i in 0..8u64 {
        ring.insert(RunId::new(i + 1), TimerTick::new(i)).unwrap();
    }
    // Sweep at t=20 — all 8 items are expired (ts + 10 <= 20).
    ring.sweep_expired(TimerTick::new(20))
        .expect("first sweep past ttl must succeed");
    assert!(ring.is_empty(), "all items should be expired");
    // Re-insert — all slots should be on the free list.
    for i in 0..8u64 {
        ring.insert(RunId::new(i + 100), TimerTick::new(100 + i))
            .unwrap();
    }
    assert_eq!(ring.len(), 8);
    // Now sweep at t=200 — all expire again.
    ring.sweep_expired(TimerTick::new(200))
        .expect("second sweep past ttl must succeed");
    assert!(ring.is_empty());
}
