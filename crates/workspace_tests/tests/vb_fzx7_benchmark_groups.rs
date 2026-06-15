//! Integration tests for benchmark contracts
//!
//! Earlier polluted branch work left placeholder tests that only asserted `true`
//! for benchmark group registration functions that no longer exist. These tests
//! now exercise the concrete public benchmark metadata, evidence-gate, budget,
//! and error contracts that the release gate depends on.

use std::time::Duration;

use vb_benchmark::{
    BenchmarkMetadata, EvidenceError, IpcBenchmarkError, RecoveryBenchmarkError,
    RuntimeBenchmarkError, StorageBenchmarkError, YamlBenchmarkError, baseline_within_budget,
    budget_utilization_percent, check_evidence_gate, latency_within_budget,
    result_exceeds_threshold,
};

fn complete_metadata() -> BenchmarkMetadata {
    BenchmarkMetadata {
        name: String::from("ipc_frame_decode"),
        baseline_us: Some(69),
        result_us: 70,
        command: String::from("cargo bench --bench ipc_frame_decode"),
        commit_hash: String::from("c15d4f375af205fd2954a419a53c6450ea266699"),
        environment: String::from("linux-x86_64"),
        budget_us: 72,
        direct_api_latency_ns: 0,
        fjall_write_latency_ns: 0,
        ipc_latency_ns: 0,
    }
}

/// Benchmark metadata preserves every field required by evidence gates.
#[test]
fn benchmark_metadata_preserves_required_evidence_fields() {
    let metadata = complete_metadata();

    assert_eq!(metadata.name, "ipc_frame_decode");
    assert_eq!(metadata.baseline_us, Some(69));
    assert_eq!(metadata.result_us, 70);
    assert_eq!(metadata.command, "cargo bench --bench ipc_frame_decode");
    assert_eq!(
        metadata.commit_hash,
        "c15d4f375af205fd2954a419a53c6450ea266699"
    );
    assert_eq!(metadata.environment, "linux-x86_64");
    assert_eq!(metadata.budget_us, 72);
}

/// Budget helpers enforce boundary behavior used by benchmark policy checks.
#[test]
fn benchmark_budget_helpers_enforce_threshold_boundaries() {
    assert!(baseline_within_budget(Duration::from_micros(70), 70));
    assert!(!latency_within_budget(Duration::from_micros(73), 72));
    assert_eq!(
        budget_utilization_percent(Duration::from_micros(36), 72),
        5_000
    );
    assert!(!result_exceeds_threshold(
        Duration::from_micros(72),
        Duration::from_micros(69),
        5
    ));
    assert!(result_exceeds_threshold(
        Duration::from_micros(73),
        Duration::from_micros(69),
        5
    ));
}

/// Evidence gate accepts complete metadata and rejects missing required fields.
#[test]
fn benchmark_evidence_gate_distinguishes_complete_and_missing_baseline() {
    let complete = complete_metadata();
    assert!(check_evidence_gate(&complete, 5).is_ok());

    let missing_baseline = BenchmarkMetadata {
        baseline_us: None,
        ..complete
    };
    assert!(matches!(
        check_evidence_gate(&missing_baseline, 5),
        Err(EvidenceError::MissingBaseline)
    ));
}

/// Evidence gate rejects missing environment metadata.
#[test]
fn benchmark_evidence_gate_rejects_missing_environment() {
    let metadata = BenchmarkMetadata {
        environment: String::new(),
        ..complete_metadata()
    };

    assert!(matches!(
        check_evidence_gate(&metadata, 5),
        Err(EvidenceError::MissingEnvironment)
    ));
}

/// Evidence gate rejects missing benchmark command metadata.
#[test]
fn benchmark_evidence_gate_rejects_missing_command() {
    let metadata = BenchmarkMetadata {
        command: String::new(),
        ..complete_metadata()
    };

    assert!(matches!(
        check_evidence_gate(&metadata, 5),
        Err(EvidenceError::MissingCommand)
    ));
}

/// Evidence gate rejects missing commit metadata.
#[test]
fn benchmark_evidence_gate_rejects_missing_commit() {
    let metadata = BenchmarkMetadata {
        commit_hash: String::new(),
        ..complete_metadata()
    };

    assert!(matches!(
        check_evidence_gate(&metadata, 5),
        Err(EvidenceError::MissingCommit)
    ));
}

/// Evidence gate rejects an empty benchmark budget.
#[test]
fn benchmark_evidence_gate_rejects_empty_budget() {
    let metadata = BenchmarkMetadata {
        budget_us: 0,
        ..complete_metadata()
    };

    assert!(matches!(
        check_evidence_gate(&metadata, 5),
        Err(EvidenceError::EmptyBudget)
    ));
}

/// Evidence gate reports the exact regression delta beyond baseline.
#[test]
fn benchmark_evidence_gate_reports_regression_delta() {
    let metadata = BenchmarkMetadata {
        result_us: 74,
        ..complete_metadata()
    };

    assert!(matches!(
        check_evidence_gate(&metadata, 5),
        Err(EvidenceError::RegressionDetected {
            benchmark,
            delta: 5
        }) if benchmark == "ipc_frame_decode"
    ));
}

/// Budget utilization uses an explicit fail-closed value for zero budgets.
#[test]
fn benchmark_budget_utilization_rejects_zero_budget() {
    assert_eq!(
        budget_utilization_percent(Duration::from_micros(1), 0),
        u128::MAX
    );
}

/// Runtime benchmark errors have stable, distinct diagnostics.
#[test]
fn runtime_benchmark_errors_report_distinct_contract_failures() {
    let step = RuntimeBenchmarkError::StepFailure(String::from("budget exhausted"));
    let primitive = RuntimeBenchmarkError::PrimitiveFailure(String::from("bad opcode"));

    assert_eq!(format!("{step}"), "runtime step failed: budget exhausted");
    assert_eq!(
        format!("{primitive}"),
        "runtime primitive failed: bad opcode"
    );
}

/// YAML benchmark errors have stable, distinct diagnostics.
#[test]
fn yaml_benchmark_errors_report_distinct_contract_failures() {
    let parse = YamlBenchmarkError::ParseFailure(String::from("bad colon"));
    let validation = YamlBenchmarkError::ValidationFailure(String::from("missing step"));

    assert_eq!(format!("{parse}"), "YAML parse failed: bad colon");
    assert_eq!(
        format!("{validation}"),
        "workflow validation failed: missing step"
    );
}

// ============================================================================
// Benchmark Group Error Handling Tests
// ============================================================================

/// Test that YamlBenchmarkError can wrap inner errors
#[test]
fn yaml_benchmark_error_wraps_parse_failure() {
    let err = YamlBenchmarkError::ParseFailure("invalid yaml syntax".to_string());
    let display = format!("{}", err);
    assert!(display.contains("YAML parse failed"));
    assert!(display.contains("invalid yaml syntax"));
}

/// Test that YamlBenchmarkError can wrap validation failures
#[test]
fn yaml_benchmark_error_wraps_validation_failure() {
    let err = YamlBenchmarkError::ValidationFailure("missing name field".to_string());
    let display = format!("{}", err);
    assert!(display.contains("workflow validation failed"));
    assert!(display.contains("missing name field"));
}

/// Test that StorageBenchmarkError can wrap journal open failures
#[test]
fn storage_benchmark_error_wraps_journal_open_failure() {
    let err = StorageBenchmarkError::JournalOpenFailure("path not found".to_string());
    let display = format!("{}", err);
    assert!(display.contains("journal open failed"));
    assert!(display.contains("path not found"));
}

/// Test that StorageBenchmarkError can wrap append failures
#[test]
fn storage_benchmark_error_wraps_append_failure() {
    let err = StorageBenchmarkError::AppendFailure("disk full".to_string());
    let display = format!("{}", err);
    assert!(display.contains("journal append failed"));
    assert!(display.contains("disk full"));
}

/// Test that IpcBenchmarkError can wrap encode failures
#[test]
fn ipc_benchmark_error_wraps_encode_failure() {
    let err = IpcBenchmarkError::EncodeFailure("frame too large".to_string());
    let display = format!("{}", err);
    assert!(display.contains("frame encode failed"));
    assert!(display.contains("frame too large"));
}

/// Test that IpcBenchmarkError can wrap decode failures
#[test]
fn ipc_benchmark_error_wraps_decode_failure() {
    let err = IpcBenchmarkError::DecodeFailure("truncated frame".to_string());
    let display = format!("{}", err);
    assert!(display.contains("frame decode failed"));
    assert!(display.contains("truncated frame"));
}

/// Test that RecoveryBenchmarkError can wrap hydration failures
#[test]
fn recovery_benchmark_error_wraps_hydration_failure() {
    let err = RecoveryBenchmarkError::HydrationFailure("invalid event type".to_string());
    let display = format!("{}", err);
    assert!(display.contains("recovery hydration failed"));
    assert!(display.contains("invalid event type"));
}
