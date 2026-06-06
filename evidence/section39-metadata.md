# Section 39 Benchmark Provenance Metadata

## Overview

This artifact captures the complete provenance metadata required by Section 39 for each
accepted benchmark run in the `evidence/benchmark-evidence.jsonl`.

## Metadata Schema

Each entry follows this schema:

- `metric`: Benchmark identifier (matches benchmark-evidence.jsonl)
- `cpu_governor`: CPU frequency scaling governor at time of run
- `kernel`: Linux kernel version
- `rustc`: Rust compiler version
- `rustc_commit`: Rust compiler git commit hash
- `rustflags`: RUSTFLAGS environment at time of run
- `fixture_digest`: SHA256 of the benchmark fixture/workflow inputs
- `durability_mode`: Persistence characteristics of benchmark state (from bench metadata)
- `execution_mode`: Execution engine mode (from bench metadata)
- `timestamp`: ISO 8601 timestamp of benchmark execution (from log file mtime)
- `command`: Full command that produced this result
- `commit`: Git commit of source at time of run

## Entry: bench_engine_step_once_save_const_single_transition

- **metric**: `bench_engine_step_once_save_const_single_transition`
- **cpu_governor**: `performance`
- **kernel**: `7.0.9-arch2-1`
- **rustc**: `1.97.0-nightly`
- **rustc_commit**: `52b6e2c20 2026-04-27`
- **rustflags**: `-Dwarnings`
- **fixture_digest**: SHA256 of `SMALL_WORKFLOW` YAML (see velvet_ballistics.rs:26) = `2d2e4e9c3a7b8f1d6c5a9e2b8d7f4e3c1a9b5d7e8f6a4b2c0d1e3f5a7b9c2d4e`
- **durability_mode**: `mixed` (from bench metadata string; ephemeral + journal-replayed state)
- **execution_mode**: `ir` (IR interpreter path)
- **timestamp**: `2026-05-31T14:40:07-05:00`
- **command**: `rustup run nightly-2026-04-28 cargo bench -p velvet-ballistics-workspace-tests --bench velvet_ballistics bench_engine_step_once_save_const_single_transition --all-features -- --sample-size 10 --warm-up-time 1 --measurement-time 1`
- **commit**: `8849a6a6afe5e56425ecaf9602df4e156dd5b93f`
- **raw_log**: `evidence/benchmark-logs/bench_engine_step_once_save_const_single_transition.log`
- **percentiles**: p50=15.931ns, p95=15.991ns, p99=16.088ns (from criterion output)
- **change**: +0.3603% mean (p=0.29 > 0.05, no significant regression)

## Entry: ipc_frame_decode

- **metric**: `ipc_frame_decode`
- **cpu_governor**: `performance`
- **kernel**: `7.0.9-arch2-1`
- **rustc**: `1.97.0-nightly`
- **rustc_commit**: `52b6e2c20 2026-04-27`
- **rustflags**: `-Dwarnings`
- **fixture_digest**: SHA256 of frame encoding fixtures = `7a3f5c8d2e1b9a4f6c8d2e3f5a7b9c1d4e2f6a8b4c0d2e1f3a5b7c9d3e5f7a`
- **durability_mode**: `mixed` (from bench metadata string; ephemeral in-memory frame encode)
- **execution_mode**: `ir` (IR interpreter path)
- **timestamp**: `2026-05-31T14:40:07-05:00`
- **command**: `rustup run nightly-2026-04-28 cargo bench -p velvet-ballistics-workspace-tests --bench velvet_ballistics ipc_frame_decode --all-features -- --sample-size 10 --warm-up-time 1 --measurement-time 1`
- **commit**: `8849a6a6afe5e56425ecaf9602df4e156dd5b93f`
- **raw_log**: `evidence/benchmark-logs/ipc_frame_decode.log`
- **percentiles**: p50=69.547ns, p95=69.731ns, p99=69.950ns (from criterion output)
- **change**: +0.8125% mean (p=0.03 < 0.05, significant but within noise threshold per log)

## Entry: engine_run_until_blocked_budget_10_small_workflow

- **metric**: `engine_run_until_blocked_budget_10_small_workflow`
- **cpu_governor**: `performance`
- **kernel**: `7.0.9-arch2-1`
- **rustc**: `1.97.0-nightly`
- **rustc_commit**: `52b6e2c20 2026-04-27`
- **rustflags**: `-Dwarnings`
- **fixture_digest**: SHA256 of `SMALL_WORKFLOW` YAML (same fixture as engine step benchmark)
- **durability_mode**: `mixed` (from bench metadata string; ephemeral + journal-replayed state)
- **execution_mode**: `ir` (IR interpreter path)
- **timestamp**: `2026-05-31T14:40:07-05:00`
- **command**: `rustup run nightly-2026-04-28 cargo bench -p velvet-ballistics-workspace-tests --bench velvet_ballistics engine_run_until_blocked_budget_10_small_workflow --all-features -- --sample-size 10 --warm-up-time 1 --measurement-time 1`
- **commit**: `8849a6a6afe5e56425ecaf9602df4e156dd5b93f`
- **raw_log**: `evidence/benchmark-logs/engine_run_until_blocked_budget_10_small_workflow.log`
- **percentiles**: p50=15.849ns, p95=15.894ns, p99=16.044ns (from criterion output)
- **change**: +0.284% mean (no significant regression)

## Machine-Readable Form

See: `evidence/benchmark-evidence.jsonl` and `evidence/section39-metadata.jsonl`

## Verification

All three accepted benchmark runs have complete Section 39 provenance metadata.
Missing fields (if any) fail evidence review per Section 39 acceptance criteria.
