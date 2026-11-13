fn validate_claim(
    root: &Path,
    budget: &PerfBudget,
    claim: &BenchmarkEvidenceClaim,
    line_number: usize,
    current_commit: &str,
) -> crate::benchmark_policy::error::Result<()> {
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
    validate_metric_claim(budget, claim, line_number)
}

fn validate_metric_claim(
    budget: &PerfBudget,
    claim: &BenchmarkEvidenceClaim,
    line_number: usize,
) -> crate::benchmark_policy::error::Result<()> {
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
    validate_regression(claim, metric_budget.max_regression_percent, line_number)
}

fn validate_regression(
    claim: &BenchmarkEvidenceClaim,
    threshold_percent: u64,
    line_number: usize,
) -> crate::benchmark_policy::error::Result<()> {
    if result_exceeds_threshold(claim.result_value, claim.baseline_value, threshold_percent) {
        bail!(
            "benchmark evidence line {line_number} regressed: metric={} baseline={} result={} threshold_percent={}",
            claim.metric,
            claim.baseline_value,
            claim.result_value,
            threshold_percent
        );
    }
    Ok(())
}

fn validate_required_evidence_fields(
    budget: &PerfBudget,
    claim: &Value,
    line_number: usize,
) -> crate::benchmark_policy::error::Result<()> {
    let Some(object) = claim.as_object() else {
        bail!("benchmark evidence line {line_number} must be a JSON object");
    };
    let missing = budget
        .required_evidence_fields
        .iter()
        .filter(|field| missing_evidence_field(object, field))
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
) -> crate::benchmark_policy::error::Result<()> {
    let required = required_claim_strings(claim);
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
) -> crate::benchmark_policy::error::Result<()> {
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

fn validate_commit(commit: &str, line_number: usize) -> crate::benchmark_policy::error::Result<()> {
    benchmark_freshness::validate_commit_format(
        commit,
        &format!("benchmark evidence line {line_number} commit"),
    )
}

fn validate_raw_log(
    root: &Path,
    raw_log: &str,
    line_number: usize,
) -> crate::benchmark_policy::error::Result<()> {
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
) -> crate::benchmark_policy::error::Result<()> {
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

fn missing_evidence_field(object: &serde_json::Map<String, Value>, field: &str) -> bool {
    object
        .get(field)
        .is_none_or(|value| !evidence_value_present(value))
}

fn required_claim_strings(claim: &BenchmarkEvidenceClaim) -> [(&'static str, &str); 7] {
    [
        ("metric", claim.metric.as_str()),
        ("claim", claim.claim.as_str()),
        ("unit", claim.unit.as_str()),
        ("raw_log", claim.raw_log.as_str()),
        ("command", claim.command.as_str()),
        ("commit", claim.commit.as_str()),
        ("mode", claim.mode.as_str()),
    ]
}
