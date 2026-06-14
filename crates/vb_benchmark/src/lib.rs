#![forbid(unsafe_code)]

//! Core benchmark metadata types for Velvet Ballistics performance tracking.
//!
//! This module provides the data structures for capturing benchmark evidence,
//! enforcing performance budgets, and gating releases based on regression
//! detection.

pub mod aggregate_resource_budget;

use std::time::Duration;

/// Benchmark metadata captured during a single benchmark run.
///
/// Contains baseline, result, and environment information required for
/// evidence-based performance regression gating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkMetadata {
    /// Name of the benchmark group.
    pub name: String,
    /// Baseline execution time in microseconds (None for new benchmarks).
    pub baseline_us: Option<u64>,
    /// Result execution time in microseconds.
    pub result_us: u64,
    /// Command that produced this result.
    pub command: String,
    /// Git commit hash (non-empty ASCII hex string).
    pub commit_hash: String,
    /// Environment identifier (e.g., "linux-x86_64").
    pub environment: String,
    /// Performance budget in microseconds.
    pub budget_us: u64,
}

/// Error types for evidence gate validation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvidenceError {
    /// Missing baseline measurement.
    MissingBaseline,
    /// Missing result measurement.
    MissingResult,
    /// Missing environment information.
    MissingEnvironment,
    /// Missing command information.
    MissingCommand,
    /// Missing commit hash.
    MissingCommit,
    /// Performance regression detected.
    RegressionDetected {
        /// Benchmark name.
        benchmark: String,
        /// Delta between result and baseline in microseconds.
        delta: u64,
    },
    /// Budget not configured (zero budget_us).
    EmptyBudget,
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceError::MissingBaseline => write!(f, "missing baseline measurement"),
            EvidenceError::MissingResult => write!(f, "missing result measurement"),
            EvidenceError::MissingEnvironment => write!(f, "missing environment"),
            EvidenceError::MissingCommand => write!(f, "missing command"),
            EvidenceError::MissingCommit => write!(f, "missing commit hash"),
            EvidenceError::RegressionDetected { benchmark, delta } => {
                write!(f, "regression detected: {benchmark} delta={delta}")
            }
            EvidenceError::EmptyBudget => write!(f, "budget not configured"),
        }
    }
}

impl std::error::Error for EvidenceError {}

/// Error types for YAML benchmark operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum YamlBenchmarkError {
    /// YAML parse failed.
    ParseFailure(String),
    /// Workflow validation failed.
    ValidationFailure(String),
}

impl std::fmt::Display for YamlBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlBenchmarkError::ParseFailure(inner) => write!(f, "YAML parse failed: {inner}"),
            YamlBenchmarkError::ValidationFailure(inner) => {
                write!(f, "workflow validation failed: {inner}")
            }
        }
    }
}

impl std::error::Error for YamlBenchmarkError {}

/// Error types for storage benchmark operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StorageBenchmarkError {
    /// Journal open failed.
    JournalOpenFailure(String),
    /// Append operation failed.
    AppendFailure(String),
}

impl std::fmt::Display for StorageBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageBenchmarkError::JournalOpenFailure(inner) => {
                write!(f, "journal open failed: {inner}")
            }
            StorageBenchmarkError::AppendFailure(inner) => {
                write!(f, "journal append failed: {inner}")
            }
        }
    }
}

impl std::error::Error for StorageBenchmarkError {}

/// Error types for IPC benchmark operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IpcBenchmarkError {
    /// Frame encode failed.
    EncodeFailure(String),
    /// Frame decode failed.
    DecodeFailure(String),
}

impl std::fmt::Display for IpcBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcBenchmarkError::EncodeFailure(inner) => write!(f, "frame encode failed: {inner}"),
            IpcBenchmarkError::DecodeFailure(inner) => write!(f, "frame decode failed: {inner}"),
        }
    }
}

impl std::error::Error for IpcBenchmarkError {}

/// Error types for recovery benchmark operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryBenchmarkError {
    /// Hydration failed.
    HydrationFailure(String),
}

impl std::fmt::Display for RecoveryBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryBenchmarkError::HydrationFailure(inner) => {
                write!(f, "recovery hydration failed: {inner}")
            }
        }
    }
}

impl std::error::Error for RecoveryBenchmarkError {}

/// Error types for runtime benchmark operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeBenchmarkError {
    /// Runtime step failed.
    StepFailure(String),
    /// Runtime primitive evaluation failed.
    PrimitiveFailure(String),
}

impl std::fmt::Display for RuntimeBenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeBenchmarkError::StepFailure(inner) => write!(f, "runtime step failed: {inner}"),
            RuntimeBenchmarkError::PrimitiveFailure(inner) => {
                write!(f, "runtime primitive failed: {inner}")
            }
        }
    }
}

impl std::error::Error for RuntimeBenchmarkError {}

/// Captures benchmark metadata from a run.
///
/// # Errors
///
/// Returns `Err(EvidenceError::MissingCommit)` if `commit_hash` is empty or not ASCII hex.
pub fn capture_metadata(
    name: &str,
    baseline: Option<Duration>,
    result: Duration,
    command: &str,
    commit_hash: &str,
    environment: &str,
    budget_us: u64,
) -> Result<BenchmarkMetadata, EvidenceError> {
    // Validate commit_hash is non-empty ASCII hex
    if commit_hash.is_empty() {
        return Err(EvidenceError::MissingCommit);
    }
    if !commit_hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(EvidenceError::MissingCommit);
    }

    // Duration::as_micros() returns u128 on nightly; convert to u64 for struct fields.
    // Benchmark durations are always well under u64::MAX microseconds (~584k years).
    let baseline_us = baseline.map(|d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX));
    let result_us = u64::try_from(result.as_micros()).unwrap_or(u64::MAX);

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

/// Checks if the baseline execution time is within the configured budget.
///
/// Returns `true` if `baseline.as_micros() <= budget_us`.
pub fn baseline_within_budget(baseline: Duration, budget_us: u64) -> bool {
    // Convert budget_us to u128 for comparison with as_micros() result
    let budget_us_u128 = u128::from(budget_us);
    baseline.as_micros() <= budget_us_u128
}

/// Checks if the result execution time exceeds the baseline by more than the threshold percentage.
///
/// Returns `true` if `result > baseline + threshold_pct * baseline / 100`.
pub fn result_exceeds_threshold(result: Duration, baseline: Duration, threshold_pct: u64) -> bool {
    let baseline_us = baseline.as_micros();
    let result_us = result.as_micros();
    let threshold_delta = baseline_us.saturating_mul(u128::from(threshold_pct)) / 100;
    result_us > baseline_us.saturating_add(threshold_delta)
}

/// Checks if the elapsed time is within the configured budget.
///
/// Returns `true` if `elapsed.as_micros() <= budget_us` and `budget_us > 0`.
pub fn latency_within_budget(elapsed: Duration, budget_us: u64) -> bool {
    if budget_us == 0 {
        return false;
    }
    let budget_us_u128 = u128::from(budget_us);
    elapsed.as_micros() <= budget_us_u128
}

/// Computes budget utilization as a percentage in basis points.
///
/// Returns `u128::MAX` if `budget_us == 0`.
/// Otherwise returns `(elapsed.as_micros() * 10000) / budget_us` clamped to `u128`.
pub fn budget_utilization_percent(elapsed: Duration, budget_us: u64) -> u128 {
    if budget_us == 0 {
        return u128::MAX;
    }
    // Duration::as_micros() returns u128 on nightly; budget_us converts to u128 for division
    let elapsed_us = elapsed.as_micros();
    let budget_us_u128 = u128::from(budget_us);
    match elapsed_us.checked_mul(10000) {
        Some(v) => v.checked_div(budget_us_u128).unwrap_or(u128::MAX),
        None => u128::MAX,
    }
}

/// Checks the evidence gate for a benchmark result.
///
/// Returns `Ok(())` if all required metadata is present and result is within
/// the configured threshold. Returns an error otherwise.
pub fn check_evidence_gate(
    metadata: &BenchmarkMetadata,
    threshold_pct: u64,
) -> Result<(), EvidenceError> {
    // Check baseline is present
    let baseline_us = match metadata.baseline_us {
        Some(b) => b,
        None => return Err(EvidenceError::MissingBaseline),
    };

    // Check environment is not empty
    if metadata.environment.is_empty() {
        return Err(EvidenceError::MissingEnvironment);
    }

    // Check command is not empty
    if metadata.command.is_empty() {
        return Err(EvidenceError::MissingCommand);
    }

    // Check commit_hash is not empty
    if metadata.commit_hash.is_empty() {
        return Err(EvidenceError::MissingCommit);
    }

    // Check budget is not zero
    if metadata.budget_us == 0 {
        return Err(EvidenceError::EmptyBudget);
    }

    // Check for regression
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
