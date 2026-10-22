# Criterion Nested Discovery Fix — Evidence Report

## Problem
Criterion writes benchmark results to nested paths: `{home}/{bench}/{group}/{func}/new/benchmark.json`. The collector's `home.glob("*/new/benchmark.json")` only matched one directory depth, missing the intermediate group directory. Result: `CollectorError: no criterion benchmark.json for {metric}`.

## Fix
`.beads/vb-j3e3s/collect_benchmark_evidence.py` line 534: changed glob from `*/new/benchmark.json` to `*/**/new/benchmark.json` (recursive).

Preserves all matching semantics: exact full_id matching via `metric_matches`, duplicate-rejection on multiple matches, no base/ fallback, no missing-result fallback.

## Regression Tests Added
Three new tests in `.beads/vb-j3e3s/test-collector-synthetic.py`, each invoking actual `criterion_run` with only subprocess execution stubbed via `run_gated` monkeypatch:

1. `criterion_nested_group_directory_found` — verifies `{bench}/{group}/{func}/new/benchmark.json` is discovered and parsed correctly
2. `criterion_base_stale_ignored` — verifies a `base/benchmark.json` at the same depth is not matched
3. `criterion_multiple_matches_rejected` — verifies two `new/benchmark.json` files matching the same metric raise CollectorError

Test fixtures use real Criterion formats: group-prefixed full_id (`runtime_core/my_func;profile=bench`), `mean.point_estimate` field, and proper JSON structure.

## Test Results
```
80 passed, 0 failed
```
Evidence log: `.beads/vb-j3e3s/evidence/nested-criterion-regressions-v9.stdout`

## Files Changed
- `.beads/vb-j3e3s/collect_benchmark_evidence.py` — 1 line: glob pattern
- `.beads/vb-j3e3s/test-collector-synthetic.py` — 3 new tests + helper function, all using real Criterion data formats