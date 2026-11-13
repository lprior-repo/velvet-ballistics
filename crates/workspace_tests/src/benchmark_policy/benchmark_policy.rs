use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::benchmark_policy::error::{Context, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::benchmark_policy::benchmark_budget::{
    self, CURRENT_MILESTONE, PerfBudget, has_deferred_marker, normalized, normalized_set,
};
use crate::benchmark_policy::benchmark_freshness;
use crate::benchmark_policy::write_stdout;

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

pub(crate) fn cmd_benchmark_policy(
    budget: &str,
    evidence: &str,
) -> crate::benchmark_policy::error::Result<()> {
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
) -> crate::benchmark_policy::error::Result<usize> {
    let mut count = 0_usize;
    let mut observed = BTreeSet::new();
    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index.saturating_add(1);
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let claim = parse_evidence_line(root, budget, line, line_number, current_commit)?;
        record_metric(&mut observed, &claim, line_number)?;
        count = count.saturating_add(1);
    }
    validate_metric_coverage(budget, &observed)?;
    Ok(count)
}

include!("claim_validation.rs");

#[cfg(test)]
#[path = "benchmark_policy_tests.rs"]
mod tests;

fn parse_evidence_line(
    root: &Path,
    budget: &PerfBudget,
    line: &str,
    line_number: usize,
    current_commit: &str,
) -> crate::benchmark_policy::error::Result<BenchmarkEvidenceClaim> {
    validate_no_forbidden_markers(line, line_number, "benchmark evidence line")?;
    let claim_value: Value = serde_json::from_str(line)
        .with_context(|| format!("parse benchmark evidence line {line_number}"))?;
    validate_required_evidence_fields(budget, &claim_value, line_number)?;
    let claim: BenchmarkEvidenceClaim = serde_json::from_value(claim_value)
        .with_context(|| format!("decode benchmark evidence line {line_number}"))?;
    validate_claim(root, budget, &claim, line_number, current_commit)?;
    Ok(claim)
}

fn validate_metric_coverage(
    budget: &PerfBudget,
    observed: &BTreeSet<String>,
) -> crate::benchmark_policy::error::Result<()> {
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
    Ok(())
}

fn record_metric(
    observed: &mut BTreeSet<String>,
    claim: &BenchmarkEvidenceClaim,
    line_number: usize,
) -> crate::benchmark_policy::error::Result<()> {
    if !observed.insert(claim.metric.clone()) {
        bail!(
            "benchmark evidence line {line_number} duplicates metric {}",
            claim.metric
        );
    }
    Ok(())
}
