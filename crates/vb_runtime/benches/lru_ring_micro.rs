//! Microbench: `LruRing` membership-backend comparison.
//!
//! Compares two membership backends for the terminal-runs registry workload:
//!   - `IndexSet<T>`   (the representation used before commit 2551cd1d4)
//!   - `BTreeSet<T>`   (the representation after  commit 2551cd1d4)
//!
//! Both variants are defined LOCALLY in this file so the bench is
//! self-contained. They mirror the production logic in
//! `crates/vb_runtime/src/shard/lru_ring.rs` byte-for-byte: saturating
//! counters, idempotent `insert`, `force_insert` growth, the `u64 -> i128`
//! cutoff widening in `sweep_expired`, and `VecDeque::with_capacity` for the
//! order ring. The only differences are the membership-set type and the
//! per-backend removal primitive the production code used at the time:
//!   - IndexSet variant uses `swap_remove` (O(1)) — what the OLD code used.
//!   - BTreeSet variant uses `remove`      (O(log n)) — what the NEW code uses.
//!
//! No production code is imported; no production code is modified.
//!
//! # Workload characterization
//! - Workload: Terminal-runs registry operations on a bounded LRU ring.
//! - Hot path: `insert`, `contains`, `force_insert`, `sweep_expired`, `remove`.
//! - Capacity N: 100_000 (matches `DEFAULT_MAX_TERMINAL_RUNS`).
//! - TTL:        86_400  (matches `DEFAULT_TERMINAL_RUNS_TTL_TICKS`).
//! - Key type:   `RunId` (Copy + Eq + Ord + Hash newtype around u64).
//! - Target HW:  see the report; bench pins nothing CPU-specific.

// Bench targets are excluded from the strict source lint gate. We still keep
// the bench panic-free / unwrap-free / unsafe-free per task constraints, but we
// allow the ergonomic clippy lints that bench ergonomics legitimately use
// (arithmetic, indexing, casts, iteration over hash types, let_underscore).
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    clippy::iter_over_hash_type,
    clippy::let_underscore_must_use,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::cast_lossless,
    clippy::module_inception
)]
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::collections::VecDeque;

use criterion::BatchSize;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main};
use indexmap::IndexSet;
use std::hint::black_box;

// ============================================================================
// Local newtypes mirroring `vb_core::ids::RunId` and
// `vb_runtime::shard::timer::TimerTick` (both `Copy + Eq + Ord + Hash` newtypes
// around `u64`, `#[repr(transparent)]` for RunId).
// ============================================================================

/// Local mirror of `vb_core::ids::RunId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
struct RunId(u64);

impl RunId {
    const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Local mirror of `vb_runtime::shard::timer::TimerTick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TimerTick(u64);

impl TimerTick {
    const fn new(value: u64) -> Self {
        Self(value)
    }
    const fn get(self) -> u64 {
        self.0
    }
}

/// Local error mirror of `RuntimeError::TerminalRunsLruFull`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LruFullError {
    capacity: usize,
}

/// Diagnostic counters — byte-for-byte mirror of `LruRingCounters`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct LruRingCounters {
    expired_evictions: u64,
    capacity_overflows: u64,
}

// ============================================================================
// Workload constants (match production `DEFAULT_*`).
// ============================================================================

const CAPACITY: usize = 100_000;
const TTL_TICKS: u64 = 86_400;
/// Tick at which the "old half" (inserted at tick 0) is expired but the "young
/// half" (inserted at this tick) is alive. Used by `bench_sweep_expired`.
const SWEEP_NOW: u64 = TTL_TICKS;
/// Inner-loop batch sizes for batched benches.
const FORCE_INNER: usize = 4_096;
const REMOVE_INNER: usize = 4_096;

// ============================================================================
// IndexSet backend — mirrors the PRE-2551cd1d4 production code.
// ============================================================================

struct LruRingIndex {
    capacity: usize,
    ttl_ticks: u64,
    order: VecDeque<(RunId, TimerTick)>,
    members: IndexSet<RunId>,
    counters: LruRingCounters,
}

impl LruRingIndex {
    fn new(capacity: usize, ttl_ticks: u64) -> Self {
        let bounded_capacity = if capacity == 0 { 1 } else { capacity };
        Self {
            capacity: bounded_capacity,
            ttl_ticks,
            order: VecDeque::with_capacity(bounded_capacity),
            members: IndexSet::with_capacity(bounded_capacity),
            counters: LruRingCounters::default(),
        }
    }

    fn contains(&self, item: &RunId) -> bool {
        self.members.contains(item)
    }

    fn insert(&mut self, item: RunId, now: TimerTick) -> Result<(), LruFullError> {
        if self.members.contains(&item) {
            return Ok(());
        }
        self.sweep_expired(now);
        if self.members.len() >= self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
            return Err(LruFullError {
                capacity: self.capacity,
            });
        }
        self.order.push_back((item, now));
        self.members.insert(item);
        Ok(())
    }

    fn force_insert(&mut self, item: RunId, now: TimerTick) {
        if self.members.contains(&item) {
            return;
        }
        self.sweep_expired(now);
        let before = self.members.len();
        self.order.push_back((item, now));
        self.members.insert(item);
        if self.members.len() > before && self.members.len() > self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
        }
    }

    fn sweep_expired(&mut self, now: TimerTick) {
        if self.ttl_ticks == 0 {
            return;
        }
        // Production i128 cutoff widening — lossless `u64 -> i128` `From`.
        let cutoff: i128 = match now.get().checked_sub(self.ttl_ticks) {
            Some(value) => i128::from(value),
            None => -1,
        };
        while let Some(&(value, ts)) = self.order.front() {
            if i128::from(ts.get()) <= cutoff {
                self.order.pop_front();
                self.members.swap_remove(&value);
                self.counters.expired_evictions =
                    self.counters.expired_evictions.saturating_add(1);
            } else {
                break;
            }
        }
    }

    fn remove(&mut self, item: &RunId) {
        if self.members.swap_remove(item) {
            if let Some(position) = self.order.iter().position(|(value, _)| value == item) {
                self.order.remove(position);
            }
        }
    }
}

// ============================================================================
// BTreeSet backend — mirrors the POST-2551cd1d4 production code.
// ============================================================================

struct LruRingBTree {
    capacity: usize,
    ttl_ticks: u64,
    order: VecDeque<(RunId, TimerTick)>,
    members: BTreeSet<RunId>,
    counters: LruRingCounters,
}

impl LruRingBTree {
    fn new(capacity: usize, ttl_ticks: u64) -> Self {
        let bounded_capacity = if capacity == 0 { 1 } else { capacity };
        Self {
            capacity: bounded_capacity,
            ttl_ticks,
            order: VecDeque::with_capacity(bounded_capacity),
            members: BTreeSet::new(),
            counters: LruRingCounters::default(),
        }
    }

    fn contains(&self, item: &RunId) -> bool {
        self.members.contains(item)
    }

    fn insert(&mut self, item: RunId, now: TimerTick) -> Result<(), LruFullError> {
        if self.members.contains(&item) {
            return Ok(());
        }
        self.sweep_expired(now);
        if self.members.len() >= self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
            return Err(LruFullError {
                capacity: self.capacity,
            });
        }
        self.order.push_back((item, now));
        self.members.insert(item);
        Ok(())
    }

    fn force_insert(&mut self, item: RunId, now: TimerTick) {
        if self.members.contains(&item) {
            return;
        }
        self.sweep_expired(now);
        let before = self.members.len();
        self.order.push_back((item, now));
        self.members.insert(item);
        if self.members.len() > before && self.members.len() > self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
        }
    }

    fn sweep_expired(&mut self, now: TimerTick) {
        if self.ttl_ticks == 0 {
            return;
        }
        let cutoff: i128 = match now.get().checked_sub(self.ttl_ticks) {
            Some(value) => i128::from(value),
            None => -1,
        };
        while let Some(&(value, ts)) = self.order.front() {
            if i128::from(ts.get()) <= cutoff {
                self.order.pop_front();
                self.members.remove(&value);
                self.counters.expired_evictions =
                    self.counters.expired_evictions.saturating_add(1);
            } else {
                break;
            }
        }
    }

    fn remove(&mut self, item: &RunId) {
        if self.members.remove(item) {
            if let Some(position) = self.order.iter().position(|(value, _)| value == item) {
                self.order.remove(position);
            }
        }
    }
}

// ============================================================================
// Deterministic workload generators (no `rand` dependency; fully reproducible).
// ============================================================================

/// SplitMix64 PRNG step — deterministic, no deps.
#[inline]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(1_184_299_674_549_157_465);
    z = (z ^ (z >> 27)).wrapping_mul(4_297_025_584_627_948_071);
    z ^ (z >> 31)
}

/// Deterministic Fisher–Yates permutation of `0..n` (used so both backends are
/// exercised over identical access orders).
fn permutation(n: usize) -> Vec<RunId> {
    let mut v: Vec<RunId> = Vec::with_capacity(n);
    let mut k: u64 = 0;
    while (k as usize) < n {
        v.push(RunId::new(k));
        k = k.saturating_add(1);
    }
    let mut state: u64 = 0xDEAD_BEEF_CAFE_BABE;
    let mut i = n;
    while i > 1 {
        i -= 1;
        let j = (splitmix64(&mut state) >> 33) as usize % (i + 1);
        v.swap(i, j);
    }
    v
}

/// Fills an `LruRingIndex` with `count` fresh entries at tick `now`.
fn fill_index(ring: &mut LruRingIndex, keys: &[RunId], now: TimerTick) {
    for &k in keys {
        match ring.insert(k, now) {
            Ok(()) => {}
            Err(_) => {}
        }
    }
}

/// Fills an `LruRingBTree` with `count` fresh entries at tick `now`.
fn fill_btree(ring: &mut LruRingBTree, keys: &[RunId], now: TimerTick) {
    for &k in keys {
        match ring.insert(k, now) {
            Ok(()) => {}
            Err(_) => {}
        }
    }
}

/// Builds a full `LruRingIndex` at `CAPACITY` entries, all alive at `now`.
fn full_index(now: TimerTick) -> LruRingIndex {
    let mut ring = LruRingIndex::new(CAPACITY, TTL_TICKS);
    let keys = permutation(CAPACITY);
    fill_index(&mut ring, &keys, now);
    ring
}

/// Builds a full `LruRingBTree` at `CAPACITY` entries, all alive at `now`.
fn full_btree(now: TimerTick) -> LruRingBTree {
    let mut ring = LruRingBTree::new(CAPACITY, TTL_TICKS);
    let keys = permutation(CAPACITY);
    fill_btree(&mut ring, &keys, now);
    ring
}

/// Builds a `LruRingIndex` with `CAPACITY` entries: the first half inserted at
/// tick 0 (expired at `SWEEP_NOW`), the second half at tick `SWEEP_NOW` (alive).
///
/// NOTE: we populate `order`/`members` DIRECTLY rather than via `insert`,
/// because every `insert(now)` internally calls `sweep_expired(now)` — so a
/// naive young-half `insert(SWEEP_NOW)` would evict the old half during
/// construction and leave a ring with nothing to sweep. Direct field access is
/// legitimate here because these are local bench types, and it gives a true
/// 50%-expired starting state for the sweep measurement.
fn half_expired_index() -> LruRingIndex {
    let mut ring = LruRingIndex::new(CAPACITY, TTL_TICKS);
    let half = CAPACITY / 2;
    let keys = permutation(CAPACITY);
    let mut i = 0;
    while i < half {
        let k = keys[i];
        ring.order.push_back((k, TimerTick::new(0)));
        let _ = ring.members.insert(k);
        i += 1;
    }
    while i < CAPACITY {
        let k = keys[i];
        ring.order.push_back((k, TimerTick::new(SWEEP_NOW)));
        let _ = ring.members.insert(k);
        i += 1;
    }
    ring
}

/// Builds a `LruRingBTree` with the same half-expired shape as
/// `half_expired_index` (direct population, see note above).
fn half_expired_btree() -> LruRingBTree {
    let mut ring = LruRingBTree::new(CAPACITY, TTL_TICKS);
    let half = CAPACITY / 2;
    let keys = permutation(CAPACITY);
    let mut i = 0;
    while i < half {
        let k = keys[i];
        ring.order.push_back((k, TimerTick::new(0)));
        let _ = ring.members.insert(k);
        i += 1;
    }
    while i < CAPACITY {
        let k = keys[i];
        ring.order.push_back((k, TimerTick::new(SWEEP_NOW)));
        let _ = ring.members.insert(k);
        i += 1;
    }
    ring
}

// ============================================================================
// Benchmarks
// ============================================================================

/// Insert N fresh entries into a pre-sized empty ring (includes growth allocs).
/// Setup = fresh empty ring; timed work = N inserts.
fn bench_insert(c: &mut Criterion) {
    let now = TimerTick::new(SWEEP_NOW);
    let keys = permutation(CAPACITY);
    let mut group = c.benchmark_group("insert");
    group.throughput(Throughput::Elements(CAPACITY as u64));
    group.sample_size(20);
    group.bench_function(BenchmarkId::new("IndexSet", CAPACITY), |b| {
        b.iter_batched(
            || LruRingIndex::new(CAPACITY, TTL_TICKS),
            |mut ring| {
                for &k in &keys {
                    match ring.insert(k, now) {
                        Ok(()) => {}
                        Err(_) => {}
                    }
                }
                black_box(&ring);
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function(BenchmarkId::new("BTreeSet", CAPACITY), |b| {
        b.iter_batched(
            || LruRingBTree::new(CAPACITY, TTL_TICKS),
            |mut ring| {
                for &k in &keys {
                    match ring.insert(k, now) {
                        Ok(()) => {}
                        Err(_) => {}
                    }
                }
                black_box(&ring);
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// `contains()` on existing entries, scrambled lookup order.
fn bench_contains_hit(c: &mut Criterion) {
    let ring_index = full_index(TimerTick::new(SWEEP_NOW));
    let ring_btree = full_btree(TimerTick::new(SWEEP_NOW));
    let lookups = permutation(CAPACITY); // distinct scrambled subset of [0,CAPACITY)
    let mut group = c.benchmark_group("contains_hit");
    group.throughput(Throughput::Elements(CAPACITY as u64));
    group.bench_function(BenchmarkId::new("IndexSet", CAPACITY), |b| {
        b.iter(|| {
            let mut hits = 0u64;
            for &k in &lookups {
                if ring_index.contains(black_box(&k)) {
                    hits = hits.saturating_add(1);
                }
            }
            black_box(hits);
        });
    });
    group.bench_function(BenchmarkId::new("BTreeSet", CAPACITY), |b| {
        b.iter(|| {
            let mut hits = 0u64;
            for &k in &lookups {
                if ring_btree.contains(black_box(&k)) {
                    hits = hits.saturating_add(1);
                }
            }
            black_box(hits);
        });
    });
    group.finish();
}

/// `contains()` on absent entries (keys outside the populated range).
fn bench_contains_miss(c: &mut Criterion) {
    let ring_index = full_index(TimerTick::new(SWEEP_NOW));
    let ring_btree = full_btree(TimerTick::new(SWEEP_NOW));
    // Scrambled miss keys: offset above the populated range.
    let mut miss = permutation(CAPACITY);
    for k in &mut miss {
        *k = RunId::new(k.0.wrapping_add(CAPACITY as u64));
    }
    let mut group = c.benchmark_group("contains_miss");
    group.throughput(Throughput::Elements(CAPACITY as u64));
    group.bench_function(BenchmarkId::new("IndexSet", CAPACITY), |b| {
        b.iter(|| {
            let mut hits = 0u64;
            for &k in &miss {
                if ring_index.contains(black_box(&k)) {
                    hits = hits.saturating_add(1);
                }
            }
            black_box(hits);
        });
    });
    group.bench_function(BenchmarkId::new("BTreeSet", CAPACITY), |b| {
        b.iter(|| {
            let mut hits = 0u64;
            for &k in &miss {
                if ring_btree.contains(black_box(&k)) {
                    hits = hits.saturating_add(1);
                }
            }
            black_box(hits);
        });
    });
    group.finish();
}

/// `force_insert()` while the ring is at capacity — the "bounded with
/// observable counter" path used by `Shard::terminal_runs_insert`.
fn bench_force_insert_full(c: &mut Criterion) {
    let now = TimerTick::new(SWEEP_NOW);
    let extra = permutation(FORCE_INNER * 2);
    let mut group = c.benchmark_group("force_insert_full");
    group.throughput(Throughput::Elements(FORCE_INNER as u64));
    group.bench_function(BenchmarkId::new("IndexSet", CAPACITY), |b| {
        b.iter_batched_ref(
            || full_index(now),
            |ring| {
                for &k in &extra[..FORCE_INNER] {
                    ring.force_insert(k, now);
                }
                black_box(&ring.counters);
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function(BenchmarkId::new("BTreeSet", CAPACITY), |b| {
        b.iter_batched_ref(
            || full_btree(now),
            |ring| {
                for &k in &extra[..FORCE_INNER] {
                    ring.force_insert(k, now);
                }
                black_box(&ring.counters);
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// `sweep_expired()` over a ring at 50% expired (CAPACITY/2 entries expire).
/// Rebuilt per measurement so every sample observes the full sweep.
fn bench_sweep_expired(c: &mut Criterion) {
    let now = TimerTick::new(SWEEP_NOW);
    let half = (CAPACITY / 2) as u64;
    let mut group = c.benchmark_group("sweep_expired_50pct");
    group.throughput(Throughput::Elements(half));
    group.sample_size(20);
    group.bench_function(BenchmarkId::new("IndexSet", CAPACITY), |b| {
        b.iter_batched(
            half_expired_index,
            |mut ring| {
                ring.sweep_expired(now);
                black_box(&ring.counters);
            },
            BatchSize::PerIteration,
        );
    });
    group.bench_function(BenchmarkId::new("BTreeSet", CAPACITY), |b| {
        b.iter_batched(
            half_expired_btree,
            |mut ring| {
                ring.sweep_expired(now);
                black_box(&ring.counters);
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

/// `remove()` of present entries — includes the O(n) `order` rescan/shift that
/// dominates both backends.
fn bench_remove(c: &mut Criterion) {
    let targets = permutation(REMOVE_INNER * 2);
    let mut group = c.benchmark_group("remove");
    group.throughput(Throughput::Elements(REMOVE_INNER as u64));
    group.bench_function(BenchmarkId::new("IndexSet", CAPACITY), |b| {
        b.iter_batched_ref(
            || full_index(TimerTick::new(SWEEP_NOW)),
            |ring| {
                for &k in &targets[..REMOVE_INNER] {
                    ring.remove(&k);
                }
                black_box(&ring.counters);
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function(BenchmarkId::new("BTreeSet", CAPACITY), |b| {
        b.iter_batched_ref(
            || full_btree(TimerTick::new(SWEEP_NOW)),
            |ring| {
                for &k in &targets[..REMOVE_INNER] {
                    ring.remove(&k);
                }
                black_box(&ring.counters);
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_contains_hit,
    bench_contains_miss,
    bench_force_insert_full,
    bench_sweep_expired,
    bench_remove,
);
criterion_main!(benches);
