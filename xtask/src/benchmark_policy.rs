#![allow(unreachable_pub)]
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::benchmark_budget::{
    self, CURRENT_MILESTONE, PerfBudget, has_deferred_marker, normalized, normalized_set,
};
use crate::benchmark_freshness;
use crate::shell::write_stdout;

const FORBIDDEN_EVIDENCE_MARKERS: &[&str] = &[
    "instructions=not-collected",
    "instruction_count=not-collected",
    "allocations=allocator-external",
    "allocation_count=allocator-external",
];

#[derive(Debug, Deserialize)]
struct BenchmarkEvidenceClaim {
    metric: String,
    claim: String,
    baseline_value: u64,
    result_value: u64,
    unit: String,
    threshold_percent: u64,
    raw_log: String,
    command: String,
    commit: String,
    mode: String,
}

pub(crate) fn cmd_benchmark_policy(budget: &str, evidence: &str) -> anyhow::Result<()> {
    let root = std::env::current_dir().context("resolve current directory")?;
    let budget_path = root.join(budget);
    let evidence_path = root.join(evidence);
    let budget_text = fs::read_to_string(&budget_path)
        .with_context(|| format!("read performance budget {}", budget_path.display()))?;
    let evidence_text = fs::read_to_string(&evidence_path)
        .with_context(|| format!("read benchmark evidence {}", evidence_path.display()))?;

    let budget = benchmark_budget::parse_budget(&budget_text)?;
    let current_commit = benchmark_freshness::current_head_commit(&root)?;
    let evidence_count = validate_evidence(&root, &budget, &evidence_text, &current_commit)?;
    write_stdout(format_args!(
        "BenchmarkRegressionPolicyOk budget={} evidence={} claims={evidence_count}",
        budget_path.display(),
        evidence_path.display()
    ))?;
    Ok(())
}

fn validate_evidence(
    root: &Path,
    budget: &PerfBudget,
    text: &str,
    current_commit: &str,
) -> anyhow::Result<usize> {
    let mut count = 0_usize;
    let mut observed = BTreeSet::new();
    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index.saturating_add(1);
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        validate_no_forbidden_markers(line, line_number, "benchmark evidence line")?;
        let claim_value: Value = serde_json::from_str(line)
            .with_context(|| format!("parse benchmark evidence line {line_number}"))?;
        validate_required_evidence_fields(budget, &claim_value, line_number)?;
        let claim: BenchmarkEvidenceClaim = serde_json::from_value(claim_value)
            .with_context(|| format!("decode benchmark evidence line {line_number}"))?;
        validate_claim(root, budget, &claim, line_number, current_commit)?;
        if !observed.insert(claim.metric.clone()) {
            bail!(
                "benchmark evidence line {line_number} duplicates metric {}",
                claim.metric
            );
        }
        count = count.saturating_add(1);
    }
    let missing = budget
        .benchmarks
        .keys()
        .filter(|metric| !observed.contains(*metric))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "benchmark evidence missing budgeted metrics: {}",
            missing.join(", ")
        );
    }
    Ok(count)
}

fn validate_claim(
    root: &Path,
    budget: &PerfBudget,
    claim: &BenchmarkEvidenceClaim,
    line_number: usize,
    current_commit: &str,
) -> anyhow::Result<()> {
    validate_required_strings(claim, line_number)?;
    validate_mode(budget, claim, line_number)?;
    validate_commit(&claim.commit, line_number)?;
    benchmark_freshness::validate_evidence_freshness(
        root,
        &claim.commit,
        current_commit,
        line_number,
    )?;
    validate_raw_log(root, &claim.raw_log, line_number)?;
    let Some(metric_budget) = budget.benchmarks.get(&claim.metric) else {
        bail!(
            "benchmark evidence line {line_number} references unbudgeted metric {}",
            claim.metric
        );
    };
    if claim.threshold_percent != metric_budget.max_regression_percent {
        bail!(
            "benchmark evidence line {line_number} threshold {} does not match budget {} for {}",
            claim.threshold_percent,
            metric_budget.max_regression_percent,
            claim.metric
        );
    }
    if result_exceeds_threshold(
        claim.result_value,
        claim.baseline_value,
        metric_budget.max_regression_percent,
    ) {
        bail!(
            "benchmark evidence line {line_number} regressed: metric={} baseline={} result={} threshold_percent={}",
            claim.metric,
            claim.baseline_value,
            claim.result_value,
            metric_budget.max_regression_percent
        );
    }
    Ok(())
}

fn validate_required_evidence_fields(
    budget: &PerfBudget,
    claim: &Value,
    line_number: usize,
) -> anyhow::Result<()> {
    let Some(object) = claim.as_object() else {
        bail!("benchmark evidence line {line_number} must be a JSON object");
    };
    let missing = budget
        .required_evidence_fields
        .iter()
        .filter(|field| {
            object
                .get(field.as_str())
                .is_none_or(|value| !evidence_value_present(value))
        })
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "benchmark evidence line {line_number} missing required metadata fields: {}",
            missing.join(", ")
        );
    }
    Ok(())
}

fn evidence_value_present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}

fn validate_required_strings(
    claim: &BenchmarkEvidenceClaim,
    line_number: usize,
) -> anyhow::Result<()> {
    let required = [
        ("metric", claim.metric.as_str()),
        ("claim", claim.claim.as_str()),
        ("unit", claim.unit.as_str()),
        ("raw_log", claim.raw_log.as_str()),
        ("command", claim.command.as_str()),
        ("commit", claim.commit.as_str()),
        ("mode", claim.mode.as_str()),
    ];
    let missing = required
        .iter()
        .filter_map(|(field, value)| value.trim().is_empty().then_some(*field))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "benchmark evidence line {line_number} has empty required fields: {}",
            missing.join(", ")
        );
    }
    if has_deferred_marker(&claim.claim) {
        bail!("benchmark evidence line {line_number} contains deferred generated/maxperf claim");
    }
    Ok(())
}

fn validate_mode(
    budget: &PerfBudget,
    claim: &BenchmarkEvidenceClaim,
    line_number: usize,
) -> anyhow::Result<()> {
    let mode = normalized(&claim.mode);
    let deferred = normalized_set(&budget.deferred_modes);
    if deferred.contains(mode.as_str()) || has_deferred_marker(&mode) {
        bail!("benchmark evidence line {line_number} uses deferred mode {mode}");
    }
    if mode != CURRENT_MILESTONE {
        bail!("benchmark evidence line {line_number} mode must be {CURRENT_MILESTONE}, got {mode}");
    }
    Ok(())
}

fn validate_commit(commit: &str, line_number: usize) -> anyhow::Result<()> {
    benchmark_freshness::validate_commit_format(
        commit,
        &format!("benchmark evidence line {line_number} commit"),
    )
}

fn validate_raw_log(root: &Path, raw_log: &str, line_number: usize) -> anyhow::Result<()> {
    let path = PathBuf::from(raw_log);
    if path.is_absolute() {
        bail!("benchmark evidence line {line_number} raw_log must be workspace-relative");
    }
    let joined = root.join(path);
    if !joined.is_file() {
        bail!(
            "benchmark evidence line {line_number} raw_log does not exist: {}",
            joined.display()
        );
    }
    let log_text = fs::read_to_string(&joined)
        .with_context(|| format!("read benchmark evidence raw log {}", joined.display()))?;
    validate_no_forbidden_markers(&log_text, line_number, "benchmark evidence raw log")?;
    Ok(())
}

fn validate_no_forbidden_markers(
    text: &str,
    line_number: usize,
    label: &str,
) -> anyhow::Result<()> {
    for marker in FORBIDDEN_EVIDENCE_MARKERS {
        if text.contains(marker) {
            bail!("{label} {line_number} contains forbidden placeholder metadata: {marker}");
        }
    }
    Ok(())
}

fn result_exceeds_threshold(result: u64, baseline: u64, threshold_percent: u64) -> bool {
    let baseline = u128::from(baseline);
    let result = u128::from(result);
    let threshold_delta = baseline.saturating_mul(u128::from(threshold_percent)) / 100;
    result > baseline.saturating_add(threshold_delta)
}

#[cfg(test)]
#[path = "benchmark_policy_tests.rs"]
mod tests;
