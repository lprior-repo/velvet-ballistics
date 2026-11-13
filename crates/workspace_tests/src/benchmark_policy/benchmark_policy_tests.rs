use std::path::Path;

use super::BenchmarkEvidenceClaim;
use super::validate_mode;
use super::{result_exceeds_threshold, validate_evidence, validate_no_forbidden_markers};
use crate::benchmark_policy::benchmark_budget::{PerfBudget, parse_budget};
use crate::benchmark_policy::benchmark_freshness::validate_commit_format;
use crate::benchmark_policy::error::Result;

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
fn parses_unique_budget_benchmark_mapping() -> crate::benchmark_policy::error::Result<()> {
    let budget = parse_budget(VALID_BUDGET)?;
    match budget.benchmarks.get("transition_set") {
        Some(threshold) => assert_eq!(threshold.max_regression_percent, 3),
        None => crate::benchmark_policy::error::bail!("expected transition_set threshold"),
    }
    Ok(())
}

#[test]
fn rejects_duplicate_benchmark_name_at_yaml_parse_stage() -> Result<()> {
    let text = VALID_BUDGET.replace(
        "  transition_set:\n    max_regression_percent: 3\n",
        concat!(
            "  transition_set:\n    max_regression_percent: 5\n",
            "  transition_set:\n    max_regression_percent: 99\n",
        ),
    );
    let error = match parse_budget(&text) {
        Err(error) => error.to_string(),
        Ok(_) => crate::benchmark_policy::error::bail!("expected duplicate benchmark key error"),
    };
    assert!(error.contains("parse performance budget yaml"));
    assert!(error.contains("duplicate entry with key \"transition_set\""));
    Ok(())
}

#[test]
fn rejects_duplicate_top_level_key_at_yaml_parse_stage() -> Result<()> {
    let text = VALID_BUDGET.replace(
        "current_milestone: ir-interpreter",
        "current_milestone: ir-interpreter\ncurrent_milestone: ir-interpreter",
    );
    let error = match parse_budget(&text) {
        Err(error) => error.to_string(),
        Ok(_) => crate::benchmark_policy::error::bail!("expected duplicate top-level key error"),
    };
    assert!(error.contains("parse performance budget yaml"));
    assert!(error.contains("duplicate entry with key \"current_milestone\""));
    Ok(())
}

#[test]
fn rejects_duplicate_inner_field_at_yaml_parse_stage() -> Result<()> {
    let text = VALID_BUDGET.replace(
        "    max_regression_percent: 3",
        "    max_regression_percent: 3\n    max_regression_percent: 3",
    );
    let error = match parse_budget(&text) {
        Err(error) => error.to_string(),
        Ok(_) => crate::benchmark_policy::error::bail!("expected duplicate inner field error"),
    };
    assert!(error.contains("parse performance budget yaml"));
    assert!(error.contains("duplicate entry with key \"max_regression_percent\""));
    Ok(())
}

#[test]
fn rejects_empty_evidence_for_budgeted_metrics() -> crate::benchmark_policy::error::Result<()> {
    let budget = parse_budget(VALID_BUDGET)?;
    match validate_evidence(Path::new("."), &budget, "\n", "326c7d0") {
        Err(error) => {
            assert!(error.to_string().contains("missing budgeted metrics"));
        }
        Ok(_) => {
            crate::benchmark_policy::error::bail!("expected validation error for empty evidence")
        }
    }
    Ok(())
}

#[test]
fn rejects_deferred_generated_budget() -> crate::benchmark_policy::error::Result<()> {
    let text = VALID_BUDGET.replace("transition_set", "generated_mode");
    match parse_budget(&text) {
        Err(error) => {
            assert!(error.to_string().contains("deferred"));
        }
        Ok(_) => crate::benchmark_policy::error::bail!("expected deferred budget to fail"),
    }
    Ok(())
}

#[test]
fn detects_integer_regression_threshold() {
    assert!(!result_exceeds_threshold(103, 100, 3));
    assert!(result_exceeds_threshold(104, 100, 3));
}

#[test]
fn rejects_placeholder_instruction_metadata() -> crate::benchmark_policy::error::Result<()> {
    match validate_no_forbidden_markers(
        "mode=ir;instructions=not-collected",
        1,
        "benchmark evidence raw log",
    ) {
        Err(error) => {
            assert!(error.to_string().contains("instructions=not-collected"));
        }
        Ok(_) => crate::benchmark_policy::error::bail!(
            "expected validation error for placeholder metadata"
        ),
    }
    Ok(())
}

#[test]
fn rejects_non_hex_commit_format() -> crate::benchmark_policy::error::Result<()> {
    let result = validate_commit_format("not-a-valid-commit", "evidence commit");
    let error = match result {
        Ok(()) => crate::benchmark_policy::error::bail!("expected a commit format error"),
        Err(e) => e,
    };
    assert!(error.to_string().contains("7-40 ASCII hex chars"));
    Ok(())
}

#[test]
fn rejects_deferred_mode_in_claim() -> crate::benchmark_policy::error::Result<()> {
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
    let result = validate_mode(&budget, &claim, 1);
    let error = match result {
        Ok(()) => crate::benchmark_policy::error::bail!("expected a deferred mode error"),
        Err(e) => e,
    };
    assert!(error.to_string().contains("deferred mode"));
    Ok(())
}

#[path = "benchmark_budget_boundary_tests.rs"]
mod budget_boundary_tests;
