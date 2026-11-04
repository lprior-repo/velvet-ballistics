#![allow(missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;measurement=criterion-timing;external-counters=separate-evidence";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

fn benchmark_aggregate_resource_budget_contract_surface(c: &mut Criterion) {
    let source = include_str!("../src/budget.rs");
    let id = metadata(
        "aggregate_resource_budget_contract_surface",
        source.len(),
        "fixture=budget_rs_source;surface=contract_surface_contains",
    );
    c.bench_function(&id, |b| {
        b.iter(|| {
            let present = black_box(source).contains("AggregateResourceBudget");
            assert!(present);
        });
    });
}

criterion_group!(
    benches,
    benchmark_aggregate_resource_budget_contract_surface
);
criterion_main!(benches);
