bead_id: vb-8cw4
bead_title: quality: Capture supply public API and perf evidence
phase: 2
updated_at: 2026-05-17T00:00:00Z
attempt: 1-of-7

# Research Notes

## Existing Benchmark Harnesses

### Main Suite: benches/velvet_ballastics.rs (2715 lines)
- Criterion-based with 17 benchmark groups
- Groups: yaml_parse, compile_validate, expression, runtime_core, storage_ipc, generated_mode, ir_vs_generated, generated_execution, ir_vs_generated_ratio, taint_scalar_expr, taint_slot_loading, taint_build_object, taint_build_list, taint_full_workflow, submit_artifact, budget_compute, evidence_chain, admission_gate, capability_check
- Metadata embedded in benchmark IDs via `metadata()` function: name, BENCH_METADATA constant, extra tags, fixture_bytes, fixture_digest (blake3)
- Latency budget enforcement via `checked_iter()` with `VB_BENCH_LATENCY_BUDGET_US` env var
- Uses `criterion_group!` and `criterion_main!` macros

### Additional Bench Files
- benches/action_dispatch.rs
- benches/action_queuing.rs
- benches/array_queue.rs
- benches/cold_start.rs
- benches/collect_page.rs
- benches/ir_traversal.rs
- benches/memory_footprint.rs
- benches/pagination_cost.rs
- benches/rtrb.rs
- benches/snapshot_restore.rs
- benches/snapshot_save.rs
- benches/timer_wheel_tick.rs

## Supply-Chain Tools

### cargo audit
- Runs against RUSTSEC database
- Known warning: RUSTSEC-2023-0089 (atomic-polyfill) - ignored in deny.toml

### cargo deny
- License allowlist: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, BSL-1.0, Unicode-3.0, Zlib
- Pre-existing failures: libfuzzer-sys (NCSA), resvg/usvg (MPL-2.0), velvet-ballastics-fuzz (unlicensed), fxhash (RUSTSEC-2025-0057)

### cargo vet
- Store at supply-chain/ (audits.toml, config.toml, imports.lock)
- Many transitive deps missing audits

### cargo geiger
- Per-package unsafe code scanning
- Configured for all first-party packages

### cargo machete
- Unused dependency detection

## Moon CI Pipeline

19 tasks in .moon.yml pipeline. Key gaps:
- `supply-chain` task: runInCI: false (not in CI pipeline)
- `benchmark-proof` task: runInCI: false (not in CI pipeline)
- No public API compatibility gate
- No semver check gate
- No binary bloat analysis gate

## What's Missing

1. **Public API compatibility evidence**: No `cargo-semver-checks` or similar tool wired
2. **Semver stability evidence**: No semver tracking mechanism
3. **Binary bloat analysis**: No `cargo bloat` integration
4. **Evidence capture system**: No structured way to capture, store, and validate evidence bundles
5. **Evidence gate tests**: No tests that verify evidence completeness
6. **Benchmark evidence metadata validation**: No gate that verifies baseline/result/command/environment metadata exists

## Public API Surface

Crates exposed: vb_core, vb_expr, vb_yaml, vb_compile, vb_runtime, vb_storage, vb_ipc, vb_codegen, vb_validate, vb_cli
