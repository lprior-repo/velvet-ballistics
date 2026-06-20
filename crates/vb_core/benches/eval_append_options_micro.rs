//! Bench sketch for `eval_append` O(n²) cumulative fix (vb-jf1c1 → vb-jim32).
//!
//! All benches in this file are `#[ignore]`d and tagged `BENCH-CANDIDATE-SKETCH`.
//! They are not run by `cargo bench`; the implementation bead `vb-jim32` will
//! either keep these or replace them with measured harnesses.
//!
//! Run with: `cargo bench -p vb_core --bench eval_append_options_micro -- --ignored`
//!
//! Baseline (from existing `expr_eval_micro.rs`): single `eval_append` at
//! N=65536 = 695ms cumulative. Pre-built `Vec` then `insert_list` = 138μs.
//! Target speedup: 5036×.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::print_stdout)]

use criterion::{Criterion, criterion_group, criterion_main};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

/// CANDIDATE — Option B (recommended): builder `Vec` + single materialize.
#[ignore = "BENCH-CANDIDATE-SKETCH: Option B candidate; requires vb-jim32 implementation"]
fn bench_eval_append_candidate_builder_materialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_append_candidate_builder");
    for &n in &[256usize, 1024, 4096, 16384, 65536] {
        group.bench_function(format!("n={n}"), |b| {
            b.iter(|| {
                let mut builder: Vec<SlotValue> = Vec::with_capacity(n);
                for i in 0..n {
                    builder.push(SlotValue::I64(i as i64));
                }
                let mut store = ValueStore::new();
                store
                    .insert_list(builder.into_boxed_slice())
                    .expect("materialize must succeed")
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_eval_append_candidate_builder_materialize);
criterion_main!(benches);
