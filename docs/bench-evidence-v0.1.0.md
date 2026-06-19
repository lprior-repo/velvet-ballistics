# Benchmark Evidence — v0.1.0

**Bead family:** `vb-a7t6` (parent) + `vb-a7t6.1` (provenance), `vb-a7t6.2` (latency),
`vb-a7t6.3` (instructions), `vb-a7t6.4` (allocations) — all five CLOSED in Dolt.

**Master sections honored:** §39 (Mandatory Benchmarks), §77.14 (Allocation Tracing Gates).

**Date:** 2026-06-19
**Commit at capture:** `969d1219cee63ef850e2d6391ccacf0762fbe271`
**Toolchain:** `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)` + `cargo 2026-04-24`
**Kernel:** `7.0.9-arch2-1`
**CPU:** `AMD Ryzen 9 9950X3D 16-Core Processor`
**Profile:** `bench` (release + line-tables; thin LTO; codegen-units=1)

---

## Executive Summary (HONEST)

The `vb-a7t6` family is fully closed. The five closed beads produced three layered evidence
files at `evidence/`:

| Evidence file | Bead | Fields | Scenarios covered |
|---|---|---|---|
| `benchmark-evidence.jsonl` | `vb-a7t6.2` + `vb-a7t6.3` | p50/p95/p99 + instructions:u + tool/cpu/kernel provenance | 3 v1 scenarios |
| `alloc-evidence.jsonl` | `vb-a7t6.4` | alloc_count + peak_heap + leak_count + heaptrack provenance | 3 v1 scenarios |
| `section39-metadata.jsonl` | `vb-a7t6.1` | cpu_governor + kernel + rustc + rustc_commit + RUSTFLAGS + fixture_digest + durability_mode + execution_mode + timestamp + command + commit | 3 v1 scenarios |

This document adds **`scripts/bench-evidence.sh`** — an idempotent umbrella wrapper that
runs the layered evidence pipeline end-to-end. The script does **not** modify any source
code under `crates/**`; it orchestrates existing scripts and the `cargo bench` driver.

## Methodology

The umbrella wrapper coordinates five evidence-collection stages:

| Stage | Tool | Output | Coverage |
|---|---|---|---|
| 1. `cargo bench --no-run` | `cargo` (workspace, all features) | compile-check log | all 41 bench executables |
| 2. `cargo bench --bench velvet_ballistics` | criterion 0.8 | `target/criterion/<id>/benchmark.json` (p50/p95/p99 in standard JSON) | all 21 bench files via criterion's standard output |
| 3. `bench-instruction-counts.sh` | `perf stat -e instructions:u` | `evidence/instruction-counts.jsonl` + per-scenario `.perf-stat.txt` + `.instructions.jsonl` | 3 v1 scenarios (Path B userspace) |
| 4. `bench-alloc-evidence.sh` | `heaptrack 1.5.0` | `evidence/alloc-evidence.jsonl` + per-scenario `alloc.<metric>.log` | 3 v1 scenarios |
| 5. summary writer | hand-rolled JSONL | `evidence/bench-evidence-summary.jsonl` | one row per wrapper run |

The umbrella script:

- Fails closed (exit 2) if `cargo`, `jq`, `perf`, `heaptrack`, or `heaptrack_print` are missing.
- Discovers every `benches/*.rs` file under `crates/` (21 files at v0.1.0).
- Reads the v1 scenario list from `evidence/section39-metadata.jsonl` (3 entries).
- Honors `--only criterion|instructions|alloc|all`, `--force` (no-op passthrough),
  `--bench-package`, `--bench-name`, `--dry-run`.
- Writes `evidence/bench-evidence-summary.jsonl` summarizing stage outcomes and residual gaps.

## p50/p95/p99 emission status

**Status: emitted for the 3 v1 scenarios; criterion emits them in standard JSON for all 21
bench files.**

The `latency_p50_p95_p99` module in `crates/workspace_tests/benches/velvet_ballistics.rs`
emits per-bench sidecar files at `evidence/benchmark-logs/<bench_id>.percentiles.jsonl` and
`<bench_id>.raw-samples.txt` for the three scenarios that wire through
`checked_iter_with_percentiles`:

| Scenario | Sidecar (`evidence/benchmark-logs/`) | Percentile file size | Sample file size |
|---|---|---|---|
| `bench_engine_step_once_save_const_single_transition` | `.percentiles.jsonl` + `.raw-samples.txt` | 202 B | 4 KiB |
| `ipc_frame_decode` | `.percentiles.jsonl` + `.raw-samples.txt` | 181 B | 6 KiB |
| `engine_run_until_blocked_budget_10_small_workflow` | `.percentiles.jsonl` + `.raw-samples.txt` | 200 B | 4 KiB |

Schema (one JSONL row per sidecar file):

```json
{
  "bench_id": "<metric>",
  "sample_count": <u64>,
  "min_ns": <u64>,
  "max_ns": <u64>,
  "total_ns": <u64>,
  "mean_ns": <u64>,
  "p50_latency_ns": <u64>,
  "p95_latency_ns": <u64>,
  "p99_latency_ns": <u64>
}
```

For every other bench file, criterion 0.8 itself writes
`target/criterion/<bench_id>/benchmark.json` with the standard `median`, `p95`, `p99`
fields — those are not duplicated into the bespoke sidecar format. The umbrella script's
criterion stage is the source of truth for the broader coverage.

## Allocation count + bytes allocated emission status

**Status: emitted for the 3 v1 scenarios via `heaptrack 1.5.0`; residual gap for the other 18 bench files.**

The `bench-alloc-evidence.sh` wrapper (closed `vb-a7t6.4`) drives `heaptrack --record-only`
over the criterion bench binary for each v1 scenario and parses the `heaptrack_print`
summary. Three rows live at `evidence/alloc-evidence.jsonl`:

| Scenario | alloc_count | peak_heap (bytes) | peak_rss (bytes) | leak_count |
|---|---|---|---|---|
| `bench_engine_step_once_save_const_single_transition` | 20,176 | 1,352,663 | 12,666,798 | 8,308 |
| `ipc_frame_decode` | 20,024 | 1,352,663 | 12,803,112 | 8,306 |
| `engine_run_until_blocked_budget_10_small_workflow` | 20,168 | 1,352,663 | 14,302,576 | 8,308 |

Each row carries: `alloc_count`, `alloc_methodology` (heaptrack LD_PRELOAD), `alloc_raw_log`
(relative path under `evidence/benchmark-logs/`), `alloc_tool` + `alloc_tool_version`,
`bytes_allocated` (peak heap proxy — disclosed via `bytes_allocated_proxy: true`),
`peak_heap`, `peak_rss`, `leak_bytes`, `leak_count`, `command`, `commit`,
`fixture_digest`, `execution_mode`, `timestamp`.

The other 18 bench files do **not** have heaptrack coverage at v0.1.0. Closing that
residual would require either a `cargo bench --no-fail-fast --workspace` driver loop in
the alloc wrapper, or switching the wrapper to use criterion's `--profile-time` allocator
instrumentation. Both are out of scope for `vb-a7t6.4` (closed); they would be a fresh
bead.

## Instruction count emission status

**Status: emitted for the 3 v1 scenarios via `perf stat -e instructions:u` (userspace, Path B);
kernel-aware Path A (valgrind + iai-callgrind) is the open follow-up `vb-a7t6.3.a`.**

The `bench-instruction-counts.sh` wrapper (closed `vb-a7t6.3`) drives `perf stat` over the
criterion bench binary for each v1 scenario. Three rows live at
`evidence/instruction-counts.jsonl`:

| Scenario | instructions_count | tool | kernel |
|---|---|---|---|
| `bench_engine_step_once_save_const_single_transition` | 241,963,878 | `perf 7.0.9-1` (userspace) | `7.0.9-arch2-1` |
| `ipc_frame_decode` | 241,628,412 | `perf 7.0.9-1` (userspace) | `7.0.9-arch2-1` |
| `engine_run_until_blocked_budget_10_small_workflow` | 241,895,784 | `perf 7.0.9-1` (userspace) | `7.0.9-arch2-1` |

**Limitation disclosure (Path B):**

- `perf_event_paranoid=2` on the build host restricts `perf stat -e instructions:u` to
  userspace counters. Kernel-mode instruction counts require `valgrind` + `iai-callgrind`
  (Path A).
- `valgrind` and `iai-callgrind-runner` are not installed in the build host at v0.1.0.
  This is captured as the open bead `vb-a7t6.3.a` (P1, unblocked). The task is environment
  tooling, not source code.

## v0.1.0 coverage matrix

| Master §39 area | Bench file(s) | p50/p95/p99 | alloc | instructions |
|---|---|---|---|---|
| YAML parsing (small + 1 MiB) | `velvet_ballistics.rs` | criterion JSON | criterion only | — |
| Validation (minimal + 1000-step) | `velvet_ballistics.rs` | criterion JSON | criterion only | — |
| Compilation (minimal + 1000-step) | `velvet_ballistics.rs` | criterion JSON | criterion only | — |
| Expression (4 scenarios) | `velvet_ballistics.rs` | criterion JSON | criterion only | — |
| Slot operations | `velvet_ballistics.rs` | criterion JSON | criterion only | — |
| Core transitions | `velvet_ballistics.rs` | criterion JSON | criterion only | — |
| Run chains (1 / 10 / 1000 steps) | `velvet_ballistics.rs` | criterion JSON | criterion only | — |
| Iteration (foreach, together, collect, reduce, repeat) | `velvet_ballistics.rs` (section39_missing) | criterion JSON | criterion only | — |
| Fjall storage (no-persist, journaled, strict) | `velvet_ballistics.rs` | criterion JSON | criterion only | — |
| Fjall read 1000 events | `velvet_ballistics.rs` | criterion JSON | criterion only | — |
| IPC frame encode + decode | `velvet_ballistics.rs` | sidecar + JSON | heaptrack v1 | perf v1 |
| ArrayQueue + rtrb | `array_queue.rs`, `rtrb.rs` | criterion JSON | criterion only | — |
| Trace event push + ring full | `velvet_ballistics.rs` (section39_missing) | criterion JSON | criterion only | — |
| Journal writer queue + group commit | `velvet_ballistics.rs` (section39_missing) | criterion JSON | criterion only | — |
| Scheduler shard submit-to-start + submit-to-finish | `velvet_ballistics.rs` (section39_missing) | criterion JSON | criterion only | — |
| Direct API submit-to-finish | `velvet_ballistics.rs` (section39_missing) | criterion JSON | criterion only | — |
| Async primitives (ask / action_complete / wait_timer) | `velvet_ballistics.rs` (section39_missing) | criterion JSON | criterion only | — |
| IR traversal | `ir_traversal.rs` | criterion JSON | criterion only | — |
| Collect page | `collect_page.rs` | criterion JSON | criterion only | — |
| Action dispatch | `action_dispatch.rs` | criterion JSON | criterion only | — |
| Memory footprint | `memory_footprint.rs` | criterion JSON | criterion only | — |
| Cold start | `cold_start.rs` | criterion JSON | criterion only | — |
| Pagination cost | `pagination_cost.rs` | criterion JSON | criterion only | — |
| Action queuing | `action_queuing.rs` | criterion JSON | criterion only | — |
| Timer wheel tick | `timer_wheel_tick.rs` | criterion JSON | criterion only | — |
| Snapshot save / restore | `snapshot_save.rs`, `snapshot_restore.rs` | criterion JSON | criterion only | — |
| Recovery / replay / boundary inventory | `vb_qi37_1_1_recovery.rs`, `vb_h6ix_replay.rs`, `vb_y1zq_boundary_inventory.rs`, `vb_kkvb_xtask_routing_red.rs` | criterion JSON | criterion only | — |
| **Single-transition step (engine)** | `velvet_ballistics.rs` | **sidecar v1** | **heaptrack v1** | **perf v1** |
| **Engine run until blocked (small workflow)** | `velvet_ballistics.rs` | **sidecar v1** | **heaptrack v1** | **perf v1** |

The bold rows are the 3 v1 scenarios with full §39 evidence bundles. The remaining rows
have criterion's standard JSON output (which does include p50/p95/p99 in the
`target/criterion/<id>/benchmark.json` files).

## Usage

```bash
# Full pipeline (compile-check, criterion, perf, heaptrack, summary)
scripts/bench-evidence.sh

# Plan only (no execution)
scripts/bench-evidence.sh --dry-run

# Criterion only
scripts/bench-evidence.sh --only criterion

# Instruction evidence only
scripts/bench-evidence.sh --only instructions

# Alloc evidence only
scripts/bench-evidence.sh --only alloc
```

Outputs land in `evidence/`:

- `evidence/bench-evidence-summary.jsonl` — one row per wrapper run, summarizing stages.
- `evidence/benchmark-logs/criterion-velvet_ballistics.log{,.stderr}` — criterion stdout.
- `evidence/benchmark-logs/cargo-bench-no-run.{stdout,stderr}.log` — compile check.
- `evidence/benchmark-logs/bench-instruction-counts.{stdout,stderr}.log` — Path B stderr.
- `evidence/benchmark-logs/bench-alloc-evidence.{stdout,stderr}.log` — heaptrack stderr.
- The existing `evidence/instruction-counts.jsonl`, `evidence/alloc-evidence.jsonl`,
  `evidence/section39-metadata.jsonl`, and `evidence/benchmark-evidence.jsonl` are
  overwritten by the child scripts (idempotent on the v1 envelope).

## Residual gaps (HONEST)

1. **Kernel-aware instruction counts** — `valgrind` + `iai-callgrind` are not installed in
   the build host. The open bead `vb-a7t6.3.a` (P1, unblocked) covers installing them and
   switching the wrapper to Path A. Until then, `instructions:u` is userspace only.

2. **Allocation evidence is bounded to 3 v1 scenarios** — `bench-alloc-evidence.sh` only
   captures the 3-scenario envelope. Extending heaptrack coverage to the other 18 bench
   files would be a fresh bead.

3. **Bespoke `latency_p50_p95_p99` sidecars are bounded to 3 v1 scenarios** — criterion
   captures p50/p95/p99 in its standard JSON output for every bench, but only the 3 v1
   scenarios emit the bespoke sidecar JSONL. Extending `checked_iter_with_percentiles`
   coverage would be a fresh bead.

4. **Alloc bytes_allocated is a peak heap proxy** — `bytes_allocated: 1,352,663` is the
   peak heap, not a sum of allocation sizes. This is disclosed via
   `bytes_allocated_proxy: true` in every `alloc-evidence.jsonl` row. A precise counter
   would need allocator instrumentation (e.g. `dhat`) or a custom global allocator with
   counting — out of scope at v0.1.0.

5. **`perf_event_paranoid=2`** — kernel-mode counters require `sudo` or a sysctl change.
   Not in our control.

## Verification

The 3 v1 scenario sidecar files are bound by regression tests in
`crates/workspace_tests/tests/vb_a7t6_2_percentile_math_tests.rs` (8 tests, vb-a7t6.2) and
`crates/workspace_tests/tests/vb_a7t6_3_instruction_count_tests.rs` (14 tests,
vb-a7t6.3). Both test files exist in the workspace; both pass per the bead closure
records.

`scripts/check-bench-registration.sh` (moon-gated) verifies that all 21 bench files are
registered in their crate's `[[bench]]` table.

## Reproduction

```bash
# Compile all bench executables
cargo bench --no-run --workspace --all-features

# Run the umbrella wrapper
scripts/bench-evidence.sh

# Inspect results
ls -la evidence/benchmark-logs/
cat evidence/bench-evidence-summary.jsonl | jq .
```
