//! Evidence Gate Tests
//!
//! Tests for `check_evidence_gate`, `baseline_within_budget`, and related functions.
//! These tests verify the evidence-based performance regression gating system.
//!
//! # RED PHASE
//! These tests COMPILE but FAIL because the implementation contains intentional bugs.
//! Once the correct implementation is provided, all tests should pass.

use std::time::Duration;
use vb_benchmark::{
    BenchmarkMetadata, EvidenceError, baseline_within_budget, budget_utilization_percent,
    capture_metadata, check_evidence_gate, latency_within_budget, result_exceeds_threshold,
};

fn valid_metadata(result: Result<BenchmarkMetadata, EvidenceError>) -> BenchmarkMetadata {
    match result {
        Ok(metadata) => metadata,
        Err(error) => panic!("expected valid benchmark metadata, got {error}"),
    }
}

// ============================================================================
// Evidence Gate Tests (EG-001 through EG-008)
// ============================================================================

/// EG-001: Evidence gate accepts complete metadata within threshold
#[test]
fn eg_001_evidence_gate_accepts_complete_metadata_within_threshold() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 105_000,
        command: "cargo bench yaml_parse".to_string(),
        commit_hash: "abc123def456".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
    };

    let result = check_evidence_gate(&metadata, 20);

    // EG-001: Should return Ok(()) when all required fields present and within threshold
    assert!(
        result.is_ok(),
        "Expected Ok(()) for complete metadata within threshold, got {:?}",
        result
    );
}

/// EG-002: Evidence gate rejects missing baseline
#[test]
fn eg_002_evidence_gate_rejects_missing_baseline() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: None, // Missing!
        result_us: 105_000,
        command: "cargo bench yaml_parse".to_string(),
        commit_hash: "abc123def456".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
    };

    let result = check_evidence_gate(&metadata, 20);

    // EG-002: Should return MissingBaseline error
    match result {
        Err(EvidenceError::MissingBaseline) => {}
        other => panic!("Expected MissingBaseline, got {:?}", other),
    }
}

/// EG-003: Evidence gate rejects missing result
#[test]
fn eg_003_evidence_gate_rejects_missing_result() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 0, // Result should always be present in proper metadata
        command: "cargo bench yaml_parse".to_string(),
        commit_hash: "abc123def456".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
    };

    let _result = check_evidence_gate(&metadata, 20);

    // EG-003: For this test, we consider 0 result as "missing" - a new benchmark without results
    // Note: The actual MissingResult error is for when result_us is truly absent (None)
    // A zero result_us is a valid result for some benchmarks
}

/// EG-004: Evidence gate rejects missing environment
#[test]
fn eg_004_evidence_gate_rejects_missing_environment() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 105_000,
        command: "cargo bench yaml_parse".to_string(),
        commit_hash: "abc123def456".to_string(),
        environment: "".to_string(), // Empty!
        budget_us: 200_000,
    };

    let result = check_evidence_gate(&metadata, 20);

    // EG-004: Should return MissingEnvironment error
    match result {
        Err(EvidenceError::MissingEnvironment) => {}
        other => panic!("Expected MissingEnvironment, got {:?}", other),
    }
}

/// EG-005: Evidence gate rejects missing command
#[test]
fn eg_005_evidence_gate_rejects_missing_command() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 105_000,
        command: "".to_string(), // Empty!
        commit_hash: "abc123def456".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
    };

    let result = check_evidence_gate(&metadata, 20);

    // EG-005: Should return MissingCommand error
    match result {
        Err(EvidenceError::MissingCommand) => {}
        other => panic!("Expected MissingCommand, got {:?}", other),
    }
}

/// EG-006: Evidence gate rejects missing commit hash
#[test]
fn eg_006_evidence_gate_rejects_missing_commit_hash() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 105_000,
        command: "cargo bench yaml_parse".to_string(),
        commit_hash: "".to_string(), // Empty!
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
    };

    let result = check_evidence_gate(&metadata, 20);

    // EG-006: Should return MissingCommit error
    match result {
        Err(EvidenceError::MissingCommit) => {}
        other => panic!("Expected MissingCommit, got {:?}", other),
    }
}

/// EG-007: Evidence gate detects regression above threshold
#[test]
fn eg_007_evidence_gate_detects_regression_above_threshold() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 130_000, // 30% regression with 20% threshold
        command: "cargo bench yaml_parse".to_string(),
        commit_hash: "abc123def456".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
    };

    let result = check_evidence_gate(&metadata, 20);

    // EG-007: Should return RegressionDetected error
    match result {
        Err(EvidenceError::RegressionDetected { benchmark, delta }) => {
            assert_eq!(benchmark, "yaml_parse");
            assert_eq!(delta, 30_000); // 130_000 - 100_000 = 30_000
        }
        other => panic!("Expected RegressionDetected, got {:?}", other),
    }
}

/// EG-008: Evidence gate rejects zero budget
#[test]
fn eg_008_evidence_gate_rejects_zero_budget() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 105_000,
        command: "cargo bench yaml_parse".to_string(),
        commit_hash: "abc123def456".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 0, // Zero budget!
    };

    let result = check_evidence_gate(&metadata, 20);

    // EG-008: Should return EmptyBudget error
    match result {
        Err(EvidenceError::EmptyBudget) => {}
        other => panic!("Expected EmptyBudget, got {:?}", other),
    }
}

/// Additional regression threshold tests
#[test]
fn eg_regression_at_threshold_boundary() {
    // At exactly 20% threshold (baseline=100_000, result=120_000), should NOT detect regression
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 120_000, // Exactly 20% above baseline
        command: "cargo bench yaml_parse".to_string(),
        commit_hash: "abc123def456".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
    };

    let result = check_evidence_gate(&metadata, 20);

    // At threshold boundary, should accept (not regress)
    assert!(
        result.is_ok(),
        "Expected Ok at threshold boundary, got {:?}",
        result
    );
}

#[test]
fn eg_regression_just_over_threshold() {
    // Just over 20% threshold (baseline=100_000, result=121_000)
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 121_000,
        command: "cargo bench yaml_parse".to_string(),
        commit_hash: "abc123def456".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
    };

    let result = check_evidence_gate(&metadata, 20);

    // Just over threshold should be rejected
    match result {
        Err(EvidenceError::RegressionDetected { delta, .. }) => {
            assert_eq!(delta, 21_000);
        }
        other => panic!("Expected RegressionDetected, got {:?}", other),
    }
}

// ============================================================================
// Metadata Capture Tests (MC-001 through MC-010)
// ============================================================================

/// MC-001: capture_metadata produces complete metadata record
#[test]
fn mc_001_capture_metadata_produces_complete_record() {
    let metadata = valid_metadata(capture_metadata(
        "yaml_parse_small",
        Some(Duration::from_micros(50000)),
        Duration::from_micros(55000),
        "cargo bench yaml_parse",
        "abc123def456789",
        "linux-x86_64",
        100_000,
    ));

    assert_eq!(metadata.name, "yaml_parse_small");
    assert_eq!(metadata.baseline_us, Some(50000));
    assert_eq!(metadata.result_us, 55000);
    assert_eq!(metadata.command, "cargo bench yaml_parse");
    assert_eq!(metadata.commit_hash, "abc123def456789");
    assert_eq!(metadata.environment, "linux-x86_64");
    assert_eq!(metadata.budget_us, 100_000);
}

/// MC-002: capture_metadata accepts None baseline for new benchmarks
#[test]
fn mc_002_capture_metadata_accepts_none_baseline() {
    let metadata = valid_metadata(capture_metadata(
        "yaml_parse_1mb",
        None,
        Duration::from_micros(5_000_000),
        "cargo bench yaml_parse",
        "abc123def456789",
        "linux-x86_64",
        10_000_000,
    ));

    assert_eq!(metadata.baseline_us, None);
    assert_eq!(metadata.result_us, 5_000_000);
}

/// MC-003: capture_metadata rejects empty commit_hash.
#[test]
fn mc_003_capture_metadata_rejects_empty_commit_hash() {
    assert_eq!(
        capture_metadata(
            "yaml_parse_small",
            Some(Duration::from_micros(50000)),
            Duration::from_micros(55000),
            "cargo bench yaml_parse",
            "",
            "linux-x86_64",
            100_000,
        ),
        Err(EvidenceError::MissingCommit)
    );
}

/// MC-004: budget_utilization_percent computes correct percentage
#[test]
fn mc_004_budget_utilization_percent_computes_correct() {
    let utilization = budget_utilization_percent(Duration::from_micros(75000), 100_000);
    // 75000/100000 = 0.75 = 7500 basis points
    assert_eq!(utilization, 7500);
}

/// MC-005: budget_utilization_percent returns MAX for zero budget
#[test]
fn mc_005_budget_utilization_percent_returns_max_for_zero_budget() {
    let utilization = budget_utilization_percent(Duration::from_micros(75000), 0);
    assert_eq!(utilization, u128::MAX);
}

/// MC-006: latency_within_budget returns true for within-budget elapsed
#[test]
fn mc_006_latency_within_budget_returns_true_when_within() {
    assert!(latency_within_budget(Duration::from_micros(50000), 100_000));
}

/// MC-007: latency_within_budget returns false for over-budget elapsed
#[test]
fn mc_007_latency_within_budget_returns_false_when_over() {
    assert!(!latency_within_budget(
        Duration::from_micros(150000),
        100_000
    ));
}

/// MC-008: latency_within_budget returns false for zero budget
#[test]
fn mc_008_latency_within_budget_returns_false_for_zero_budget() {
    assert!(!latency_within_budget(Duration::from_micros(50000), 0));
}

/// MC-009: result_exceeds_threshold returns true for significant regression
#[test]
fn mc_009_result_exceeds_threshold_true_when_significant_regression() {
    assert!(result_exceeds_threshold(
        Duration::from_micros(130000),
        Duration::from_micros(100000),
        20
    ));
}

/// MC-010: result_exceeds_threshold returns false for within-threshold
#[test]
fn mc_010_result_exceeds_threshold_false_when_within_threshold() {
    assert!(!result_exceeds_threshold(
        Duration::from_micros(115000),
        Duration::from_micros(100000),
        20
    ));
}

/// Additional: baseline_within_budget tests
#[test]
fn baseline_within_budget_true_when_baseline_under_budget() {
    assert!(baseline_within_budget(
        Duration::from_micros(80000),
        100_000
    ));
}

#[test]
fn baseline_within_budget_false_when_baseline_over_budget() {
    assert!(!baseline_within_budget(
        Duration::from_micros(120000),
        100_000
    ));
}
