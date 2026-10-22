use std::path::Path;

use anyhow::Context;

use super::BenchmarkEvidenceClaim;
use super::validate_mode;
use super::{result_exceeds_threshold, validate_evidence, validate_no_forbidden_markers};
use crate::benchmark_budget::{PerfBudget, parse_budget};
use crate::benchmark_freshness::validate_commit_format;

const VALID_BUDGET: &str = r#"
current_milestone: ir-interpreter
generated_maxperf_deferred: true
deferred_modes:
  - generated
  - maxperf
required_evidence_fields:
  - metric
  - claim
  - baseline_value
  - result_value
  - unit
  - threshold_percent
  - raw_log
  - command
  - commit
  - rustc_version
  - nightly_date
  - cpu_model
  - cpu_governor
  - kernel_version
  - build_profile
  - rustflags
  - benchmark_tool_version
  - sample_count_or_instruction_count
  - input_fixture_digest
  - durability_profile
  - mode
  - p50_latency
  - p95_latency
  - p99_latency
  - instruction_count
  - allocation_count
  - bytes_allocated
  - fjall_write_latency
  - direct_api_latency
  - ipc_latency
benchmarks:
  transition_set:
    max_regression_percent: 3
"#;

#[test]
fn rejects_empty_evidence_for_budgeted_metrics() {
    let budget = parse_budget(VALID_BUDGET).expect("valid budget");
    let error = validate_evidence(Path::new("."), &budget, "\n", "326c7d0")
        .expect_err("budgeted metrics require evidence");
    assert!(error.to_string().contains("missing budgeted metrics"));
}

#[test]
fn rejects_deferred_generated_budget() {
    let text = VALID_BUDGET.replace("transition_set", "generated_mode");
    let error = parse_budget(&text).expect_err("generated budget is deferred");
    assert!(error.to_string().contains("deferred"));
}

#[test]
fn detects_integer_regression_threshold() {
    assert!(!result_exceeds_threshold(103, 100, 3));
    assert!(result_exceeds_threshold(104, 100, 3));
}

#[test]
fn rejects_placeholder_instruction_metadata() {
    let error = validate_no_forbidden_markers(
        "mode=ir;instructions=not-collected",
        1,
        "benchmark evidence raw log",
    )
    .expect_err("placeholder instruction metadata is rejected");
    assert!(error.to_string().contains("instructions=not-collected"));
}

#[test]
fn rejects_non_hex_commit_format() -> anyhow::Result<()> {
    let error = validate_commit_format("not-a-valid-commit", "evidence commit")
        .err()
        .context("expected a commit format error")?;
    assert!(error.to_string().contains("7-40 ASCII hex chars"));
    Ok(())
}

#[test]
fn rejects_deferred_mode_in_claim() -> anyhow::Result<()> {
    let budget: PerfBudget = parse_budget(VALID_BUDGET)?;
    let claim: BenchmarkEvidenceClaim = serde_json::from_value(serde_json::json!({
        "metric": "transition_set",
        "claim": "baseline snapshot",
        "baseline_value": 100,
        "result_value": 100,
        "unit": "ns",
        "threshold_percent": 3,
        "raw_log": "evidence/benchmark-logs/test.log",
        "command": "cargo bench",
        "commit": "0123456789abcdef0123456789abcdef01234567",
        "mode": "generated",
    }))?;
    let error = validate_mode(&budget, &claim, 1)
        .err()
        .context("expected a deferred mode error")?;
    assert!(error.to_string().contains("deferred mode"));
    Ok(())
}
