use std::collections::{BTreeMap, BTreeSet};

use crate::benchmark_policy::error::{Context, bail};
use serde::Deserialize;

#[path = "benchmark_list_values.rs"]
mod list_values;

#[cfg(test)]
#[path = "benchmark_list_values_tests.rs"]
mod list_values_tests;

pub(crate) const CURRENT_MILESTONE: &str = "ir-interpreter";
pub(crate) const REQUIRED_EVIDENCE_FIELDS: &[&str] = &[
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

#[derive(Debug, Deserialize)]
pub(crate) struct PerfBudget {
    pub(crate) current_milestone: String,
    pub(crate) generated_maxperf_deferred: bool,
    pub(crate) deferred_modes: Vec<String>,
    pub(crate) required_evidence_fields: Vec<String>,
    pub(crate) benchmarks: BTreeMap<String, BenchmarkBudget>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BenchmarkBudget {
    pub(crate) max_regression_percent: u64,
}

pub(crate) fn parse_budget(text: &str) -> crate::benchmark_policy::error::Result<PerfBudget> {
    let text = list_values::normalize_lists(text)?;
    let raw: serde_yaml::Value =
        serde_yaml::from_str(&text).context("parse performance budget yaml")?;
    validate_raw_budget(&raw)?;
    let budget: PerfBudget =
        serde_yaml::from_str(&text).context("parse performance budget yaml")?;
    validate_budget(&budget)?;
    Ok(budget)
}

fn validate_raw_budget(raw: &serde_yaml::Value) -> crate::benchmark_policy::error::Result<()> {
    validate_raw_metadata(raw)?;
    let deferred = raw_string_set(raw, "deferred_modes")?;
    if !deferred.contains("generated") || !deferred.contains("maxperf") {
        bail!("deferred_modes must include generated and maxperf");
    }
    let required = raw_string_set(raw, "required_evidence_fields")?;
    validate_required_fields(&required)
}

fn validate_raw_metadata(raw: &serde_yaml::Value) -> crate::benchmark_policy::error::Result<()> {
    let milestone = raw.get("current_milestone");
    if milestone.and_then(serde_yaml::Value::as_str) != Some(CURRENT_MILESTONE) {
        let actual = match milestone {
            None => String::from("None"),
            Some(serde_yaml::Value::String(text)) => text.clone(),
            Some(value) => serde_yaml::to_string(value)
                .context("render budget milestone")?
                .trim()
                .to_owned(),
        };
        bail!("performance budget milestone must be {CURRENT_MILESTONE}, got {actual}");
    }
    if raw
        .get("generated_maxperf_deferred")
        .and_then(serde_yaml::Value::as_bool)
        != Some(true)
    {
        bail!("performance budget must mark generated/maxperf claims as deferred");
    }
    Ok(())
}

fn raw_string_set(
    raw: &serde_yaml::Value,
    field: &str,
) -> crate::benchmark_policy::error::Result<BTreeSet<String>> {
    let values = raw
        .get(field)
        .and_then(serde_yaml::Value::as_sequence)
        .filter(|items| items.iter().all(serde_yaml::Value::is_string))
        .with_context(|| format!("performance budget {field} must be a list of strings"))?;
    Ok(values
        .iter()
        .filter_map(serde_yaml::Value::as_str)
        .map(normalized)
        .collect())
}

pub(crate) fn validate_budget(budget: &PerfBudget) -> crate::benchmark_policy::error::Result<()> {
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
    validate_required_budget_fields(budget)?;
    if budget.benchmarks.is_empty() {
        bail!("performance budget must define at least one benchmark threshold");
    }
    for (name, threshold) in &budget.benchmarks {
        validate_benchmark_budget(name, threshold)?;
    }
    Ok(())
}

fn validate_benchmark_budget(
    name: &str,
    budget: &BenchmarkBudget,
) -> crate::benchmark_policy::error::Result<()> {
    if has_deferred_marker(name) {
        bail!("deferred generated/maxperf benchmark must not be budgeted: {name}");
    }
    if budget.max_regression_percent == 0 {
        bail!("benchmark {name} must define a positive max_regression_percent");
    }
    Ok(())
}

pub(crate) fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(crate) fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values.iter().map(|value| normalized(value)).collect()
}

pub(crate) fn has_deferred_marker(value: &str) -> bool {
    let normalized = normalized(value);
    normalized.contains("generated") || normalized.contains("maxperf")
}

fn validate_required_budget_fields(
    budget: &PerfBudget,
) -> crate::benchmark_policy::error::Result<()> {
    let required = normalized_set(&budget.required_evidence_fields);
    validate_required_fields(&required)
}

fn validate_required_fields(
    required: &BTreeSet<String>,
) -> crate::benchmark_policy::error::Result<()> {
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
    Ok(())
}
