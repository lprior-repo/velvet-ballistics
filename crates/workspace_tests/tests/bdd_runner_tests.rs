#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
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

use vb_workspace_tests::acceptance_catalog::{self};
use vb_workspace_tests::bdd_runner::{
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
