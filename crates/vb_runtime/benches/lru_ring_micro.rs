//! Microbench: `LruRing` membership-backend comparison.
//!
//! Compares the terminal-runs registry workload with `IndexSet<T>` and
//! `BTreeSet<T>` membership backends. The implementation lives in sibling
//! modules so the source-length gate can enforce the same <=300-line file
//! budget for benchmark code as for the rest of the workspace.

// Bench targets are excluded from the strict source lint gate. We still keep
// this bench unsafe-free / unwrap-free / expect-free / panic-free, but allow the
// ergonomic clippy lints that benchmark scaffolds legitimately use.
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

#[path = "lru_ring_micro/mod.rs"]
mod bench_support;

use bench_support::{
    bench_contains_hit, bench_contains_miss, bench_force_insert_full, bench_insert, bench_remove,
    bench_sweep_expired,
};
use criterion::{criterion_group, criterion_main};

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
