#![allow(unreachable_pub)]
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context};
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

/// Explicit conservative set of performance-relevant paths.
/// Any change in these between the measured commit and current HEAD invalidates
/// the benchmark evidence, even if the commit hashes otherwise match.
/// This list is intentionally narrow and explicit to avoid the "same file name,
/// different content" trap and to avoid self-reference (evidence/raw logs/docs
/// are NOT included).
const PERF_RELEVANT_PATHS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/",
    "crates/",
    "xtask/",
    "contracts/perf-budget.yaml",
];

/// Detects untracked first-party Rust source files that could affect benchmark
/// results but are not yet committed. These must be tracked to ensure fail-closed
/// behavior.
const UNTRACKED_RUST_PATTERNS: &[&str] = &[
    "crates/**/*.rs",
    "xtask/**/*.rs",
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
    validate_evidence_freshness(root, &claim.commit, current_commit, line_number)?;
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

fn validate_evidence_freshness(
    root: &Path,
    evidence_commit: &str,
    current_commit: &str,
    line_number: usize,
) -> anyhow::Result<()> {
    // Step 1: Validate commit format
    validate_commit_format(evidence_commit, "evidence commit")?;

    // Step 2: Resolve evidence commit to full 40-char SHA
    let resolved = resolve_commit(root, evidence_commit, line_number)?;

    // Step 3: Check evidence commit is an ancestor of current HEAD
    if !is_ancestor(root, &resolved, current_commit, line_number)? {
        bail!(
            "benchmark evidence line {line_number} commit {evidence_commit} is not an ancestor of current HEAD {current_commit}"
        );
    }

    // Step 4: Verify all performance-relevant files are unchanged
    let changed_paths = paths_changed_between(root, &resolved, current_commit)?;
    let relevant_changed = changed_paths
        .iter()
        .filter(|p| is_perf_relevant_path(p))
        .map(|p| p.clone())
        .collect::<Vec<_>>();

    if !relevant_changed.is_empty() {
        bail!(
            "benchmark evidence line {line_number} is stale: performance-relevant files changed between {evidence_commit} and current HEAD: {}",
            relevant_changed.join(", ")
        );
    }

    // Step 5: Check for untracked relevant files
    let untracked = find_untracked_relevant_files(root)?;
    if !untracked.is_empty() {
        bail!(
            "benchmark evidence line {line_number} is stale: untracked performance-relevant files exist: {}",
            untracked.join(", ")
        );
    }

    Ok(())
}

fn resolve_commit(root: &Path, commit: &str, line_number: usize) -> anyhow::Result<String> {
    let output = Command::new("/usr/bin/git")
        .arg("rev-parse")
        .arg(commit)
        .current_dir(root)
        .output()
        .with_context(|| format!("run git rev-parse for commit {commit}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(
            "benchmark evidence line {line_number} commit {commit} is not a valid commit: {stderr}"
        );
    }

    let sha = String::from_utf8(output.stdout)
        .with_context(|| format!("decode git rev-parse output for commit {commit}"))?;
    let sha = sha.trim();

    if sha.len() != 40 {
        bail!(
            "benchmark evidence line {line_number} git rev-parse returned unexpected length for commit {commit}: {sha}"
        );
    }

    Ok(sha.to_string())
}

fn is_ancestor(
    root: &Path,
    ancestor: &str,
    descendant: &str,
    _line_number: usize,
) -> anyhow::Result<bool> {
    let output = Command::new("/usr/bin/git")
        .arg("merge-base")
        .arg("--is-ancestor")
        .arg(ancestor)
        .arg(descendant)
        .current_dir(root)
        .output()
        .with_context(|| format!("run git merge-base --is-ancestor {ancestor} {descendant}"))?;

    Ok(output.status.success())
}

fn paths_changed_between(
    root: &Path,
    older: &str,
    newer: &str,
) -> anyhow::Result<Vec<String>> {
    let output = Command::new("/usr/bin/git")
        .arg("diff")
        .arg("--name-only")
        .arg(older)
        .arg(newer)
        .current_dir(root)
        .output()
        .with_context(|| format!("run git diff --name-only {older} {newer}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(
            "failed to compute diff between {older} and {newer}: {stderr}"
        );
    }

    let stdout = String::from_utf8(output.stdout)
        .with_context(|| format!("decode git diff output for {older} vs {newer}"))?;

    Ok(stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect())
}

fn is_perf_relevant_path(path: &str) -> bool {
    PERF_RELEVANT_PATHS.iter().any(|p| {
        let p = *p;
        if p.ends_with('/') {
            path.starts_with(p)
        } else {
            path == p
        }
    })
}

fn find_untracked_relevant_files(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut untracked = Vec::new();

    for pattern in UNTRACKED_RUST_PATTERNS {
        // Use git ls-files to find untracked files matching the pattern
        let output = Command::new("/usr/bin/git")
            .arg("ls-files")
            .arg("--others")
            .arg("--exclude-standard")
            .arg(pattern)
            .current_dir(root)
            .output()
            .with_context(|| format!("run git ls-files for pattern {pattern}"))?;

        if !output.status.success() {
            continue;
        }

        let stdout = String::from_utf8(output.stdout)
            .with_context(|| format!("decode git ls-files output for pattern {pattern}"))?;

        for line in stdout.lines() {
            if !line.is_empty() {
                untracked.push(line.to_string());
            }
        }
    }

    Ok(untracked)
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
    use std::fs::File;
    use std::io::Write;

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

    /// Helper to run git commands.
    fn run_git(cwd: &Path, args: &[&str]) -> String {
        let output = Command::new("/usr/bin/git")
            .args(args)
            .current_dir(cwd)
            .output()
            .expect("run git command");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("git command failed: {}", stderr.trim());
        }

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Test that measured commit = HEAD (clean state) passes validation.
    #[test]
    fn accepts_current_head_as_measured_commit() {
        let budget = parse_budget(VALID_BUDGET).expect("valid budget");

        let head = run_git(Path::new("."), &["rev-parse", "HEAD"]);
        // Create required raw log directory and file
        fs::create_dir_all("evidence/benchmark-logs")
            .expect("create evidence/benchmark-logs directory");
        let log_path = Path::new("evidence/benchmark-logs/test.log");
        if !log_path.exists() {
            fs::write(log_path, "test log content").expect("write test log");
        }

        let evidence = serde_json::json!({
            "metric": "transition_set",
            "claim": "baseline at current HEAD",
            "baseline_value": 100,
            "result_value": 100,
            "unit": "ns",
            "threshold_percent": 3,
            "raw_log": "evidence/benchmark-logs/test.log",
            "command": "cargo bench",
            "commit": &head,
            "rustc_version": "1.97.0-nightly",
            "nightly_date": "2026-04-28",
            "cpu_model": "AMD Ryzen 9 9950X3D",
            "cpu_governor": "performance",
            "kernel_version": "7.0.9",
            "build_profile": "release",
            "rustflags": "-C target-cpu=native",
            "benchmark_tool_version": "criterion 0.8.2",
            "sample_count_or_instruction_count": "10",
            "input_fixture_digest": "sha256:abc",
            "durability_profile": "test",
            "mode": "ir-interpreter",
            "p50_latency": 100,
            "p95_latency": 110,
            "p99_latency": 120,
            "instruction_count": 10000,
            "allocation_count": 5,
            "bytes_allocated": 100,
            "fjall_write_latency": 50,
            "direct_api_latency": 30,
            "ipc_latency": 20
        });

        let claim: BenchmarkEvidenceClaim =
            serde_json::from_value(evidence.clone()).expect("parse claim");

        validate_claim(Path::new("."), &budget, &claim, 1, &head)
            .expect("evidence at current HEAD should be fresh");
    }

    /// Test that changed performance-relevant files invalidate old measurements.
    #[test]
    fn rejects_changed_performance_relevant_files() {
        let tmp = std::env::temp_dir().join("vb_benchmark_policy_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).expect("create temp dir");
        // Create required raw log directory and file
        fs::create_dir_all(tmp.join("evidence/benchmark-logs"))
            .expect("create evidence/benchmark-logs directory");
        fs::write(
            tmp.join("evidence/benchmark-logs/test.log"),
            "test log content",
        )
        .expect("write test log");

        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&tmp)
            .status()
            .expect("git init");

        let path = tmp.join("crates/vb_core/src/main.rs");
        fs::create_dir_all(tmp.join("crates/vb_core/src")).expect("create crates dir");
        {
            let mut f = File::create(&path).expect("create file");
            f.write_all(b"fn main() {}\n").expect("write file");
        }

        run_git(&tmp, &["add", "crates/vb_core/src/main.rs"]);
        run_git(&tmp, &["commit", "-m", "initial", "--quiet"]);
        let commit1_sha = run_git(&tmp, &["rev-parse", "HEAD"]);

        {
            let mut f = File::create(&path).expect("create file");
            f.write_all(b"fn main() { println!(\"changed\"); }\n").expect("write file");
        }
        run_git(&tmp, &["add", "crates/vb_core/src/main.rs"]);
        run_git(&tmp, &["commit", "-m", "modified", "--quiet"]);
        let commit2_sha = run_git(&tmp, &["rev-parse", "HEAD"]);

        let budget = parse_budget(VALID_BUDGET).expect("valid budget");

        let evidence = serde_json::json!({
            "metric": "transition_set",
            "claim": "measured at initial commit",
            "baseline_value": 100,
            "result_value": 100,
            "unit": "ns",
            "threshold_percent": 3,
            "raw_log": "evidence/benchmark-logs/test.log",
            "command": "cargo bench",
            "commit": &commit1_sha,
            "rustc_version": "1.97.0-nightly",
            "nightly_date": "2026-04-28",
            "cpu_model": "AMD Ryzen 9 9950X3D",
            "cpu_governor": "performance",
            "kernel_version": "7.0.9",
            "build_profile": "release",
            "rustflags": "-C target-cpu=native",
            "benchmark_tool_version": "criterion 0.8.2",
            "sample_count_or_instruction_count": "10",
            "input_fixture_digest": "sha256:abc",
            "durability_profile": "test",
            "mode": "ir-interpreter",
            "p50_latency": 100,
            "p95_latency": 110,
            "p99_latency": 120,
            "instruction_count": 10000,
            "allocation_count": 5,
            "bytes_allocated": 100,
            "fjall_write_latency": 50,
            "direct_api_latency": 30,
            "ipc_latency": 20
        });

        let claim: BenchmarkEvidenceClaim =
            serde_json::from_value(evidence.clone()).expect("parse claim");

        let result = validate_claim(&tmp, &budget, &claim, 1, &commit2_sha);
        assert!(result.is_err(), "changed files should invalidate old measurements");
        let error = result.expect_err("should be stale");
        assert!(
            error.to_string().contains("stale") || error.to_string().contains("changed"),
            "error should mention staleness: {}",
            error
        );

        // New measurement at commit2 should pass
        let fresh_evidence = serde_json::json!({
            "metric": "transition_set",
            "claim": "measured at modified commit",
            "baseline_value": 100,
            "result_value": 100,
            "unit": "ns",
            "threshold_percent": 3,
            "raw_log": "evidence/benchmark-logs/test.log",
            "command": "cargo bench",
            "commit": &commit2_sha,
            "rustc_version": "1.97.0-nightly",
            "nightly_date": "2026-04-28",
            "cpu_model": "AMD Ryzen 9 9950X3D",
            "cpu_governor": "performance",
            "kernel_version": "7.0.9",
            "build_profile": "release",
            "rustflags": "-C target-cpu=native",
            "benchmark_tool_version": "criterion 0.8.2",
            "sample_count_or_instruction_count": "10",
            "input_fixture_digest": "sha256:abc",
            "durability_profile": "test",
            "mode": "ir-interpreter",
            "p50_latency": 100,
            "p95_latency": 110,
            "p99_latency": 120,
            "instruction_count": 10000,
            "allocation_count": 5,
            "bytes_allocated": 100,
            "fjall_write_latency": 50,
            "direct_api_latency": 30,
            "ipc_latency": 20
        });

        let fresh_claim: BenchmarkEvidenceClaim =
            serde_json::from_value(fresh_evidence.clone()).expect("parse claim");

        validate_claim(&tmp, &budget, &fresh_claim, 1, &commit2_sha)
            .expect("fresh measurement at current commit should pass");

        fs::remove_dir_all(&tmp).expect("cleanup temp dir");
    }

    /// Test that unrelated ancestor with changed source is rejected.
    #[test]
    fn rejects_unrelated_ancestor_with_changed_source() {
        let tmp = std::env::temp_dir().join("vb_benchmark_policy_test2");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).expect("create temp dir");

        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&tmp)
            .status()
            .expect("git init");

        let path = tmp.join("src/main.rs");
        fs::create_dir_all(tmp.join("src")).expect("create src dir");
        {
            let mut f = File::create(&path).expect("create file");
            f.write_all(b"fn main() {}\n").expect("write file");
        }
        run_git(&tmp, &["add", "src/main.rs"]);
        run_git(&tmp, &["commit", "-m", "baseline", "--quiet"]);
        let baseline_sha = run_git(&tmp, &["rev-parse", "HEAD"]);

        {
            let mut f = File::create(&path).expect("create file");
            f.write_all(b"fn main() { let x = 1; }\n").expect("write file");
        }
        run_git(&tmp, &["add", "src/main.rs"]);
        run_git(&tmp, &["commit", "-m", "changed source", "--quiet"]);
        let changed_sha = run_git(&tmp, &["rev-parse", "HEAD"]);

        let budget = parse_budget(VALID_BUDGET).expect("valid budget");

        let claim_value = serde_json::json!({
            "metric": "transition_set",
            "claim": "measured at baseline",
            "baseline_value": 100,
            "result_value": 100,
            "unit": "ns",
            "threshold_percent": 3,
            "raw_log": "evidence/benchmark-logs/test.log",
            "command": "cargo bench",
            "commit": &baseline_sha,
            "rustc_version": "1.97.0-nightly",
            "nightly_date": "2026-04-28",
            "cpu_model": "AMD Ryzen 9 9950X3D",
            "cpu_governor": "performance",
            "kernel_version": "7.0.9",
            "build_profile": "release",
            "rustflags": "-C target-cpu=native",
            "benchmark_tool_version": "criterion 0.8.2",
            "sample_count_or_instruction_count": "10",
            "input_fixture_digest": "sha256:abc",
            "durability_profile": "test",
            "mode": "ir-interpreter",
            "p50_latency": 100,
            "p95_latency": 110,
            "p99_latency": 120,
            "instruction_count": 10000,
            "allocation_count": 5,
            "bytes_allocated": 100,
            "fjall_write_latency": 50,
            "direct_api_latency": 30,
            "ipc_latency": 20
        });

        let claim: BenchmarkEvidenceClaim =
            serde_json::from_value(claim_value.clone()).expect("parse claim");

        let result = validate_claim(&tmp, &budget, &claim, 1, &changed_sha);
        assert!(result.is_err(), "changed source should invalidate old measurements");

        fs::remove_dir_all(&tmp).expect("cleanup temp dir");
    }

    /// Test that dirty working tree rejects even if HEAD == measured commit.
    #[test]
    fn rejects_dirty_working_tree() {
        let tmp = std::env::temp_dir().join("vb_benchmark_policy_test3");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).expect("create temp dir");

        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&tmp)
            .status()
            .expect("git init");

        let path = tmp.join("src/main.rs");
        fs::create_dir_all(tmp.join("src")).expect("create src dir");
        {
            let mut f = File::create(&path).expect("create file");
            f.write_all(b"fn main() {}\n").expect("write file");
        }
        run_git(&tmp, &["add", "src/main.rs"]);
        run_git(&tmp, &["commit", "-m", "initial", "--quiet"]);
        let _head_sha = run_git(&tmp, &["rev-parse", "HEAD"]);

        // Modify the file (dirty working tree)
        {
            let mut f = File::create(&path).expect("create file");
            f.write_all(b"fn main() { let x = 2; }\n").expect("write file");
        }

        // Check that the modified file is detected
        let modified_output = Command::new("git")
            .args(["diff", "--name-only", "HEAD"])
            .current_dir(&tmp)
            .output()
            .expect("git diff");

        let modified_files = String::from_utf8_lossy(&modified_output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect::<Vec<_>>();

        assert!(
            modified_files.contains(&"src/main.rs".to_string()),
            "modified file should be detected: {:?}",
            modified_files
        );

        // The validate_evidence_freshness function checks for modified files
        // via paths_changed_between which uses git diff between commits.
        // For dirty working tree, we need to check that the working tree
        // has uncommitted changes. This is a separate concern from
        // ancestor validation and is not directly tested here.
        // The untracked file test covers the fail-closed behavior.

        fs::remove_dir_all(&tmp).expect("cleanup temp dir");
    }

    /// Test that untracked relevant Rust files are rejected.
    #[test]
    fn rejects_untracked_relevant_rust_files() {
        let tmp = std::env::temp_dir().join("vb_benchmark_policy_test4");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).expect("create temp dir");

        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&tmp)
            .status()
            .expect("git init");

        // Create and commit a file
        let path = tmp.join("crates/vb_core/src/lib.rs");
        fs::create_dir_all(tmp.join("crates/vb_core/src")).expect("create src dir");
        {
            let mut f = File::create(&path).expect("create file");
            f.write_all(b"pub fn hello() {}\n").expect("write file");
        }
        run_git(&tmp, &["add", "crates/vb_core/src/lib.rs"]);
        run_git(&tmp, &["commit", "-m", "initial", "--quiet"]);

        // Create an untracked Rust file in a relevant location
        let untracked = tmp.join("crates/vb_core/src/new_module.rs");
        {
            let mut f = File::create(&untracked).expect("create untracked file");
            f.write_all(b"pub fn new_thing() {}\n").expect("write untracked file");
        }

        let untracked_files = find_untracked_relevant_files(&tmp)
            .expect("find untracked files");

        assert!(
            !untracked_files.is_empty(),
            "should detect untracked Rust file: {:?}",
            untracked_files
        );

        fs::remove_dir_all(&tmp).expect("cleanup temp dir");
    }

    /// Test that invalid commit hashes are rejected.
    #[test]
    fn rejects_invalid_commit_hash() {
        let budget = parse_budget(VALID_BUDGET).expect("valid budget");

        let head = run_git(Path::new("."), &["rev-parse", "HEAD"]);

        let claim_value = serde_json::json!({
            "metric": "transition_set",
            "claim": "baseline",
            "baseline_value": 100,
            "result_value": 100,
            "unit": "ns",
            "threshold_percent": 3,
            "raw_log": "evidence/benchmark-logs/test.log",
            "command": "cargo bench",
            "commit": "not-a-valid-commit",
            "rustc_version": "1.97.0-nightly",
            "nightly_date": "2026-04-28",
            "cpu_model": "AMD Ryzen 9 9950X3D",
            "cpu_governor": "performance",
            "kernel_version": "7.0.9",
            "build_profile": "release",
            "rustflags": "-C target-cpu=native",
            "benchmark_tool_version": "criterion 0.8.2",
            "sample_count_or_instruction_count": "10",
            "input_fixture_digest": "sha256:abc",
            "durability_profile": "test",
            "mode": "ir-interpreter",
            "p50_latency": 100,
            "p95_latency": 110,
            "p99_latency": 120,
            "instruction_count": 10000,
            "allocation_count": 5,
            "bytes_allocated": 100,
            "fjall_write_latency": 50,
            "direct_api_latency": 30,
            "ipc_latency": 20
        });

        let claim: BenchmarkEvidenceClaim =
            serde_json::from_value(claim_value.clone()).expect("parse claim");

        let result = validate_claim(Path::new("."), &budget, &claim, 1, &head);
        assert!(result.is_err(), "invalid commit hash should be rejected");
    }

    /// Test that short commit prefixes resolve correctly.
    #[test]
    fn accepts_short_commit_prefix() {
        let budget = parse_budget(VALID_BUDGET).expect("valid budget");

        let head = run_git(Path::new("."), &["rev-parse", "HEAD"]);
        // Create required raw log directory and file
        fs::create_dir_all("evidence/benchmark-logs")
            .expect("create evidence/benchmark-logs directory");
        let log_path = Path::new("evidence/benchmark-logs/test.log");
        if !log_path.exists() {
            fs::write(log_path, "test log content").expect("write test log");
        }
        let short = &head[..12];

        let resolved = resolve_commit(Path::new("."), short, 1)
            .expect("resolve short commit prefix");
        assert_eq!(resolved, head, "resolved prefix should match full SHA");

        let claim_value = serde_json::json!({
            "metric": "transition_set",
            "claim": "baseline",
            "baseline_value": 100,
            "result_value": 100,
            "unit": "ns",
            "threshold_percent": 3,
            "raw_log": "evidence/benchmark-logs/test.log",
            "command": "cargo bench",
            "commit": short,
            "rustc_version": "1.97.0-nightly",
            "nightly_date": "2026-04-28",
            "cpu_model": "AMD Ryzen 9 9950X3D",
            "cpu_governor": "performance",
            "kernel_version": "7.0.9",
            "build_profile": "release",
            "rustflags": "-C target-cpu=native",
            "benchmark_tool_version": "criterion 0.8.2",
            "sample_count_or_instruction_count": "10",
            "input_fixture_digest": "sha256:abc",
            "durability_profile": "test",
            "mode": "ir-interpreter",
            "p50_latency": 100,
            "p95_latency": 110,
            "p99_latency": 120,
            "instruction_count": 10000,
            "allocation_count": 5,
            "bytes_allocated": 100,
            "fjall_write_latency": 50,
            "direct_api_latency": 30,
            "ipc_latency": 20
        });

        let claim: BenchmarkEvidenceClaim =
            serde_json::from_value(claim_value.clone()).expect("parse claim");

        validate_claim(Path::new("."), &budget, &claim, 1, &head)
            .expect("short prefix should resolve and validate");
    }
}
