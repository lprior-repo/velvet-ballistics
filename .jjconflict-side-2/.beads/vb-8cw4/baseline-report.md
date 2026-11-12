bead_id: vb-8cw4
bead_title: quality: Capture supply public API and perf evidence
phase: 1
updated_at: 2026-05-17T00:00:00Z
attempt: 1-of-7

# Baseline Report

## Supply-Chain Gates

### cargo audit
- Status: PASS (exit 0)
- Known warning: RUSTSEC-2023-0089 (atomic-polyfill unmaintained) - ignored in deny.toml
- Command: `cargo audit --quiet`

### cargo deny check
- Status: FAIL (advisories FAILED, licenses FAILED)
- License failures:
  - libfuzzer-sys 0.4.12: NCSA license not in allowlist
  - resvg 0.42.0: MPL-2.0 (copyleft) not in allowlist
  - usvg 0.42.0: MPL-2.0 (copyleft) not in allowlist
  - velvet-ballistics-fuzz 0.1.0: no license field
- Advisory failures:
  - fxhash 0.2.1: RUSTSEC-2025-0057 (unmaintained)
- Bans: ok
- Sources: ok
- Command: `cargo deny check --hide-inclusion-graph`

### cargo vet
- Status: PASS (exit 0) but many missing audits
- Missing safe-to-deploy: serde, serde_yaml, unsafe-libyaml, wasm-bindgen family, windows family, winnow
- Missing safe-to-run: tracing family, sharded-slab, thread_local, valuable, web-sys
- Command: `cargo vet --store-path supply-chain --locked --verbose error`

### cargo geiger
- Not yet run in baseline; configured per-package in moon supply-chain task

### cargo machete
- Not yet run in baseline; configured in moon supply-chain task

## Benchmark Gates

### bench-build
- Status: EXISTS (13 benchmark files in benches/)
- Benchmarks: velvet_ballistics.rs (main suite), action_dispatch.rs, action_queuing.rs, array_queue.rs, cold_start.rs, collect_page.rs, ir_traversal.rs, memory_footprint.rs, pagination_cost.rs, rtrb.rs, snapshot_restore.rs, snapshot_save.rs, timer_wheel_tick.rs
- Main suite covers: yaml_parse, compile_validate, expression, runtime_core, storage_ipc, generated_mode, ir_vs_generated, generated_execution, ir_vs_generated_ratio
- Criterion-based with metadata embedding (BENCH_METADATA constant)
- No `target/criterion/` directory exists - benchmark-proof has never been run

### benchmark-proof
- Status: NOT RUN
- Command: `cargo bench --workspace --all-features -- --save-baseline vb-current`
- No baseline data exists

## Moon CI Pipeline

The `.moon.yml` pipeline includes 19 tasks in order:
1. fmt
2. lint-src
3. check
4. nightly-feature-gate
5. nightly-feature-cargo-probe
6. source-length
7. supply-chain
8. hardened-build
9. test
10. doc-test
11. doc
12. mutants-smoke
13. fuzz-smoke
14. miri
15. coverage
16. maxperf
17. maxperf-native
18. bench-build

Note: `benchmark-proof` is NOT in the CI pipeline (runInCI: false).
Note: `supply-chain` is NOT in CI (runInCI: false).

## Governance

- Pinned nightly: nightly-2026-04-28
- Allowed unstable: try_blocks, portable_simd (normal), allocator_api, generic_const_exprs (perf-only)
- Zero unsafe/unwrap/expect/panic/todo/unimplemented/dbg in production
- Performance claims require baseline/result/command/environment evidence

## Pre-existing DEFERRED_GLOBAL Issues

1. cargo deny license failures (libfuzzer-sys NCSA, resvg/usvg MPL-2.0, fuzz unlicensed)
2. cargo deny advisory failure (fxhash RUSTSEC-2025-0057)
3. cargo vet missing audits for many transitive dependencies
4. benchmark-proof not in CI pipeline
5. supply-chain task not in CI pipeline
6. No existing Criterion baseline data

## Key Files

- deny.toml: License allowlist (MIT, Apache-2.0, BSD-2/3-Clause, ISC, BSL-1.0, Unicode-3.0, Zlib)
- cargo-vet.toml: Vet configuration
- supply-chain/: audits.toml, config.toml, imports.lock
- .moon/tasks/all.yml: All moon task definitions
- docs/rust-governance.md: Governance policy document
- benches/velvet_ballistics.rs: Main benchmark suite with metadata
