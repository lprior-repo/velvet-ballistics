use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use vb_boundary_inventory::boundary_inventory::{
    BoundaryCandidate, classify_boundary, parse_inventory,
};

fn benchmark_boundary_inventory_parser(c: &mut Criterion) {
    let input = br#"{"schema_version":1,"boundaries":[]}"#;

    c.bench_function("vb_y1zq_parse_minimal_inventory", |b| {
        b.iter(|| parse_inventory(black_box(input)))
    });
}

fn benchmark_boundary_classification(c: &mut Criterion) {
    c.bench_function("vb_y1zq_classify_ipc_boundary", |b| {
        b.iter(|| {
            classify_boundary(black_box(BoundaryCandidate::new(
                "crates/vb_ipc/src/frame.rs",
                "ipc-frame-boundary",
            )))
        })
    });
}

criterion_group!(
    benches,
    benchmark_boundary_inventory_parser,
    benchmark_boundary_classification
);
criterion_main!(benches);
