use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};

use super::backends::{LruRingBTree, LruRingIndex};
use super::types::{CAPACITY, FORCE_INNER, REMOVE_INNER, RunId, SWEEP_NOW, TTL_TICKS, TimerTick};
use super::workloads::{
    full_btree, full_index, half_expired_btree, half_expired_index, permutation,
};

/// Insert N fresh entries into a pre-sized empty ring.
pub(crate) fn bench_insert(c: &mut Criterion) {
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
pub(crate) fn bench_contains_hit(c: &mut Criterion) {
    let ring_index = full_index(TimerTick::new(SWEEP_NOW));
    let ring_btree = full_btree(TimerTick::new(SWEEP_NOW));
    let lookups = permutation(CAPACITY);
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

/// `contains()` on absent entries.
pub(crate) fn bench_contains_miss(c: &mut Criterion) {
    let ring_index = full_index(TimerTick::new(SWEEP_NOW));
    let ring_btree = full_btree(TimerTick::new(SWEEP_NOW));
    let mut miss = permutation(CAPACITY);
    for k in &mut miss {
        *k = RunId::new(k.get().wrapping_add(CAPACITY as u64));
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

/// `force_insert()` while the ring is at capacity.
pub(crate) fn bench_force_insert_full(c: &mut Criterion) {
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
                black_box(ring.counters());
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
                black_box(ring.counters());
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// `sweep_expired()` over a ring at 50% expired.
pub(crate) fn bench_sweep_expired(c: &mut Criterion) {
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
                black_box(ring.counters());
            },
            BatchSize::PerIteration,
        );
    });
    group.bench_function(BenchmarkId::new("BTreeSet", CAPACITY), |b| {
        b.iter_batched(
            half_expired_btree,
            |mut ring| {
                ring.sweep_expired(now);
                black_box(ring.counters());
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

/// `remove()` of present entries.
pub(crate) fn bench_remove(c: &mut Criterion) {
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
                black_box(ring.counters());
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
                black_box(ring.counters());
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}
