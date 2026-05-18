//! Core benchmark metadata types for Velvet Ballastics performance tracking.
//!
//! This module provides the data structures for capturing benchmark evidence,
//! enforcing performance budgets, and gating releases based on regression detection.
//!
//! # RED PHASE NOTICE
//! This module contains STUB implementations that will cause tests to FAIL.
//! The real implementation must replace these stubs.

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
    // The truncation is safe because no benchmark can run for 584k years.
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
    // Use checked_mul to avoid overflow; on overflow return u128::MAX (100% utilization)
    // Division is safe here because we checked budget_us != 0 above.
    #[allow(clippy::arithmetic_side_effects)]
    let result = match elapsed_us.checked_mul(10000) {
        Some(v) => v / budget_us_u128,
        None => u128::MAX,
    };
    result
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

#[cfg(test)]
mod tests {
    use super::*;

    // These tests verify the STUB implementations fail as expected.
    // When the real implementation is provided, these tests should pass.

    #[test]
    fn baseline_within_budget_returns_true_when_under() {
        // STUB: baseline_within_budget always returns false
        // This test will FAIL until the real implementation is provided
        assert!(baseline_within_budget(
            Duration::from_micros(80000),
            100_000
        ));
    }

    #[test]
    fn baseline_within_budget_returns_false_when_over() {
        // STUB: baseline_within_budget always returns false
        // This test will FAIL because we expect false but get false (coincidentally correct)
        // Actually this passes because stub always returns false!
        assert!(!baseline_within_budget(
            Duration::from_micros(120000),
            100_000
        ));
    }

    #[test]
    fn budget_utilization_percent_computes_correct() {
        // STUB: budget_utilization_percent always returns 0
        // This test will FAIL until the real implementation is provided
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(75000), 100_000),
            7500
        );
    }

    #[test]
    fn budget_utilization_percent_returns_max_for_zero_budget() {
        // STUB: budget_utilization_percent returns MAX for zero budget (correct)
        // This test passes with the stub
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(75000), 0),
            u128::MAX
        );
    }

    #[test]
    fn latency_within_budget_returns_true_when_within() {
        // STUB: latency_within_budget inverts the check
        // This test will FAIL until the real implementation is provided
        assert!(latency_within_budget(Duration::from_micros(50000), 100_000));
    }

    #[test]
    fn latency_within_budget_returns_false_when_over() {
        // STUB: latency_within_budget inverts the check (elapsed > budget)
        // This test will FAIL because stub returns true when over budget
        assert!(!latency_within_budget(
            Duration::from_micros(150000),
            100_000
        ));
    }

    #[test]
    fn result_exceeds_threshold_true_when_significant_regression() {
        // STUB: result_exceeds_threshold inverts the logic
        // This test will FAIL because stub returns false when it should return true
        assert!(result_exceeds_threshold(
            Duration::from_micros(130000),
            Duration::from_micros(100000),
            20
        ));
    }

    #[test]
    fn result_exceeds_threshold_false_when_within_threshold() {
        // STUB: result_exceeds_threshold inverts the logic
        // This test will FAIL because stub returns true when it should return false
        assert!(!result_exceeds_threshold(
            Duration::from_micros(115000),
            Duration::from_micros(100000),
            20
        ));
    }

    #[test]
    fn check_evidence_gate_rejects_missing_baseline() {
        // STUB: check_evidence_gate always returns Ok
        // This test will FAIL until the real implementation is provided
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
        // STUB: check_evidence_gate always returns Ok
        // This test will FAIL until the real implementation is provided
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 130_000, // 30% regression
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
        // STUB: check_evidence_gate always returns Ok
        // This test will PASS with the stub (coincidentally correct)
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

    // === capture_metadata tests ===

    #[test]
    fn capture_metadata_rejects_empty_commit_hash() {
        let result = capture_metadata(
            "yaml_parse",
            Some(Duration::from_micros(100_000)),
            Duration::from_micros(105_000),
            "cargo bench",
            "",
            "linux-x86_64",
            200_000,
        );
        assert!(matches!(result, Err(EvidenceError::MissingCommit)));
    }

    #[test]
    fn capture_metadata_rejects_non_hex_commit_hash() {
        let result = capture_metadata(
            "yaml_parse",
            Some(Duration::from_micros(100_000)),
            Duration::from_micros(105_000),
            "cargo bench",
            "xyz123!", // '!' is not hex
            "linux-x86_64",
            200_000,
        );
        assert!(matches!(result, Err(EvidenceError::MissingCommit)));
    }

    #[test]
    fn capture_metadata_accepts_valid_inputs() {
        let result = capture_metadata(
            "yaml_parse",
            Some(Duration::from_micros(100_000)),
            Duration::from_micros(105_000),
            "cargo bench",
            "abc123def",
            "linux-x86_64",
            200_000,
        );
        assert!(result.is_ok());
        let meta = result.unwrap();
        assert_eq!(meta.name, "yaml_parse");
        assert_eq!(meta.baseline_us, Some(100_000));
        assert_eq!(meta.result_us, 105_000);
        assert_eq!(meta.commit_hash, "abc123def");
    }

    #[test]
    fn capture_metadata_handles_none_baseline() {
        let result = capture_metadata(
            "new_benchmark",
            None,
            Duration::from_micros(50_000),
            "cargo bench",
            "abc123",
            "linux-x86_64",
            100_000,
        );
        assert!(result.is_ok());
        let meta = result.unwrap();
        assert_eq!(meta.baseline_us, None);
        assert_eq!(meta.result_us, 50_000);
    }

    // === result_exceeds_threshold boundary tests ===

    #[test]
    fn result_exceeds_threshold_false_when_exactly_at_threshold() {
        // baseline=100000, threshold_pct=20 → threshold=120000
        // result=120000 is NOT > baseline+delta, so false
        assert!(!result_exceeds_threshold(
            Duration::from_micros(120_000),
            Duration::from_micros(100_000),
            20
        ));
    }

    #[test]
    fn result_exceeds_threshold_true_one_micro_over_threshold() {
        // baseline=100000, threshold_pct=20 → threshold=120000
        // result=120001 just exceeds, so true
        assert!(result_exceeds_threshold(
            Duration::from_micros(120_001),
            Duration::from_micros(100_000),
            20
        ));
    }

    #[test]
    fn result_exceeds_threshold_false_zero_threshold_pct() {
        // threshold_pct=0 → any result > baseline triggers regression
        // result=100001 > baseline=100000, so result > baseline → true for regression
        // For no-regression (result NOT exceeds): result <= baseline
        assert!(!result_exceeds_threshold(
            Duration::from_micros(100_000),
            Duration::from_micros(100_000),
            0
        ));
    }

    #[test]
    fn result_exceeds_threshold_false_when_under_baseline() {
        // result < baseline → no regression
        assert!(!result_exceeds_threshold(
            Duration::from_micros(90_000),
            Duration::from_micros(100_000),
            20
        ));
    }

    // === baseline_within_budget boundary tests ===

    #[test]
    fn baseline_within_budget_false_at_exact_budget() {
        // baseline=100000 == budget → baseline <= budget, so true
        // But wait: baseline_within_budget returns baseline.as_micros() <= budget_us
        // So 100000 <= 100000 is TRUE. The function name is "within_budget"
        // Let's check: if baseline == budget exactly, it IS within budget → true
        assert!(baseline_within_budget(
            Duration::from_micros(100_000),
            100_000
        ));
    }

    #[test]
    fn baseline_within_budget_false_when_over_budget() {
        assert!(!baseline_within_budget(
            Duration::from_micros(100_001),
            100_000
        ));
    }

    // === latency_within_budget boundary tests ===

    #[test]
    fn latency_within_budget_false_for_zero_budget() {
        // budget_us == 0 → function returns false immediately
        assert!(!latency_within_budget(Duration::from_micros(0), 0));
    }

    #[test]
    fn latency_within_budget_true_at_exact_budget() {
        // elapsed=100000, budget=100000 → elapsed <= budget → true
        assert!(latency_within_budget(
            Duration::from_micros(100_000),
            100_000
        ));
    }

    #[test]
    fn latency_within_budget_false_one_over_budget() {
        assert!(!latency_within_budget(
            Duration::from_micros(100_001),
            100_000
        ));
    }

    // === budget_utilization_percent boundary tests ===

    #[test]
    fn budget_utilization_percent_returns_max_at_zero_budget() {
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(1), 0),
            u128::MAX
        );
    }

    #[test]
    fn budget_utilization_percent_exact_100_percent() {
        // elapsed=100000, budget=100000 → 100000*10000/100000 = 10000 (100.00%)
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(100_000), 100_000),
            10_000
        );
    }

    #[test]
    fn budget_utilization_percent_50_percent() {
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(50_000), 100_000),
            5_000
        );
    }

    // === check_evidence_gate remaining error variants ===

    #[test]
    fn check_evidence_gate_rejects_missing_environment() {
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 105_000,
            command: "cargo bench".to_string(),
            commit_hash: "abc123".to_string(),
            environment: "".to_string(), // empty environment
            budget_us: 200_000,
        };
        let result = check_evidence_gate(&metadata, 20);
        assert!(matches!(result, Err(EvidenceError::MissingEnvironment)));
    }

    #[test]
    fn check_evidence_gate_rejects_missing_command() {
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 105_000,
            command: "".to_string(), // empty command
            commit_hash: "abc123".to_string(),
            environment: "linux-x86_64".to_string(),
            budget_us: 200_000,
        };
        let result = check_evidence_gate(&metadata, 20);
        assert!(matches!(result, Err(EvidenceError::MissingCommand)));
    }

    #[test]
    fn check_evidence_gate_rejects_missing_commit() {
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 105_000,
            command: "cargo bench".to_string(),
            commit_hash: "".to_string(), // empty commit
            environment: "linux-x86_64".to_string(),
            budget_us: 200_000,
        };
        let result = check_evidence_gate(&metadata, 20);
        assert!(matches!(result, Err(EvidenceError::MissingCommit)));
    }

    #[test]
    fn check_evidence_gate_rejects_empty_budget() {
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 105_000,
            command: "cargo bench".to_string(),
            commit_hash: "abc123".to_string(),
            environment: "linux-x86_64".to_string(),
            budget_us: 0, // zero budget
        };
        let result = check_evidence_gate(&metadata, 20);
        assert!(matches!(result, Err(EvidenceError::EmptyBudget)));
    }

    // === Display impl tests for EvidenceError ===

    #[test]
    fn evidence_error_display_missing_baseline() {
        let err = EvidenceError::MissingBaseline;
        let display = format!("{}", err);
        assert_eq!(display, "missing baseline measurement");
    }

    #[test]
    fn evidence_error_display_missing_result() {
        let err = EvidenceError::MissingResult;
        let display = format!("{}", err);
        assert_eq!(display, "missing result measurement");
    }

    #[test]
    fn evidence_error_display_regression_detected() {
        let err = EvidenceError::RegressionDetected {
            benchmark: "test_bench".to_string(),
            delta: 5000,
        };
        let display = format!("{}", err);
        assert_eq!(display, "regression detected: test_bench delta=5000");
    }

    #[test]
    fn evidence_error_display_empty_budget() {
        let err = EvidenceError::EmptyBudget;
        let display = format!("{}", err);
        assert_eq!(display, "budget not configured");
    }

    // === Display impl tests for YamlBenchmarkError ===

    #[test]
    fn yaml_benchmark_error_display_parse_failure() {
        let err = YamlBenchmarkError::ParseFailure("invalid yaml".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "YAML parse failed: invalid yaml");
    }

    #[test]
    fn yaml_benchmark_error_display_validation_failure() {
        let err = YamlBenchmarkError::ValidationFailure("missing field".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "workflow validation failed: missing field");
    }

    // === Display impl tests for StorageBenchmarkError ===

    #[test]
    fn storage_benchmark_error_display_journal_open_failure() {
        let err = StorageBenchmarkError::JournalOpenFailure("file not found".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "journal open failed: file not found");
    }

    #[test]
    fn storage_benchmark_error_display_append_failure() {
        let err = StorageBenchmarkError::AppendFailure("disk full".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "journal append failed: disk full");
    }

    // === Display impl tests for IpcBenchmarkError ===

    #[test]
    fn ipc_benchmark_error_display_encode_failure() {
        let err = IpcBenchmarkError::EncodeFailure("buffer overflow".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "frame encode failed: buffer overflow");
    }

    #[test]
    fn ipc_benchmark_error_display_decode_failure() {
        let err = IpcBenchmarkError::DecodeFailure("truncated frame".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "frame decode failed: truncated frame");
    }

    // === Display impl tests for RecoveryBenchmarkError ===

    #[test]
    fn recovery_benchmark_error_display_hydration_failure() {
        let err = RecoveryBenchmarkError::HydrationFailure("missing checkpoint".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "recovery hydration failed: missing checkpoint");
    }

    // === Display impl tests for RuntimeBenchmarkError ===

    #[test]
    fn runtime_benchmark_error_display_step_failure() {
        let err = RuntimeBenchmarkError::StepFailure("stuck in loop".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "runtime step failed: stuck in loop");
    }

    #[test]
    fn runtime_benchmark_error_display_primitive_failure() {
        let err = RuntimeBenchmarkError::PrimitiveFailure("invalid opcode".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "runtime primitive failed: invalid opcode");
    }

    // === budget_utilization_percent overflow edge cases ===

    #[test]
    fn budget_utilization_percent_exact_100_bps() {
        // 100% utilization = 10000 basis points
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(100_000), 100_000),
            10_000
        );
    }

    #[test]
    fn budget_utilization_percent_150_percent() {
        // elapsed > budget: 150000/100000 = 1.5 = 15000 bps
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(150_000), 100_000),
            15_000
        );
    }

    #[test]
    fn budget_utilization_percent_200_percent() {
        // elapsed = 2x budget = 20000 bps
        assert_eq!(
            budget_utilization_percent(Duration::from_micros(200_000), 100_000),
            20_000
        );
    }

    // === capture_metadata edge cases ===

    #[test]
    fn capture_metadata_rejects_single_non_hex_char() {
        // Only 'g' is invalid hex
        let result = capture_metadata(
            "bench",
            Some(Duration::from_micros(100_000)),
            Duration::from_micros(105_000),
            "cargo bench",
            "123g",
            "linux-x86_64",
            200_000,
        );
        assert!(matches!(result, Err(EvidenceError::MissingCommit)));
    }

    #[test]
    fn capture_metadata_accepts_max_uint64_commit() {
        // u64::MAX as hex string
        let result = capture_metadata(
            "bench",
            Some(Duration::from_micros(100_000)),
            Duration::from_micros(105_000),
            "cargo bench",
            "ffffffffffffffff",
            "linux-x86_64",
            200_000,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().commit_hash, "ffffffffffffffff");
    }

    // === check_evidence_gate threshold boundary tests ===

    #[test]
    fn check_evidence_gate_accepts_exactly_at_threshold() {
        // baseline=100000, result=120000, threshold=20%
        // 120000 is NOT > 100000 + 20000 = 120000, so should pass
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 120_000,
            command: "cargo bench".to_string(),
            commit_hash: "abc123".to_string(),
            environment: "linux-x86_64".to_string(),
            budget_us: 200_000,
        };
        assert!(check_evidence_gate(&metadata, 20).is_ok());
    }

    #[test]
    fn check_evidence_gate_accepts_zero_threshold_within_baseline() {
        // threshold=0%, result=baseline
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 100_000,
            command: "cargo bench".to_string(),
            commit_hash: "abc123".to_string(),
            environment: "linux-x86_64".to_string(),
            budget_us: 200_000,
        };
        assert!(check_evidence_gate(&metadata, 0).is_ok());
    }

    #[test]
    fn check_evidence_gate_rejects_one_micro_over_threshold() {
        // baseline=100000, result=120001, threshold=20%
        // 120001 > 100000 + 20000 = 120000, so regression
        let metadata = BenchmarkMetadata {
            name: "yaml_parse".to_string(),
            baseline_us: Some(100_000),
            result_us: 120_001,
            command: "cargo bench".to_string(),
            commit_hash: "abc123".to_string(),
            environment: "linux-x86_64".to_string(),
            budget_us: 200_000,
        };
        let result = check_evidence_gate(&metadata, 20);
        // delta = result - baseline = 120001 - 100000 = 20001
        assert!(matches!(
            result,
            Err(EvidenceError::RegressionDetected { delta: 20_001, .. })
        ));
    }
}
