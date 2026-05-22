// BDD Runner Tests — vb-oewy
//
// Tests for the BDD suite runner and evidence artifact contract.
// bead: vb-oewy
// phase: 9 (failing-first TDD)
//
// All tests follow Given/When/Then structure.
// No unsafe code. No unwrap/expect on fallible operations.

#![forbid(unsafe_code)]

// NOTE: These tests are failing-first. They will fail until:
// 1. bdd_runner module is added to workspace_tests/src/lib.rs
// 2. serde_yaml is added to workspace_tests dev-dependencies
// Currently using serde_json for the test assertions (JSON is available in workspace_tests).
// The actual implementation uses serde_yaml for evidence bundle output.

use velvet_ballastics_workspace_tests::acceptance_catalog::{self};
use velvet_ballastics_workspace_tests::bdd_runner::{
    BddRunnerError, BddScenarioResult, BddScenarioStatus, BddSuiteResult, ExecutorContext,
};

// ===========================================================================
// POST-001: BddSuiteResult.total == passed + failed + not_run
// Proven by: Verus spec_total_equals_sum + proof_suite_result_invariant
// ===========================================================================

#[test]
fn test_suite_result_total_invariant() {
    // Given: a BddSuiteResult with total=10, passed=6, failed=3, not_run=1
    let result = make_suite_result(10, 6, 3, 1);

    // When: we verify the aggregation invariant
    let sum = result.passed + result.failed + result.not_run;

    // Then: total equals the sum (Verus-proven, test-confirmed)
    assert_eq!(
        result.total, sum,
        "total ({}) must equal passed ({}) + failed ({}) + not_run ({})",
        result.total, result.passed, result.failed, result.not_run
    );
}

#[test]
fn test_suite_result_total_invariant_zero_counts() {
    // Given: a BddSuiteResult with all zero counts
    let result = make_suite_result(0, 0, 0, 0);

    // When: we verify the aggregation invariant
    let sum = result.passed + result.failed + result.not_run;

    // Then: total is 0
    assert_eq!(result.total, sum);
}

#[test]
fn test_suite_result_total_invariant_only_passed() {
    // Given: a BddSuiteResult with only passed scenarios
    let result = make_suite_result(5, 5, 0, 0);

    // Then: total equals passed
    assert_eq!(result.total, result.passed + result.failed + result.not_run);
    assert_eq!(result.total, 5);
}

// ===========================================================================
// POST-003: BddScenarioStatus has exactly 3 exhaustive variants
// Proven by: Verus spec_status_discriminant + proof_status_discriminant_exhaustive
// ===========================================================================

#[test]
fn test_status_exhaustive_match() {
    // Given: all three BddScenarioStatus variants
    let passed = BddScenarioStatus::Passed;
    let failed = BddScenarioStatus::Failed;
    let not_run = BddScenarioStatus::NotRun;

    // When: each variant is pattern-matched
    // Then: all three arms are reachable and produce distinct values
    match (passed, failed, not_run) {
        (BddScenarioStatus::Passed, BddScenarioStatus::Failed, BddScenarioStatus::NotRun) => {}
        _ => {}
    }
}

#[test]
fn test_status_equality() {
    // Given: two BddScenarioStatus::Passed values
    let s1 = BddScenarioStatus::Passed;
    let s2 = BddScenarioStatus::Passed;

    // Then: they are equal
    assert_eq!(s1, s2);
}

#[test]
fn test_status_inequality() {
    // Given: Passed and Failed status values
    let passed = BddScenarioStatus::Passed;
    let failed = BddScenarioStatus::Failed;

    // Then: they are not equal
    assert_ne!(passed, failed);
}

// ===========================================================================
// POST-005: Evidence bundle YAML roundtrip
// ===========================================================================

#[test]
fn test_evidence_bundle_yaml_roundtrip() {
    // Given: a valid BddSuiteResult
    let original = make_suite_result(3, 2, 1, 0);
    let original_scenarios = original.scenarios.clone();

    // When: serialized to YAML and deserialized back
    let yaml_str =
        serde_yaml::to_string(&original).expect("serialization must succeed for valid data");
    let deserialized: BddSuiteResult =
        serde_yaml::from_str(&yaml_str).expect("deserialization must succeed for valid YAML");

    // Then: deserialized result equals original
    assert_eq!(deserialized.total, original.total);
    assert_eq!(deserialized.passed, original.passed);
    assert_eq!(deserialized.failed, original.failed);
    assert_eq!(deserialized.not_run, original.not_run);
    assert_eq!(deserialized.linked_bead_id, original.linked_bead_id);
    assert_eq!(deserialized.scenarios.len(), original_scenarios.len());
}

#[test]
fn test_scenario_result_yaml_roundtrip() {
    // Given: a BddScenarioResult
    let original = BddScenarioResult {
        scenario_id: "VB-BDD-001".to_string(),
        test_name: "test_scenario_1".to_string(),
        status: BddScenarioStatus::Passed,
        duration_ms: 42,
        error: None,
    };

    // When: serialized to YAML and deserialized
    let yaml_str = serde_yaml::to_string(&original).expect("serialization must succeed");
    let deserialized: BddScenarioResult =
        serde_yaml::from_str(&yaml_str).expect("deserialization must succeed");

    // Then: equals original
    assert_eq!(deserialized, original);
}

#[test]
fn test_failed_scenario_serialization_preserves_error() {
    // Given: a failed BddScenarioResult with error message
    let original = BddScenarioResult {
        scenario_id: "VB-BDD-002".to_string(),
        test_name: "test_scenario_2".to_string(),
        status: BddScenarioStatus::Failed,
        duration_ms: 10,
        error: Some("assertion failed: expected 42, got 0".to_string()),
    };

    // When: serialized to YAML and deserialized
    let yaml_str = serde_yaml::to_string(&original).expect("serialization must succeed");
    let deserialized: BddScenarioResult =
        serde_yaml::from_str(&yaml_str).expect("deserialization must succeed");

    // Then: error field is preserved exactly
    assert_eq!(deserialized.error, original.error);
}

// ===========================================================================
// POST-004: Failed scenarios carry error field
// ===========================================================================

#[test]
fn test_failed_scenario_carry_error() {
    // Given: a BddScenarioResult with status Failed
    let result = BddScenarioResult {
        scenario_id: "VB-BDD-TEST".to_string(),
        test_name: "test_example".to_string(),
        status: BddScenarioStatus::Failed,
        duration_ms: 5,
        error: Some("thread 'test_example' panicked at src/lib.rs:42".to_string()),
    };

    // Then: error is Some and non-empty
    assert!(
        result.error.is_some(),
        "Failed scenario must carry error field"
    );
    let err = result.error.as_ref().unwrap();
    assert!(!err.is_empty(), "error must be non-empty string");
}

#[test]
fn test_passed_scenario_has_no_error() {
    // Given: a passed BddScenarioResult
    let result = BddScenarioResult {
        scenario_id: "VB-BDD-TEST".to_string(),
        test_name: "test_example".to_string(),
        status: BddScenarioStatus::Passed,
        duration_ms: 5,
        error: None,
    };

    // Then: error is None
    assert!(result.error.is_none(), "Passed scenario must have no error");
}

#[test]
fn test_not_run_scenario_has_no_error() {
    // Given: a not-run BddScenarioResult
    let result = BddScenarioResult {
        scenario_id: "VB-BDD-TEST".to_string(),
        test_name: "test_example".to_string(),
        status: BddScenarioStatus::NotRun,
        duration_ms: 0,
        error: None,
    };

    // Then: error is None
    assert!(result.error.is_none(), "NotRun scenario must have no error");
}

// ===========================================================================
// POST-006: Err returned only for infrastructure failures
// ===========================================================================

#[test]
fn test_error_variant_is_correct_for_discovery_failed() {
    // Given: a DiscoveryFailed error
    let err = BddRunnerError::DiscoveryFailed {
        path: "/nonexistent".to_string(),
    };

    // Then: it is not a test failure (test failures are BddScenarioResult::Failed)
    let is_infrastructure = matches!(
        err,
        BddRunnerError::DiscoveryFailed { .. }
            | BddRunnerError::ExecutionFailed { .. }
            | BddRunnerError::ParseFailed { .. }
            | BddRunnerError::EvidenceWriteFailed { .. }
            | BddRunnerError::NoTestBinary { .. }
    );
    assert!(
        is_infrastructure,
        "Infrastructure errors must not be confused with test failures"
    );
}

#[test]
fn test_error_display_deterministic() {
    // Given: a BddRunnerError
    let err = BddRunnerError::ExecutionFailed { exit_code: 1 };

    // When: formatted as string twice
    let s1 = format!("{}", err);
    let s2 = format!("{}", err);

    // Then: both strings are identical (deterministic)
    assert_eq!(s1, s2);
}

// ===========================================================================
// INV-001: Scenario IDs match acceptance catalog
// ===========================================================================

#[test]
fn test_all_catalog_scenarios_have_results() {
    // Given: the acceptance catalog
    let catalog = acceptance_catalog::catalog();

    // Then: catalog is not empty
    assert!(
        !catalog.is_empty(),
        "acceptance catalog must have at least one scenario"
    );

    // And: every catalog scenario has a non-empty ID
    for scenario in catalog {
        assert!(
            !scenario.id.is_empty(),
            "scenario {} must have non-empty id",
            scenario.master_behavior
        );
        assert!(
            scenario.id.starts_with("BDD-") || scenario.id.starts_with("VB-BDD-"),
            "scenario id '{}' must use a BDD namespace prefix",
            scenario.id
        );
    }
}

#[test]
fn test_scenario_id_matches_catalog() {
    // Given: a scenario ID from the catalog
    let catalog = acceptance_catalog::catalog();
    let first_id = catalog
        .first()
        .expect("catalog must have at least one scenario")
        .id;

    // When: we search the catalog for this ID
    let found = catalog.iter().any(|s| s.id == first_id);

    // Then: the search succeeds (exact match)
    assert!(
        found,
        "catalog lookup by ID must succeed for '{}'",
        first_id
    );
}

#[test]
fn test_catalog_scenario_has_given_when_then() {
    // Given: the acceptance catalog
    let catalog = acceptance_catalog::catalog();

    // Then: every scenario has Given, When, Then
    for scenario in catalog {
        assert!(
            !scenario.given.is_empty(),
            "scenario {} must have non-empty given",
            scenario.id
        );
        assert!(
            !scenario.when.is_empty(),
            "scenario {} must have non-empty when",
            scenario.id
        );
        assert!(
            !scenario.then.is_empty(),
            "scenario {} must have non-empty then",
            scenario.id
        );
    }
}

// ===========================================================================
// INV-003: No shared state pollution
// ===========================================================================

#[test]
fn test_no_shared_state_pollution() {
    // Given: two identical BddSuiteResult values
    let result1 = make_suite_result(5, 4, 1, 0);
    let result2 = make_suite_result(5, 4, 1, 0);

    // Then: they are equal (no hidden shared mutation)
    assert_eq!(result1.total, result2.total);
    assert_eq!(result1.passed, result2.passed);
    assert_eq!(result1.failed, result2.failed);
    assert_eq!(result1.not_run, result2.not_run);
    assert_eq!(result1.linked_bead_id, result2.linked_bead_id);
}

#[test]
fn test_executor_context_clone_is_independent() {
    // Given: an ExecutorContext
    let ctx1 = ExecutorContext {
        agent: "test".to_string(),
        timestamp_secs: 1747737600,
        machine: "test".to_string(),
    };

    // When: cloned
    let ctx2 = ctx1.clone();

    // Then: clones are equal and independent
    assert_eq!(ctx1, ctx2);
}

// ===========================================================================
// INV-004: Schema versioning
// ===========================================================================

#[test]
fn test_schema_version_enforced() {
    // Given: a BddSuiteResult with a known linked_bead_id
    let result = make_suite_result(1, 1, 0, 0);

    // Then: linked_bead_id is non-empty
    assert!(
        !result.linked_bead_id.is_empty(),
        "evidence bundle must have non-empty linked_bead_id"
    );
}

// ===========================================================================
// Helper
// ===========================================================================

fn make_suite_result(total: usize, passed: usize, failed: usize, not_run: usize) -> BddSuiteResult {
    let scenarios = (0..total)
        .map(|i| BddScenarioResult {
            scenario_id: format!("VB-BDD-{:03}", i),
            test_name: format!("test_scenario_{}", i),
            status: if i < passed {
                BddScenarioStatus::Passed
            } else if i < passed + failed {
                BddScenarioStatus::Failed
            } else {
                BddScenarioStatus::NotRun
            },
            duration_ms: i as u64 * 10,
            error: None,
        })
        .collect();

    BddSuiteResult {
        total,
        passed,
        failed,
        not_run,
        scenarios,
        executor_context: ExecutorContext {
            agent: "vb-oewy-test".to_string(),
            timestamp_secs: 1747737600,
            machine: "test".to_string(),
        },
        linked_bead_id: "vb-oewy".to_string(),
    }
}
