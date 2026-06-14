S-19r vb-benchmark-cleanup: Replace 11 STUB: markers in benchmark_tests.rs with real assertions; convert aggregate_resource_budget stub into real Criterion bench (NEW FILE)

# Verification excerpts (read-before-write)

## crates/vb_benchmark/tests/benchmark_tests.rs (609 lines)
- The 11 STUB: markers are at lines 10, 20, 31, 41, 51, 58, 68, 79, 90, 107, 127. Confirmed by reading the file.
- The actual STUB: text is `// STUB: <function_name> <description>` — NOT `// STUB: This test will FAIL`. Examples:
  - Line 10: `// STUB: baseline_within_budget always returns false`
  - Line 31: `// STUB: budget_utilization_percent always returns 0`
  - Line 51: `// STUB: latency_within_budget inverts the check`
  - Line 68: `// STUB: result_exceeds_threshold inverts the logic`
  - Line 90: `// STUB: check_evidence_gate always returns Ok`

## crates/vb_benchmark/src/ — DIRECTORY LISTING
- The directory contains ONLY `error.rs` and `lib.rs`. There is NO `aggregate_resource_budget.rs` — that file does NOT EXIST. The round-2 bead's file path is fabricated.

## crates/vb_benchmark/benches/ — DIRECTORY LISTING
- This directory does NOT EXIST. The round-2 bead's reference to `crates/vb_benchmark/benches/` is fabricated.

# Round-2 corrections applied (from black-hat review)

The round-2 bead cited `crates/vb_benchmark/src/aggregate_resource_budget.rs` and used a generic `// STUB: This test will FAIL` pattern. Both are wrong.

The new spec:
1. The 11 STUB: markers in `benchmark_tests.rs` use the ACTUAL format `// STUB: <function_name> <description>`.
2. `aggregate_resource_budget` does not exist as a source file — it must be CREATED as a NEW file at `crates/vb_benchmark/src/aggregate_resource_budget.rs`.
3. The Criterion bench harness also needs to be CREATED as a NEW file at `crates/vb_benchmark/benches/aggregate_resource_budget.rs` (and `benches/` directory added to `Cargo.toml`).

# Scope (verified, no fabrication)

Part 1: Replace 11 STUB: function bodies in `crates/vb_benchmark/tests/benchmark_tests.rs`:
- `baseline_within_budget` (line 10, 20): compare `actual.as_micros() <= budget_us`.
- `budget_utilization_percent` (line 31, 41): compute `(actual.as_micros() * 10_000) / budget_us` (basis points).
- `latency_within_budget` (line 51, 58): compare `elapsed.as_micros() <= budget_us`.
- `result_exceeds_threshold` (line 68, 79): compute threshold and check `result > baseline + delta`.
- `check_evidence_gate` (line 90, 107, 127): check evidence file exists, baseline present, regression within threshold.

Part 2: Create `crates/vb_benchmark/src/aggregate_resource_budget.rs` (NEW):
- Define `pub fn aggregate_resource_budget(runs: &[RunMetrics]) -> ResourceBudgetReport` that aggregates resource usage across 10-100 runs.
- Export it from `crates/vb_benchmark/src/lib.rs`.

Part 3: Create `crates/vb_benchmark/benches/aggregate_resource_budget.rs` (NEW):
- `use criterion::{criterion_group, criterion_main, Criterion};`
- A Criterion bench that calls `aggregate_resource_budget` on 10, 50, 100 runs and reports the median.

Part 4: Add a regression-shield test that asserts STUB: count in `benchmark_tests.rs` is 0:
- `grep -rn "STUB:" crates/vb_benchmark/tests/ | wc -l == 0`

# Acceptance test

```rust
#[test]
fn regression_shield_zero_stub_markers_in_benchmark_tests() {
    let output = std::process::Command::new("grep")
        .args(["-rn", "STUB:", "crates/vb_benchmark/tests/"])
        .output()
        .unwrap();
    let count = String::from_utf8_lossy(&output.stdout).lines().count();
    assert_eq!(count, 0, "expected 0 STUB: markers, found {}", count);
}
```

# Anti-hallucination guards

- DO NOT cite `crates/vb_benchmark/src/aggregate_resource_budget.rs` as existing — it does not. It must be CREATED.
- DO NOT cite `crates/vb_benchmark/benches/` as existing — it does not. The directory must be CREATED.
- DO NOT use `// STUB: This test will FAIL` as the pattern — the actual text is `// STUB: <function_name> <description>`.

# Kani harness (skipped — these are test/bench code; no hot-path contracts)

# Dependency

This bead has NO dependencies.
