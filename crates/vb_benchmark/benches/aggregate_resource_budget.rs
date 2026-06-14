//! Criterion benchmark for [`vb_benchmark::aggregate_resource_budget`].
//!
//! Folds a fixed synthetic workload of 1,000 [`RunMetrics`] through the
//! aggregator and reports the per-iteration median wall-clock time. The input
//! vector is pre-built outside the timed region so the measurement reflects the
//! aggregator alone, not the synthetic workload generator.

#![forbid(unsafe_code)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use vb_benchmark::aggregate_resource_budget::{RunMetrics, aggregate_resource_budget};

/// Build a deterministic workload of 1,000 runs with non-trivial numeric
/// distribution so the optimiser cannot collapse the loop to a constant.
fn build_workload(size: usize) -> Vec<RunMetrics> {
    (0..size)
        .map(|i| {
            let i = u64::try_from(i).unwrap_or(0);
            RunMetrics {
                cpu_us: i.wrapping_mul(1_234),
                memory_bytes: i.wrapping_mul(5_678),
                iterations: i.wrapping_add(1),
            }
        })
        .collect()
}

fn bench_aggregate_resource_budget(c: &mut Criterion) {
    let runs = build_workload(1_000);
    c.bench_function("aggregate_resource_budget/1000_runs", |b| {
        b.iter(|| {
            let report = aggregate_resource_budget(black_box(&runs));
            black_box(report);
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = bench_aggregate_resource_budget
);
criterion_main!(benches);
