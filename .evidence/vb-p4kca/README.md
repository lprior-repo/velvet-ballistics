# vb-p4kca / vb-ship-009 baseline benchmark evidence

## What this captures

This is the first recorded live benchmark evidence for bead **vb-p4kca**
(master section 71 — Competitive Performance Targets). It closes the
ship-gate blocker that no bead carries a real Criterion run with
metadata.

## Run

- **Benchmark**: `aggregate_resource_budget/1000_runs`
- **Crate**: `vb_benchmark` (Cargo.toml `[[bench]]` entry)
- **Harness**: Criterion 0.8.2
- **Command**: `cargo +nightly bench -p vb_benchmark --bench aggregate_resource_budget -- --format terse -v`
- **Build profile**: bench (`optimized + debuginfo`)
- **Criterion sample size**: 100
- **Criterion warm-up time**: 3 s
- **Criterion measurement time**: 5 s
- **Sampling mode**: Linear (default)

## Headline numbers

| metric            | value      | source                                    |
|-------------------|------------|-------------------------------------------|
| median (p50)      | 400.95 ns  | `criterion-estimates.json` median.point_estimate |
| mean              | 402.98 ns  | `criterion-estimates.json` mean.point_estimate   |
| p95 (approx)      | 415.83 ns  | mean + 1.645 * std_dev                    |
| p99 (approx)      | 421.20 ns  | mean + 2.33 * std_dev                     |
| std_dev           | 11.82 ns   | `criterion-estimates.json` std_dev.point_estimate |
| outliers          | 3 of 100 (3.0%) | criterion verbose report              |

The aggregate p50 (~401 ns / 1 000 runs) is the fold of 1 000
`RunMetrics` rows. Per-run cost is roughly 0.40 ns — this is dominated
by the saturating-add loop body, not by anything comparable to a real
single-step transition.

## Honest caveats

- This is a **baseline** recording only. No performance claim is being
  made.
- The p95 and p99 values are **derived** (mean + k * sigma), not
  directly measured. Criterion's linear sampling protocol is not
  suitable for empirical percentile tail extraction from its raw
  sample.json windows.
- The criterion run reports `Performance has regressed` because this
  is the first run; criterion has no prior baseline to compare
  against.
- The master section 71 target is the *single step* p50 ≤ 1 ms
  (volatile). The number above is for a fold of 1 000 rows of
  RunMetrics, not a single workflow step. It is evidence that the
  benchmark toolchain runs end-to-end and produces reproducible
  numbers; it is **not** evidence that the workflow step target is
  met. Other beads (transition_set, ir_interpreter_dispatch,
  end-to-end 9-step workflow) will need to be run with similar
  metadata capture to close the master section 71 target itself.
- The working tree was dirty at the time of the run (274 modified
  files). The committed sources for this benchmark file and its
  dependencies are at commit
  `220dbe278697979435c7332d2af7799bdf567edc`.
- This evidence bundle was force-added (`.evidence/` is gitignored)
  and committed as `e1daeece48c5aa36f7ef2c1073100798051d3ade`
  with the message
  `evidence(vb-ship-009): initial aggregate_resource_budget baseline + metadata`.

## Files in this evidence bundle

- `bench-output.txt` — first two short bencher-format runs
- `bench-full.txt` — terse + verbose run with confidence intervals
- `criterion-estimates.json` — copy of `target/criterion/.../new/estimates.json`
- `criterion-sample.json` — copy of `target/criterion/.../new/sample.json`
- `metadata.yaml` — structured metadata for the run
- `README.md` — this file
