#![forbid(unsafe_code)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "test-only lint overrides; production code in lru_ring.rs is unaffected"
)]
//! Adversarial property tests for `LruRing::remove` (vb-xfu6m slot-arena).
//!
//! Each test exercises the position-index + free-list invariants that the
//! O(1) remove path must maintain under specific failure modes the
//! red-queen pressure suite has already flagged in earlier beads:
//! target drops only, reinsert after remove, free-list reuse across
//! mixed insert/remove, idempotent remove-twice, capacity-overflow
//! recovery, and the FINDING-006+012 push_tail debug_assert contract
//! under repeated fill/drain cycles.

use crate::shard::lru_ring::LruRing;
use crate::shard::timer::TimerTick;
use vb_core::ids::RunId;

#[test]
fn lru_ring_property_remove_only_drops_target() {
    let mut ring: LruRing<RunId> = LruRing::try_new(8, u64::MAX).expect("non-zero test capacity");
    for i in 0..8 {
        ring.insert(RunId::new(i), TimerTick::new(0)).expect("fill");
    }
    let target = RunId::new(3);
    ring.remove(&target).expect("remove target must succeed");
    assert_eq!(ring.len(), 7);
    for i in 0..8 {
        let r = RunId::new(i);
        if r != target {
            assert!(ring.contains(&r), "sibling {r:?} must remain");
        } else {
            assert!(!ring.contains(&r), "target {r:?} must be gone");
        }
    }
}

#[test]
fn lru_ring_property_remove_reinsert_at_tail() {
    // After removing and re-inserting the same id, the ring should have the
    // correct membership and order. Re-insert of an existing id is a no-op
    // (idempotent), so we remove first.
    let mut ring: LruRing<RunId> = LruRing::try_new(4, u64::MAX).expect("non-zero test capacity");
    let a = RunId::new(1);
    let b = RunId::new(2);
    let c = RunId::new(3);

    ring.insert(a, TimerTick::new(0)).unwrap();
    ring.insert(b, TimerTick::new(0)).unwrap();
    ring.insert(c, TimerTick::new(0)).unwrap();

    ring.remove(&b).expect("remove b must succeed");
    assert!(!ring.contains(&b));
    assert!(ring.contains(&a));
    assert!(ring.contains(&c));
    assert_eq!(ring.len(), 2);

    // Insert b again at a later tick.
    ring.insert(b, TimerTick::new(10)).unwrap();
    assert!(ring.contains(&b));
    assert_eq!(ring.len(), 3);
}

#[test]
fn lru_ring_property_remove_uses_free_list_correctly() {
    // Insert 4, remove middle 2, re-insert — verifies that the free list
    // (slot reuse) doesn't corrupt the linked list.
    let mut ring: LruRing<RunId> = LruRing::try_new(4, u64::MAX).expect("non-zero test capacity");
    for i in 0..4 {
        ring.insert(RunId::new(i), TimerTick::new(0)).expect("fill");
    }
    ring.remove(&RunId::new(1))
        .expect("remove first must succeed");
    ring.remove(&RunId::new(2))
        .expect("remove second must succeed");
    assert_eq!(ring.len(), 2);
    assert!(ring.contains(&RunId::new(0)));
    assert!(ring.contains(&RunId::new(3)));

    // Re-insert should fill up to capacity 4.
    ring.insert(RunId::new(5), TimerTick::new(0))
        .expect("refill");
    ring.insert(RunId::new(6), TimerTick::new(0))
        .expect("refill");
    assert_eq!(ring.len(), 4);

    // Now at capacity; another insert should fail.
    let result = ring.insert(RunId::new(7), TimerTick::new(0));
    assert!(
        matches!(result, Err(crate::RuntimeError::TerminalRunsLruFull { .. })),
        "insert at capacity must return Err(TerminalRunsLruFull {{ .. }}), got {result:?}"
    );
}

/// Adversarial test for FINDING-006+012 push_tail debug_assert!.
///
/// Repeatedly fill, drain, refill a ring. Every remove pushes a slot onto
/// the free list; the next insert pops it. The push_tail debug_assert!
/// requires that every popped slot is `None` (free). If a future regression
/// breaks the free-list accounting (e.g. pushing a live slot back onto
/// free), this test panics with the new debug_assert message.
#[test]
fn lru_ring_property_push_tail_invariant_repeated_fill_drain_cycles() {
    // Try multiple capacities and id pool sizes to exercise all slot
    // positions in the free list.
    for (capacity, pool, cycles) in [(2usize, 4usize, 2000usize), (8, 16, 1000), (16, 32, 500)] {
        let mut ring: LruRing<RunId> =
            LruRing::try_new(capacity, u64::MAX).expect("non-zero test capacity");
        for cycle in 0..cycles {
            // Fill the ring with `capacity` unique ids.
            for i in 0..capacity {
                let id = RunId::new(((cycle * capacity + i) % pool) as u64 + 1);
                let _ = ring.insert(id, TimerTick::new(cycle as u64));
            }
            assert_eq!(
                ring.len(),
                capacity,
                "cycle {cycle}: ring should be full at capacity {capacity}"
            );
            // Drain it.
            for i in 0..capacity {
                let id = RunId::new(((cycle * capacity + i) % pool) as u64 + 1);
                ring.remove(&id)
                    .expect("repeated fill/drain remove must succeed");
            }
            assert!(
                ring.is_empty(),
                "cycle {cycle}: ring should be empty after drain"
            );
        }
    }
}

/// Adversarial test: remove-same-item-twice is a documented no-op and MUST
/// NOT push the same slot onto free twice. If it did, the next insert
/// would pop a slot whose arena entry is still Some (because we never
/// unlinked it the second time), triggering the push_tail debug_assert.
#[test]
fn lru_ring_property_remove_same_item_twice_is_safe() {
    let mut ring: LruRing<RunId> = LruRing::try_new(4, u64::MAX).expect("non-zero test capacity");
    ring.insert(RunId::new(1), TimerTick::new(0)).unwrap();
    ring.insert(RunId::new(2), TimerTick::new(0)).unwrap();
    ring.remove(&RunId::new(1))
        .expect("first remove must succeed");
    // Second remove of the same id: must be a no-op (item not in
    // position anymore).
    ring.remove(&RunId::new(1))
        .expect("second remove of absent id must succeed");
    assert_eq!(ring.len(), 1);
    // Reinsert 1 — slot 0 should be reused safely.
    ring.insert(RunId::new(1), TimerTick::new(10)).unwrap();
    assert_eq!(ring.len(), 2);
    assert!(ring.contains(&RunId::new(1)));
    assert!(ring.contains(&RunId::new(2)));
}

/// Adversarial test: capacity overflow path. fill → insert (full) → remove
/// → insert. The `TerminalRunsLruFull` error path must NOT leave the ring
/// in an inconsistent state. The next insert after a remove must succeed
/// and properly update the free list (this is the case that exercises the
/// push_tail debug_assert most aggressively).
#[test]
fn lru_ring_property_capacity_overflow_then_recover() {
    let mut ring: LruRing<RunId> = LruRing::try_new(3, u64::MAX).expect("non-zero test capacity");
    ring.insert(RunId::new(1), TimerTick::new(0)).unwrap();
    ring.insert(RunId::new(2), TimerTick::new(0)).unwrap();
    ring.insert(RunId::new(3), TimerTick::new(0)).unwrap();
    // At capacity.
    let r = ring.insert(RunId::new(4), TimerTick::new(0));
    assert!(
        matches!(r, Err(crate::RuntimeError::TerminalRunsLruFull { .. })),
        "capacity overflow must surface TerminalRunsLruFull, got {r:?}"
    );
    assert_eq!(ring.len(), 3);
    // Remove one and try again — must succeed.
    ring.remove(&RunId::new(2))
        .expect("recovery remove must succeed");
    ring.insert(RunId::new(5), TimerTick::new(0)).unwrap();
    assert_eq!(ring.len(), 3);
    assert!(ring.contains(&RunId::new(5)));
    assert!(!ring.contains(&RunId::new(2)));
    assert!(ring.contains(&RunId::new(1)));
    assert!(ring.contains(&RunId::new(3)));
}
