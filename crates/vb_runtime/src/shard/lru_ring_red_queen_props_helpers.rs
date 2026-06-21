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
//! Test-only helpers shared by `lru_ring_red_queen_*_props` modules.
//!
//! Owns the deterministic LCG, the synthetic `Op` enum, and the
//! `run_with_invariants` ground-truth simulator that the red-queen
//! pressure tests rely on. Kept in its own module so the per-property
//! test files stay under the 300-line architectural-drift cap.

use crate::shard::lru_ring::LruRing;
use crate::shard::timer::TimerTick;
use vb_core::ids::RunId;

/// Linear congruential generator — deterministic, seedable.
pub(super) struct Lcg {
    state: u64,
}

impl Lcg {
    pub(super) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub(super) fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005_u64)
            .wrapping_add(1_442_695_040_888_963_407_u64);
        self.state
    }

    pub(super) fn next_usize(&mut self, range: usize) -> usize {
        if range == 0 {
            return 0;
        }
        (self.next_u64() >> 32) as usize % range
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Op {
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
pub(super) fn run_with_invariants(seed: u64, ops: &[Op], capacity: usize, ttl: u64) {
    let mut ring: LruRing<RunId> = LruRing::try_new(capacity, ttl).expect("non-zero test capacity");

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
