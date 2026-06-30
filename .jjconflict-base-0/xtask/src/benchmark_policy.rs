use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::shell::write_stdout;

const CURRENT_MILESTONE: &str = "ir-interpreter";
const REQUIRED_EVIDENCE_FIELDS: &[&str] = &[
    "metric",
    "claim",
    "baseline_value",
    "result_value",
    "unit",
    "threshold_percent",
    "raw_log",
    "command",
    "commit",
    "rustc_version",
    "nightly_date",
    "cpu_model",
    "cpu_governor",
    "kernel_version",
    "build_profile",
    "rustflags",
    "benchmark_tool_version",
    "sample_count_or_instruction_count",
    "input_fixture_digest",
    "durability_profile",
    "mode",
    "p50_latency",
    "p95_latency",
    "p99_latency",
    "instruction_count",
    "allocation_count",
    "bytes_allocated",
    "fjall_write_latency",
    "direct_api_latency",
    "ipc_latency",
];
const FORBIDDEN_EVIDENCE_MARKERS: &[&str] = &[
    "instructions=not-collected",
    "instruction_count=not-collected",
    "allocations=allocator-external",
    "allocation_count=allocator-external",
];

#[derive(Debug, Deserialize)]
struct PerfBudget {
    current_milestone: String,
    generated_maxperf_deferred: bool,
    deferred_modes: Vec<String>,
    required_evidence_fields: Vec<String>,
    benchmarks: BTreeMap<String, BenchmarkBudget>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkBudget {
    max_regression_percent: u64,
}

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

    let budget = parse_budget(&budget_text)?;
    let current_commit = current_head_commit(&root)?;
    let evidence_count = validate_evidence(&root, &budget, &evidence_text, &current_commit)?;
    write_stdout(format_args!(
        "BenchmarkRegressionPolicyOk budget={} evidence={} claims={evidence_count}",
        budget_path.display(),
        evidence_path.display()
    ))?;
    Ok(())
}

fn parse_budget(text: &str) -> anyhow::Result<PerfBudget> {
    let budget: PerfBudget = serde_yaml::from_str(text).context("parse performance budget yaml")?;
    validate_budget(&budget)?;
    Ok(budget)
}

fn validate_budget(budget: &PerfBudget) -> anyhow::Result<()> {
    if budget.current_milestone != CURRENT_MILESTONE {
        bail!(
            "performance budget milestone must be {CURRENT_MILESTONE}, got {}",
            budget.current_milestone
        );
    }
    if !budget.generated_maxperf_deferred {
        bail!("performance budget must mark generated/maxperf claims as deferred");
    }
    let deferred = normalized_set(&budget.deferred_modes);
    if !deferred.contains("generated") || !deferred.contains("maxperf") {
        bail!("deferred_modes must include generated and maxperf");
    }
    let required = normalized_set(&budget.required_evidence_fields);
    let missing = REQUIRED_EVIDENCE_FIELDS
        .iter()
        .filter(|field| !required.contains(**field))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "performance budget missing evidence fields: {}",
            missing.join(", ")
        );
    }
    if budget.benchmarks.is_empty() {
        bail!("performance budget must define at least one benchmark threshold");
    }
    for (name, threshold) in &budget.benchmarks {
        validate_benchmark_budget(name, threshold)?;
    }
    Ok(())
}

fn validate_benchmark_budget(name: &str, budget: &BenchmarkBudget) -> anyhow::Result<()> {
    if has_deferred_marker(name) {
        bail!("deferred generated/maxperf benchmark must not be budgeted: {name}");
    }
    if budget.max_regression_percent == 0 {
        bail!("benchmark {name} must define a positive max_regression_percent");
    }
    Ok(())
}

fn current_head_commit(root: &Path) -> anyhow::Result<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(root)
        .output()
        .context("run git rev-parse HEAD for benchmark evidence policy")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git rev-parse HEAD failed: {}", stderr.trim());
    }
    let stdout = String::from_utf8(output.stdout).context("decode git rev-parse HEAD output")?;
    let commit = stdout.trim();
    validate_commit_format(commit, "current HEAD")?;
    Ok(commit.to_owned())
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
    validate_current_commit(&claim.commit, current_commit, line_number)?;
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
    validate_commit_format(
        commit,
        &format!("benchmark evidence line {line_number} commit"),
    )
}

fn validate_commit_format(commit: &str, label: &str) -> anyhow::Result<()> {
    let len = commit.len();
    if !(7..=40).contains(&len) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label} must be 7-40 ASCII hex chars");
    }
    Ok(())
}

fn validate_current_commit(
    evidence_commit: &str,
    current_commit: &str,
    line_number: usize,
) -> anyhow::Result<()> {
    let evidence = evidence_commit.to_ascii_lowercase();
    let current = current_commit.to_ascii_lowercase();
    if !current.starts_with(&evidence) {
        bail!(
            "benchmark evidence line {line_number} commit is stale: evidence={evidence_commit} current_head={current_commit}"
        );
    }
    Ok(())
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

fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values.iter().map(|value| normalized(value)).collect()
}

fn has_deferred_marker(value: &str) -> bool {
    let normalized = normalized(value);
    normalized.contains("generated") || normalized.contains("maxperf")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn rejects_stale_benchmark_commit() {
        let error =
            validate_current_commit("deadbeef", "326c7d0f646e437f9adf18378afee0b21c38d522", 1)
                .expect_err("stale benchmark commit is rejected");
        assert!(error.to_string().contains("stale"));
    }

    #[test]
    fn accepts_current_benchmark_commit_prefix() {
        validate_current_commit("326c7d0", "326c7d0f646e437f9adf18378afee0b21c38d522", 1)
            .expect("current commit prefix is valid");
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
}
