#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::Deserialize;

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
    "mode",
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
    let evidence_count = validate_evidence(&root, &budget, &evidence_text)?;
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

fn validate_evidence(root: &Path, budget: &PerfBudget, text: &str) -> anyhow::Result<usize> {
    let mut count = 0_usize;
    let mut observed = BTreeSet::new();
    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index.saturating_add(1);
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let claim: BenchmarkEvidenceClaim = serde_json::from_str(line)
            .with_context(|| format!("parse benchmark evidence line {line_number}"))?;
        validate_claim(root, budget, &claim, line_number)?;
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
) -> anyhow::Result<()> {
    validate_required_strings(claim, line_number)?;
    validate_mode(budget, claim, line_number)?;
    validate_commit(&claim.commit, line_number)?;
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
    let len = commit.len();
    if !(7..=40).contains(&len) || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("benchmark evidence line {line_number} commit must be 7-40 ASCII hex chars");
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
  - mode
benchmarks:
  transition_set:
    max_regression_percent: 3
"#;

    #[test]
    fn rejects_empty_evidence_for_budgeted_metrics() {
        let budget = parse_budget(VALID_BUDGET).expect("valid budget");
        let error = validate_evidence(Path::new("."), &budget, "\n")
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
}
