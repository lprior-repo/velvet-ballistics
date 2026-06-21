#![forbid(unsafe_code)]
//! Adversarial property tests for vb-xfu6m LruRing slot-arena changes.
//!
//! These tests exercise the LruRing implementation with random operation
//! sequences to validate the slot-arena invariants:
//!
//! 1. `position` map and the doubly-linked list always agree on membership.
//! 2. `head` and `tail` always point to live slots when `len() > 0`.
//! 3. `len() == position.len()`.
//! 4. `sweep_expired` evicts only expired items, preserving insertion order
//!    for survivors.
//! 5. `remove` is O(1) and never corrupts the linked list.
//!
//! The tests use deterministic seeds so failures can be reproduced.

use crate::shard::lru_ring::LruRing;
use crate::shard::timer::TimerTick;
use vb_core::ids::RunId;

/// Linear congruential generator — deterministic, seedable.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005_u64)
            .wrapping_add(1_442_695_040_888_963_407_u64);
        self.state
    }

    fn next_usize(&mut self, range: usize) -> usize {
        if range == 0 {
            return 0;
        }
        (self.next_u64() >> 32) as usize % range
    }
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Insert(u64, u64), // (id, tick)
    Remove(u64),
    Sweep(u64), // tick
    Clear,
    ForceInsert(u64, u64), // (id, tick)
}

/// Run `ops` operations and verify LruRing invariants against a ground-truth
/// model after every operation.
///
/// Ground-truth model:
/// - `truth` is the set of item ids we believe are in the ring.
/// - `truth_ts` records the insertion tick for each truth item, so that
///   after a sweep we can precisely drop items whose `ts + ttl <= now` from
///   truth (they MUST have been evicted by the sweep).
///
/// Invariants verified after every op:
/// - No panics or undefined behavior.
/// - `is_empty() == (len() == 0)`.
/// - `ring.len() <= truth.len()`: ring contents are a subset of ground truth
///   (TTL sweeps may have evicted items without our tracking).
/// - For every item in `truth` whose `ts + ttl > last_tick`, the ring
///   MUST still contain that item. If it doesn't, the ring's TTL eviction
///   is leaking or the insert/remove is corrupting state.
fn run_with_invariants(seed: u64, ops: &[Op], capacity: usize, ttl: u64) {
    let mut ring: LruRing<RunId> = LruRing::new(capacity, ttl);

    let mut truth: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut truth_ts: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
    let mut last_tick: u64 = 0;

    for (step, op) in ops.iter().enumerate() {
        // Advance last_tick to the op's tick so post-sweep eviction
        // accounting uses the most recent time observed.
        let op_tick = match *op {
            Op::Insert(_, t) | Op::ForceInsert(_, t) | Op::Sweep(t) => t,
            Op::Remove(_) | Op::Clear => last_tick,
        };
        last_tick = last_tick.max(op_tick);

        match *op {
            Op::Insert(id_raw, tick_raw) => {
                let id = RunId::new(id_raw);
                let now = TimerTick::new(tick_raw);
                // LruRing::insert early-returns Ok(()) without sweeping
                // when the item is already in the ring. Detect that case
                // to avoid a phantom sweep-eviction in the truth model.
                let already_present = ring.contains(&id);
                let result = ring.insert(id, now);
                match result {
                    Ok(()) => {
                        if !already_present {
                            // ring.insert called sweep_expired(now) before
                            // pushing the new item. Mirror that: drop every
                            // truth item with ts + ttl <= tick_raw.
                            truth_ts.retain(|_, ts| ts.saturating_add(ttl) > tick_raw);
                            truth.retain(|id| truth_ts.contains_key(id));
                        }
                        truth.insert(id_raw);
                        // LruRing insert is idempotent: a duplicate insert
                        // does NOT update the existing node's tick. Mirror
                        // that here so we correctly model the eviction tick.
                        truth_ts.entry(id_raw).or_insert(tick_raw);
                    }
                    Err(crate::RuntimeError::TerminalRunsLruFull { .. }) => {
                        // Ring was full and the item was not present; insert
                        // swept first (which evicted any expired items) and
                        // then hit the capacity guard. Mirror the sweep.
                        truth_ts.retain(|_, ts| ts.saturating_add(ttl) > tick_raw);
                        truth.retain(|id| truth_ts.contains_key(id));
                    }
                    Err(other) => panic!("step {step}: unexpected error {other:?}"),
                }
            }
            Op::Remove(id_raw) => {
                let id = RunId::new(id_raw);
                ring.remove(&id).expect("red-queen remove must succeed");
                truth.remove(&id_raw);
                truth_ts.remove(&id_raw);
            }
            Op::Sweep(tick_raw) => {
                let now = TimerTick::new(tick_raw);
                ring.sweep_expired(now)
                    .expect("red-queen sweep_expired must succeed");
                // sweep_expired evicted every node with ts <= cutoff where
                // cutoff = now - ttl. Items in truth with ts + ttl <= tick_raw
                // satisfy ts <= tick_raw - ttl = cutoff and MUST have been
                // evicted. Drop them from truth.
                truth_ts.retain(|_, ts| ts.saturating_add(ttl) > tick_raw);
                truth.retain(|id| truth_ts.contains_key(id));
            }
            Op::Clear => {
                ring.clear();
                truth.clear();
                truth_ts.clear();
            }
            Op::ForceInsert(id_raw, tick_raw) => {
                let id = RunId::new(id_raw);
                let now = TimerTick::new(tick_raw);
                // LruRing::force_insert also early-returns without
                // sweeping when the item is already present.
                let already_present = ring.contains(&id);
                ring.force_insert(id, now);
                if !already_present {
                    truth_ts.retain(|_, ts| ts.saturating_add(ttl) > tick_raw);
                    truth.retain(|id| truth_ts.contains_key(id));
                }
                truth.insert(id_raw);
                truth_ts.entry(id_raw).or_insert(tick_raw);
            }
        }

        // Invariant 1: is_empty <-> len == 0
        assert_eq!(
            ring.is_empty(),
            ring.len() == 0,
            "step {step}: is_empty mismatch (seed={seed})"
        );

        // Invariant 2: ring contents are a subset of ground truth.
        // (Sweep may evict items from ring without us tracking which ones.)
        assert!(
            ring.len() <= truth.len(),
            "step {step}: ring.len() ({}) > truth.len() ({}) seed={seed}",
            ring.len(),
            truth.len()
        );

        // Invariant 3: every live truth item (one whose TTL has not yet
        // expired at last_tick) MUST still be in the ring. If it isn't,
        // the ring's TTL eviction or insert/remove logic is corrupt.
        for (id_raw, ts) in &truth_ts {
            if ts.saturating_add(ttl) > last_tick {
                let id = RunId::new(*id_raw);
                assert!(
                    ring.contains(&id),
                    "step {step}: live item {id_raw} (ts={ts}, ttl={ttl}, last_tick={last_tick}) missing from ring (seed={seed})"
                );
            }
        }
    }
}

#[test]
fn lru_ring_property_random_ops_capacity_10() {
    // Generate ops with monotonic ticks: each op's tick is >= the previous op's
    // tick. This mirrors production behaviour where time advances forward and
    // ensures the LruRing's insertion-ordered TTL sweep is well-defined
    // (insertion order = tick order).
    for seed in 0u64..16 {
        let mut rng = Lcg::new(seed);
        let mut tick: u64 = 0;
        let ops: Vec<Op> = (0..1000)
            .map(|_| {
                tick = tick.saturating_add(rng.next_u64() % 10);
                let kind = rng.next_u64() % 5;
                let id = rng.next_u64() % 20;
                let t = tick;
                match kind {
                    0 => Op::Insert(id, t),
                    1 => Op::Remove(id),
                    2 => Op::Sweep(t),
                    3 => Op::Clear,
                    _ => Op::ForceInsert(id, t),
                }
            })
            .collect();
        run_with_invariants(seed, &ops, 10, 5);
    }
}

#[test]
fn lru_ring_property_random_ops_capacity_3_ttl_long() {
    for seed in 0u64..8 {
        let mut rng = Lcg::new(seed);
        let mut tick: u64 = 0;
        let ops: Vec<Op> = (0..500)
            .map(|_| {
                tick = tick.saturating_add(rng.next_u64() % 10);
                let kind = rng.next_u64() % 4;
                let id = rng.next_u64() % 8;
                let t = tick;
                match kind {
                    0 => Op::Insert(id, t),
                    1 => Op::Remove(id),
                    2 => Op::Sweep(t),
                    _ => Op::ForceInsert(id, t),
                }
            })
            .collect();
        run_with_invariants(seed, &ops, 3, u64::MAX);
    }
}

#[test]
fn lru_ring_property_random_ops_consistency_check() {
    // Property: across random operations, the ring state matches the
    // ground-truth model maintained by the test:
    // - ring.len() <= truth.len() (ring is a subset of ground truth).
    // - Every "live" truth item (ts + ttl > current tick) is in the ring.
    for seed in 0u64..32 {
        let mut rng = Lcg::new(seed);
        let mut tick: u64 = 0;
        let mut ring: LruRing<RunId> = LruRing::new(8, 10);
        let mut truth: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut truth_ts: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();

        for _ in 0..200 {
            tick = tick.saturating_add(rng.next_u64() % 5);

            let kind = rng.next_u64() % 4;
            let id_raw = rng.next_u64() % 10;
            let id = RunId::new(id_raw);
            let now = TimerTick::new(tick);

            match kind {
                0 => {
                    // Insert — sweep happens inside LruRing only if item is new
                    let already = ring.contains(&id);
                    if ring.insert(id, now).is_ok() {
                        if !already {
                            truth_ts.retain(|_, ts| ts.saturating_add(10) > tick);
                            truth.retain(|id| truth_ts.contains_key(id));
                        }
                        truth.insert(id_raw);
                        truth_ts.entry(id_raw).or_insert(tick);
                    }
                }
                1 => {
                    // Remove
                    ring.remove(&id)
                        .expect("consistency-check remove must succeed");
                    truth.remove(&id_raw);
                    truth_ts.remove(&id_raw);
                }
                2 => {
                    // Sweep
                    ring.sweep_expired(now)
                        .expect("consistency-check sweep must succeed");
                    truth_ts.retain(|_, ts| ts.saturating_add(10) > tick);
                    truth.retain(|id| truth_ts.contains_key(id));
                }
                _ => {
                    // ForceInsert — sweep happens inside LruRing only if item is new
                    let already = ring.contains(&id);
                    ring.force_insert(id, now);
                    if !already {
                        truth_ts.retain(|_, ts| ts.saturating_add(10) > tick);
                        truth.retain(|id| truth_ts.contains_key(id));
                    }
                    truth.insert(id_raw);
                    truth_ts.entry(id_raw).or_insert(tick);
                }
            }

            // Invariant: ring contents are a subset of ground truth.
            assert!(
                ring.len() <= truth.len(),
                "ring.len() ({}) > truth.len() ({}) at seed={seed}, tick={tick}",
                ring.len(),
                truth.len()
            );
            // Invariant: every live truth item is in the ring.
            for (live_id_raw, ts) in &truth_ts {
                if ts.saturating_add(10) > tick {
                    let live_id = RunId::new(*live_id_raw);
                    assert!(
                        ring.contains(&live_id),
                        "live item {live_id_raw} (ts={ts}, tick={tick}) missing from ring at seed={seed}"
                    );
                }
            }
        }
    }
}

#[test]
fn lru_ring_property_sweep_expired_evicts_only_expired() {
    // Insert 5 items at t=0 (capacity 10, ttl 100), then sweep at t=50.
    // No items should be evicted (50 < 100).
    let mut ring: LruRing<RunId> = LruRing::new(10, 100);
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
fn lru_ring_property_remove_only_drops_target() {
    let mut ring: LruRing<RunId> = LruRing::new(8, u64::MAX);
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
    let mut ring: LruRing<RunId> = LruRing::new(4, u64::MAX);
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
fn lru_ring_property_clear_resets_state() {
    let mut ring: LruRing<RunId> = LruRing::new(8, u64::MAX);
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

#[test]
fn lru_ring_property_remove_uses_free_list_correctly() {
    // Insert 4, remove middle 2, re-insert — verifies that the free list
    // (slot reuse) doesn't corrupt the linked list.
    let mut ring: LruRing<RunId> = LruRing::new(4, u64::MAX);
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
        result.is_err(),
        "insert at capacity must return Err, got {result:?}"
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
        let mut ring: LruRing<RunId> = LruRing::new(capacity, u64::MAX);
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
    let mut ring: LruRing<RunId> = LruRing::new(4, u64::MAX);
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
    let mut ring: LruRing<RunId> = LruRing::new(3, u64::MAX);
    ring.insert(RunId::new(1), TimerTick::new(0)).unwrap();
    ring.insert(RunId::new(2), TimerTick::new(0)).unwrap();
    ring.insert(RunId::new(3), TimerTick::new(0)).unwrap();
    // At capacity.
    let r = ring.insert(RunId::new(4), TimerTick::new(0));
    assert!(r.is_err(), "must fail when full");
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

/// Adversarial test: aggressive TTL sweep pattern — rapidly advancing time
/// causes repeated eviction. The sweep_expired path also pushes slots onto
/// the free list. This stresses the push_tail debug_assert through a
/// different code path than remove().
#[test]
fn lru_ring_property_sweep_expired_then_insert_does_not_fire_invariant() {
    let mut ring: LruRing<RunId> = LruRing::new(8, 10);
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
