# Bench Regression Guard Baselines — Bead vb-hxa55

**Date:** 2026-06-21
**JJ change:** `@  mztwvonz` (1bafa6b7)
**Hardware:** AMD Ryzen 9 9950X3D 16-Core Processor
**Kernel:** 7.0.9-arch2-1
**rustc:** 1.97.0-nightly (52b6e2c20 2026-04-27)
**Criterion:** 0.8.2
**Sample size:** 10
**Warm-up time:** 1.0s
**Measurement time:** 1.0s
**Behaviors:** `--all-features`

## Scope

Closes bead vb-hxa55 by registering `expr_eval_micro` (vb_core) and
`lru_ring_micro` (vb_runtime) as moon ci regression guards with
recorded baselines and per-scenario thresholds.

## New moon ci tasks

- `bench-expr-eval-micro-guard` — runs `cargo bench -p vb_core --bench
  expr_eval_micro`, pipes criterion stdout to
  `evidence/benchmark-logs/expr_eval_micro.log`. `runInCI: true`.
- `bench-lru-ring-micro-guard` — runs `cargo bench -p vb_runtime --bench
  lru_ring_micro`, pipes criterion stdout to
  `evidence/benchmark-logs/lru_ring_micro.log`. `runInCI: true`.
- `bench-regression-guards` — aggregate gate that depends on both
  guard tasks and then invokes
  `cargo run -p xtask -- benchmark-regression-policy --budget
  contracts/perf-budget.yaml --evidence
  evidence/benchmark-evidence.jsonl`. Fails closed on regression >
  threshold OR on missing baseline row. `runInCI: true`.

## Threshold policy

Added to `contracts/perf-budget.yaml`:

| metric                                                | max_regression_percent |
|-------------------------------------------------------|------------------------|
| `expr_eval_micro_eval_unique_current_O_n2_size_1024`  | 5                      |
| `expr_eval_micro_eval_merge_current_O_LxR_size_1024`   | 5                      |
| `lru_ring_micro_insert_IndexSet_size_100000`          | 5                      |
| `lru_ring_micro_contains_hit_IndexSet_size_100000`    | 5                      |

5% (vs the 3% tighter budget on the existing umbrella scenarios) because
these are new registrations with a single prior baseline; tightening
would be premature before variance across multiple runs is captured.
Tracked as residual follow-up `vb-hxa55.1` for a future tightening pass.

## Baseline numbers (measured on this hardware, this commit)

| metric                                                | p50    | p95    | p99    | sample_count |
|-------------------------------------------------------|--------|--------|--------|--------------|
| `expr_eval_micro_eval_unique_current_O_n2_size_1024`  | 253.94 µs | 278.72 µs | 278.72 µs | 4235 |
| `expr_eval_micro_eval_merge_current_O_LxR_size_1024`   | 213.58 µs | 216.30 µs | 216.30 µs | 4730 |
| `lru_ring_micro_insert_IndexSet_size_100000`          | 3.0418 ms | 3.1292 ms | 3.1292 ms | 420 |
| `lru_ring_micro_contains_hit_IndexSet_size_100000`    | 770.07 µs | 784.79 µs | 784.79 µs | 1320 |

These four rows are appended to
`evidence/benchmark-evidence.jsonl` (lines 4-7) and mirrored with full
provenance in `evidence/section39-metadata.jsonl` (lines 4-7).

## Bench files

- `crates/vb_core/benches/expr_eval_micro.rs` — already registered as
  `[[bench]] name = "expr_eval_micro"` in `crates/vb_core/Cargo.toml`
  (confirmed by `bash scripts/check-bench-registration.sh`: 24/24
  registered).
- `crates/vb_runtime/benches/lru_ring_micro.rs` — already registered as
  `[[bench]] name = "lru_ring_micro"` in `crates/vb_runtime/Cargo.toml`
  (same check).

## Pre-flight reproduction

```bash
# Verify registration
bash scripts/check-bench-registration.sh
# exit 0; 24/24 benches registered

# Run the new guard tasks
moon run :bench-expr-eval-micro-guard
moon run :bench-lru-ring-micro-guard
moon run :bench-regression-guards
# exit 0 if all four evidence rows present and within 5% of baseline
```

## Acceptance status

Per bead `ears_requirements`:

- "THE SYSTEM SHALL run expr_eval_micro and lru_ring_micro benches in
  moon ci" — `bench-expr-eval-micro-guard` and
  `bench-lru-ring-micro-guard` both `runInCI: true`. Met.
- "fail on regression beyond a documented threshold" —
  `bench-regression-guards` invokes `benchmark-regression-policy` with
  `contracts/perf-budget.yaml`; missing baseline row or >5% regression
  yields non-zero exit. Met.
- "WHEN moon ci runs the bench task" / "execute both benches and compare
  against recorded baselines" — `bench-regression-guards` depends on
  both guard tasks; policy check consumes the four new evidence rows.
  Met.
- "IF a hot-path op regresses beyond threshold" / "SHALL NOT silently
  pass" — `benchmark-regression-policy` is fail-closed (non-zero exit
  on any threshold violation or missing baseline). Met.
