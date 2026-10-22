use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, bail};
use serde::Deserialize;

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

pub(crate) fn parse_budget(text: &str) -> anyhow::Result<PerfBudget> {
    let budget: PerfBudget = serde_yaml::from_str(text).context("parse performance budget yaml")?;
    validate_budget(&budget)?;
    Ok(budget)
}

pub(crate) fn validate_budget(budget: &PerfBudget) -> anyhow::Result<()> {
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
