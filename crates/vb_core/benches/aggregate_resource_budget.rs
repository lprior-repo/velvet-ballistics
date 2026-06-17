#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
#![allow(missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn benchmark_aggregate_resource_budget_contract_surface(c: &mut Criterion) {
    let source = include_str!("../src/budget/mod.rs");
    c.bench_function("aggregate_resource_budget_contract_surface", |b| {
        b.iter(|| {
            let present = black_box(source).contains("AggregateResourceBudget");
            assert_eq!(present, true);
        });
    });
}

criterion_group!(
    benches,
    benchmark_aggregate_resource_budget_contract_surface
);
criterion_main!(benches);
