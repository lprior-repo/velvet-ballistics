//! Fixture-backed top-level benchmarks.

#![allow(missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const WORKFLOW_YAML: &str = r#"
version: "velvet-ballastics/v1"
name: bench-workflow
trigger:
  type: manual
steps:
  - id: start
    set:
      output: greeting
      value: "hello"
    then: finish
  - id: finish
    finish:
      result: "done"
"#;

const WORKSPACE_TOML: &str = r#"
[workspace]
resolver = "2"
members = ["crates/velvet_ballastics", "fuzz"]
"#;

const VALUE_FIXTURE: &[u8] = b"velvet-ballastics-benchmark-fixture";

fn yaml_parse_bench(c: &mut Criterion) {
    c.bench_function("yaml_parse_fixture", |b| {
        b.iter(|| serde_yaml::from_str::<serde_yaml::Value>(criterion::black_box(WORKFLOW_YAML)))
    });
}

fn toml_parse_bench(c: &mut Criterion) {
    c.bench_function("toml_parse_fixture", |b| {
        b.iter(|| toml::from_str::<toml::Value>(criterion::black_box(WORKSPACE_TOML)))
    });
}

fn value_clone_bench(c: &mut Criterion) {
    let value = Vec::from(VALUE_FIXTURE);
    c.bench_function("value_clone_fixture", |b| {
        b.iter(|| criterion::black_box(value.clone()))
    });
}

fn value_serialize_bench(c: &mut Criterion) {
    c.bench_function("value_serialize_fixture", |b| {
        b.iter(|| criterion::black_box(VALUE_FIXTURE.to_vec()))
    });
}

fn value_deserialize_bench(c: &mut Criterion) {
    let encoded = Vec::from(VALUE_FIXTURE);
    c.bench_function("value_deserialize_fixture", |b| {
        b.iter(|| String::from_utf8(criterion::black_box(encoded.clone())))
    });
}

fn workflow_digest_bench(c: &mut Criterion) {
    c.bench_function("workflow_digest_fixture", |b| {
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            criterion::black_box(WORKFLOW_YAML).hash(&mut hasher);
            criterion::black_box(hasher.finish())
        })
    });
}

fn diagnostic_render_bench(c: &mut Criterion) {
    let error = "compile error: missing required field steps";
    c.bench_function("diagnostic_render_fixture", |b| {
        b.iter(|| format!("error: {}", criterion::black_box(error)))
    });
}

fn id_route_bench(c: &mut Criterion) {
    c.bench_function("id_route_fixture", |b| {
        b.iter(|| {
            let mut shard = 0_u64;
            let bytes = criterion::black_box(VALUE_FIXTURE);
            for byte in bytes {
                shard = shard.wrapping_mul(33).wrapping_add(u64::from(*byte));
            }
            criterion::black_box(shard % 16)
        })
    });
}

criterion_group!(
    benches,
    yaml_parse_bench,
    toml_parse_bench,
    value_clone_bench,
    value_serialize_bench,
    value_deserialize_bench,
    workflow_digest_bench,
    diagnostic_render_bench,
    id_route_bench
);
criterion_main!(benches);
