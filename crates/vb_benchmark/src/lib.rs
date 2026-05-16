//! Core benchmark metadata types for Velvet Ballistics performance tracking.
//!
//! This module provides the data structures for capturing benchmark evidence,
//! enforcing performance budgets, and gating releases based on regression detection.
//!
//! # RED PHASE NOTICE
//! This module contains STUB implementations that will cause tests to FAIL.
//! The real implementation must replace these stubs.

mod error;

use std::time::Duration;

pub use error::{
    EvidenceError, IpcBenchmarkError, RecoveryBenchmarkError, RuntimeBenchmarkError,
    StorageBenchmarkError, YamlBenchmarkError,
};

pub struct BenchmarkMetadata {
    pub name: String,
    pub baseline_us: Option<u64>,
    pub result_us: u64,
    pub command: String,
    pub commit_hash: String,
    pub environment: String,
    pub budget_us: u64,
}

pub fn capture_metadata(
    name: &str,
    baseline: Option<Duration>,
    result: Duration,
    command: &str,
    commit_hash: &str,
    environment: &str,
    budget_us: u64,
) -> Result<BenchmarkMetadata, EvidenceError> {
    if commit_hash.is_empty() {
        return Err(EvidenceError::MissingCommit);
    }
    if !commit_hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(EvidenceError::MissingCommit);
    }

    #[allow(clippy::as_conversions)]
    let baseline_us = baseline.map(|d| d.as_micros() as u64);
    #[allow(clippy::as_conversions)]
    let result_us = result.as_micros() as u64;

    Ok(BenchmarkMetadata {
        name: name.to_string(),
        baseline_us,
        result_us,
        command: command.to_string(),
        commit_hash: commit_hash.to_string(),
        environment: environment.to_string(),
        budget_us,
    })
}

pub fn baseline_within_budget(baseline: Duration, budget_us: u64) -> bool {
    let budget_us_u128 = u128::from(budget_us);
    baseline.as_micros() <= budget_us_u128
}

pub fn result_exceeds_threshold(result: Duration, baseline: Duration, threshold_pct: u64) -> bool {
    let baseline_us = baseline.as_micros();
    let result_us = result.as_micros();
    let threshold_delta = baseline_us.saturating_mul(u128::from(threshold_pct)) / 100;
    result_us > baseline_us.saturating_add(threshold_delta)
}

pub fn latency_within_budget(elapsed: Duration, budget_us: u64) -> bool {
    if budget_us == 0 {
        return false;
    }
    let budget_us_u128 = u128::from(budget_us);
    elapsed.as_micros() <= budget_us_u128
}

pub fn budget_utilization_percent(elapsed: Duration, budget_us: u64) -> u128 {
    if budget_us == 0 {
        return u128::MAX;
    }
    let elapsed_us = elapsed.as_micros();
    let budget_us_u128 = u128::from(budget_us);
    #[allow(clippy::arithmetic_side_effects)]
    let result = match elapsed_us.checked_mul(10000) {
        Some(v) => v / budget_us_u128,
        None => u128::MAX,
    };
    result
}

pub fn check_evidence_gate(
    metadata: &BenchmarkMetadata,
    threshold_pct: u64,
) -> Result<(), EvidenceError> {
    let baseline_us = match metadata.baseline_us {
        Some(b) => b,
        None => return Err(EvidenceError::MissingBaseline),
    };

    if metadata.environment.is_empty() {
        return Err(EvidenceError::MissingEnvironment);
    }

    if metadata.command.is_empty() {
        return Err(EvidenceError::MissingCommand);
    }

    if metadata.commit_hash.is_empty() {
        return Err(EvidenceError::MissingCommit);
    }

    if metadata.budget_us == 0 {
        return Err(EvidenceError::EmptyBudget);
    }

    let result_us = metadata.result_us;
    let threshold_delta = baseline_us.saturating_mul(threshold_pct) / 100;
    if result_us > baseline_us.saturating_add(threshold_delta) {
        let delta = result_us.saturating_sub(baseline_us);
        return Err(EvidenceError::RegressionDetected {
            benchmark: metadata.name.clone(),
            delta,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_within_budget_returns_true_when_under() {
        assert!(baseline_within_budget(
            Duration::from_micros(80000),
            100_000
        ));
    }

    #[test]
    fn baseline_within_budget_returns_false_when_over() {
        assert!(!baseline_within_budget(
            Duration::from_micros(120000),
            100_000
        ));
    }

    #[test]
    fn budget_utilization_percent_computes_correct() {
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(75000), 100_000),
            7500
        );
    }

    #[test]
    fn budget_utilization_percent_returns_max_for_zero_budget() {
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(75000), 0),
            u128::MAX
        );
    }

    #[test]
    fn latency_within_budget_returns_true_when_within() {
        assert!(latency_within_budget(Duration::from_micros(50000), 100_000));
    }

    #[test]
    fn latency_within_budget_returns_false_when_over() {
        assert!(!latency_within_budget(
            Duration::from_micros(150000),
            100_000
        ));
    }

    #[test]
    fn result_exceeds_threshold_true_when_significant_regression() {
        assert!(result_exceeds_threshold(
            Duration::from_micros(130000),
            Duration::from_micros(100000),
            20
        ));
    }

    #[test]
    fn result_exceeds_threshold_false_when_within_threshold() {
        assert!(!result_exceeds_threshold(
            Duration::from_micros(115000),
            Duration::from_micros(100000),
            20
        ));
    }

    #[test]
    fn check_evidence_gate_rejects_missing_baseline() {
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: None,
            result_us: 105_000,
            command: "cargo bench".to_string(),
            commit_hash: "abc123".to_string(),
            environment: "linux-x86_64".to_string(),
            budget_us: 200_000,
        };
        let result = check_evidence_gate(&metadata, 20);
        assert!(matches!(result, Err(EvidenceError::MissingBaseline)));
    }

    #[test]
    fn check_evidence_gate_rejects_regression() {
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 130_000,
            command: "cargo bench".to_string(),
            commit_hash: "abc123".to_string(),
            environment: "linux-x86_64".to_string(),
            budget_us: 200_000,
        };
        let result = check_evidence_gate(&metadata, 20);
        assert!(matches!(
            result,
            Err(EvidenceError::RegressionDetected { .. })
        ));
    }

    #[test]
    fn check_evidence_gate_accepts_valid() {
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 105_000,
            command: "cargo bench".to_string(),
            commit_hash: "abc123".to_string(),
            environment: "linux-x86_64".to_string(),
            budget_us: 200_000,
        };
        assert!(check_evidence_gate(&metadata, 20).is_ok());
    }
}
