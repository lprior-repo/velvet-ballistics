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
//! Combined-op adversarial property tests for `LruRing` (vb-xfu6m).
//!
//! These tests interleave insert / remove / sweep / clear / force_insert
//! against a ground-truth model maintained by
//! `super::lru_ring_red_queen_props_helpers::run_with_invariants`.
//! Validates that the slot-arena invariants hold under arbitrary operation
//! sequences:
//!
//! 1. `position` map and the doubly-linked list always agree on membership.
//! 2. `head` and `tail` always point to live slots when `len() > 0`.
//! 3. `len() == position.len()`.
//! 4. `sweep_expired` evicts only expired items, preserving insertion order
//!    for survivors.
//! 5. `remove` is O(1) and never corrupts the linked list.
//!
//! All seeds are deterministic (Numerical Recipes LCG) so failures can be
//! reproduced.

use super::lru_ring_red_queen_props_helpers::{Lcg, Op, run_with_invariants};
use crate::shard::lru_ring::LruRing;
use crate::shard::timer::TimerTick;
use vb_core::ids::RunId;

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
        let mut ring: LruRing<RunId> = LruRing::try_new(8, 10).expect("non-zero test capacity");
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
