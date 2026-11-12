//! Error Variant Tests
//!
//! Tests for all error types: EvidenceError, YamlBenchmarkError, StorageBenchmarkError,
//! IpcBenchmarkError, RecoveryBenchmarkError, RuntimeBenchmarkError.
//!
//! # RED PHASE
//! These tests COMPILE but FAIL because the implementation contains intentional bugs.

use xtask::benchmark_metadata::{
    EvidenceError, IpcBenchmarkError, RecoveryBenchmarkError, RuntimeBenchmarkError,
    StorageBenchmarkError, YamlBenchmarkError,
};

// ============================================================================
// EvidenceError Display Implementations (EV-001 through EV-007)
// ============================================================================

#[test]
fn evidence_error_missing_baseline_display() {
    let err = EvidenceError::MissingBaseline;
    let display = format!("{}", err);
    assert_eq!(display, "missing baseline measurement");
}

#[test]
fn evidence_error_missing_result_display() {
    let err = EvidenceError::MissingResult;
    let display = format!("{}", err);
    assert_eq!(display, "missing result measurement");
}

#[test]
fn evidence_error_missing_environment_display() {
    let err = EvidenceError::MissingEnvironment;
    let display = format!("{}", err);
    assert_eq!(display, "missing environment");
}

#[test]
fn evidence_error_missing_command_display() {
    let err = EvidenceError::MissingCommand;
    let display = format!("{}", err);
    assert_eq!(display, "missing command");
}

#[test]
fn evidence_error_missing_commit_display() {
    let err = EvidenceError::MissingCommit;
    let display = format!("{}", err);
    assert_eq!(display, "missing commit hash");
}

#[test]
fn evidence_error_regression_detected_display() {
    let err = EvidenceError::RegressionDetected {
        benchmark: "yaml_parse".to_string(),
        delta: 30000,
    };
    let display = format!("{}", err);
    assert_eq!(display, "regression detected: yaml_parse delta=30000");
}

#[test]
fn evidence_error_empty_budget_display() {
    let err = EvidenceError::EmptyBudget;
    let display = format!("{}", err);
    assert_eq!(display, "budget not configured");
}

// ============================================================================
// YamlBenchmarkError Tests (BE-001, BE-002)
// ============================================================================

#[test]
fn yaml_benchmark_error_parse_failure_display() {
    let err = YamlBenchmarkError::ParseFailure("unexpected token".to_string());
    let display = format!("{}", err);
    assert_eq!(display, "YAML parse failed: unexpected token");
}

#[test]
fn yaml_benchmark_error_validation_failure_display() {
    let err = YamlBenchmarkError::ValidationFailure("missing required field".to_string());
    let display = format!("{}", err);
    assert_eq!(
        display,
        "workflow validation failed: missing required field"
    );
}

#[test]
fn yaml_benchmark_error_parse_failure_debug() {
    let err = YamlBenchmarkError::ParseFailure("syntax error".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("ParseFailure"));
    assert!(debug.contains("syntax error"));
}

#[test]
fn yaml_benchmark_error_validation_failure_debug() {
    let err = YamlBenchmarkError::ValidationFailure("invalid schema".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("ValidationFailure"));
    assert!(debug.contains("invalid schema"));
}

// ============================================================================
// StorageBenchmarkError Tests (BE-003, BE-004)
// ============================================================================

#[test]
fn storage_benchmark_error_journal_open_failure_display() {
    let err = StorageBenchmarkError::JournalOpenFailure("path not found".to_string());
    let display = format!("{}", err);
    assert_eq!(display, "journal open failed: path not found");
}

#[test]
fn storage_benchmark_error_append_failure_display() {
    let err = StorageBenchmarkError::AppendFailure("disk full".to_string());
    let display = format!("{}", err);
    assert_eq!(display, "journal append failed: disk full");
}

#[test]
fn storage_benchmark_error_journal_open_failure_debug() {
    let err = StorageBenchmarkError::JournalOpenFailure("permission denied".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("JournalOpenFailure"));
    assert!(debug.contains("permission denied"));
}

#[test]
fn storage_benchmark_error_append_failure_debug() {
    let err = StorageBenchmarkError::AppendFailure("io error".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("AppendFailure"));
    assert!(debug.contains("io error"));
}

// ============================================================================
// IpcBenchmarkError Tests (BE-005, BE-006)
// ============================================================================

#[test]
fn ipc_benchmark_error_encode_failure_display() {
    let err = IpcBenchmarkError::EncodeFailure("invalid frame".to_string());
    let display = format!("{}", err);
    assert_eq!(display, "frame encode failed: invalid frame");
}

#[test]
fn ipc_benchmark_error_decode_failure_display() {
    let err = IpcBenchmarkError::DecodeFailure("truncated data".to_string());
    let display = format!("{}", err);
    assert_eq!(display, "frame decode failed: truncated data");
}

#[test]
fn ipc_benchmark_error_encode_failure_debug() {
    let err = IpcBenchmarkError::EncodeFailure("buffer overflow".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("EncodeFailure"));
    assert!(debug.contains("buffer overflow"));
}

#[test]
fn ipc_benchmark_error_decode_failure_debug() {
    let err = IpcBenchmarkError::DecodeFailure("checksum mismatch".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("DecodeFailure"));
    assert!(debug.contains("checksum mismatch"));
}

// ============================================================================
// RecoveryBenchmarkError Tests (BE-007)
// ============================================================================

#[test]
fn recovery_benchmark_error_hydration_failure_display() {
    let err = RecoveryBenchmarkError::HydrationFailure("invalid event sequence".to_string());
    let display = format!("{}", err);
    assert_eq!(display, "recovery hydration failed: invalid event sequence");
}

#[test]
fn recovery_benchmark_error_hydration_failure_debug() {
    let err = RecoveryBenchmarkError::HydrationFailure("missing checkpoint".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("HydrationFailure"));
    assert!(debug.contains("missing checkpoint"));
}

// ============================================================================
// RuntimeBenchmarkError Tests
// ============================================================================

#[test]
fn runtime_benchmark_error_step_failure_display() {
    let err = RuntimeBenchmarkError::StepFailure("engine stuck".to_string());
    let display = format!("{}", err);
    assert_eq!(display, "runtime step failed: engine stuck");
}

#[test]
fn runtime_benchmark_error_primitive_failure_display() {
    let err = RuntimeBenchmarkError::PrimitiveFailure("division by zero".to_string());
    let display = format!("{}", err);
    assert_eq!(display, "runtime primitive failed: division by zero");
}

#[test]
fn runtime_benchmark_error_step_failure_debug() {
    let err = RuntimeBenchmarkError::StepFailure("timeout".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("StepFailure"));
    assert!(debug.contains("timeout"));
}

#[test]
fn runtime_benchmark_error_primitive_failure_debug() {
    let err = RuntimeBenchmarkError::PrimitiveFailure("type mismatch".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("PrimitiveFailure"));
    assert!(debug.contains("type mismatch"));
}

// ============================================================================
// Error Equality Tests
// ============================================================================

#[test]
fn evidence_error_variants_are_equatable() {
    let err1 = EvidenceError::MissingBaseline;
    let err2 = EvidenceError::MissingBaseline;
    let err3 = EvidenceError::MissingResult;

    assert_eq!(err1, err2);
    assert_ne!(err1, err3);
}

#[test]
fn evidence_error_regression_detected_equatable() {
    let err1 = EvidenceError::RegressionDetected {
        benchmark: "yaml_parse".to_string(),
        delta: 30000,
    };
    let err2 = EvidenceError::RegressionDetected {
        benchmark: "yaml_parse".to_string(),
        delta: 30000,
    };
    let err3 = EvidenceError::RegressionDetected {
        benchmark: "yaml_compile".to_string(),
        delta: 30000,
    };
    let err4 = EvidenceError::RegressionDetected {
        benchmark: "yaml_parse".to_string(),
        delta: 40000,
    };

    assert_eq!(err1, err2);
    assert_ne!(err1, err3);
    assert_ne!(err1, err4);
}

#[test]
fn yaml_benchmark_error_equatable() {
    let err1 = YamlBenchmarkError::ParseFailure("error".to_string());
    let err2 = YamlBenchmarkError::ParseFailure("error".to_string());
    let err3 = YamlBenchmarkError::ParseFailure("other".to_string());
    let err4 = YamlBenchmarkError::ValidationFailure("error".to_string());

    assert_eq!(err1, err2);
    assert_ne!(err1, err3);
    assert_ne!(err1, err4);
}
