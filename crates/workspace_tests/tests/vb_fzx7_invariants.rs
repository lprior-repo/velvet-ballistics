#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::derivable_impls,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::io_other_error,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::new_without_default,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

//! Invariant Tests with Proptest
//!
//! Property-based tests for evidence gate arithmetic invariants and metadata
//! field invariants.
//!
//! # RED PHASE
//! These tests COMPILE but FAIL because the implementation contains intentional bugs.

use proptest::prelude::*;
use std::time::Duration;
use vb_benchmark::{
    baseline_within_budget, budget_utilization_percent, capture_metadata, latency_within_budget,
    result_exceeds_threshold,
};

// ============================================================================
// Evidence Gate Arithmetic Invariants (Section 4.1 of Test Plan)
// ============================================================================

proptest! {
    /// INVARIANT: result_exceeds_threshold is false when result equals baseline
    #[test]
    fn prop_result_exceeds_threshold_false_when_result_equals_baseline(baseline_us in 1u64..1_000_000_000u64) {
        let baseline = Duration::from_micros(baseline_us);
        let threshold_pct = 20u64;

        // result == baseline should never exceed threshold
        prop_assert!(!result_exceeds_threshold(baseline, baseline, threshold_pct));
    }

    /// INVARIANT: result_exceeds_threshold true when result significantly greater (2x)
    #[test]
    fn prop_result_exceeds_threshold_true_when_result_significantly_greater(baseline_us in 1u64..500_000_000u64) {
        let baseline = Duration::from_micros(baseline_us);
        let threshold_pct = 20u64;
        // result = 2 * baseline should exceed threshold
        let result = Duration::from_micros(baseline_us.saturating_mul(2));

        prop_assert!(result_exceeds_threshold(result, baseline, threshold_pct));
    }

    /// INVARIANT: latency_within_budget false when elapsed > budget
    #[test]
    fn prop_latency_within_budget_false_when_elapsed_greater_than_budget(budget_us in 1u64..1_000_000_000u64) {
        let budget = budget_us;
        // elapsed > budget
        let elapsed = Duration::from_micros(budget.saturating_add(1));

        prop_assert!(!latency_within_budget(elapsed, budget));
    }

    /// INVARIANT: latency_within_budget true when elapsed < budget
    #[test]
    fn prop_latency_within_budget_true_when_elapsed_less_than_budget(budget_us in 2u64..1_000_000_000u64) {
        let budget = budget_us;
        // elapsed < budget
        let elapsed = Duration::from_micros(budget - 1);

        prop_assert!(latency_within_budget(elapsed, budget));
    }

    /// INVARIANT: budget_utilization_percent never exceeds 10000 for elapsed <= budget
    #[test]
    fn prop_budget_utilization_percent_never_exceeds_10000_for_valid_budget(
        elapsed_us in 0u64..1_000_000_000u64,
        budget_us in 1u64..1_000_000_000u64
    ) {
        let elapsed = Duration::from_micros(elapsed_us);
        let budget = budget_us;

        // Only test when elapsed <= budget
        if elapsed_us <= budget_us {
            let utilization = budget_utilization_percent(elapsed, budget);
            prop_assert!(utilization <= 10000, "utilization {} should be <= 10000 when elapsed <= budget", utilization);
        }
    }

    /// INVARIANT: budget_utilization_percent returns MAX for zero budget
    #[test]
    fn prop_budget_utilization_percent_returns_max_for_zero_budget(elapsed_us in 0u64..1_000_000_000u64) {
        let elapsed = Duration::from_micros(elapsed_us);
        prop_assert_eq!(budget_utilization_percent(elapsed, 0), u128::MAX);
    }

    /// INVARIANT: baseline_within_budget consistent with latency_within_budget
    #[test]
    fn prop_baseline_within_budget_consistency_with_latency_within_budget(
        baseline_us in 0u64..1_000_000_000u64,
        budget_us in 0u64..1_000_000_000u64
    ) {
        let baseline = Duration::from_micros(baseline_us);
        let budget = budget_us;

        let baseline_within = baseline_within_budget(baseline, budget);
        let latency_within = latency_within_budget(baseline, budget);

        prop_assert_eq!(
            baseline_within,
            latency_within,
            "baseline_within_budget and latency_within_budget should be consistent"
        );
    }
}

// ============================================================================
// Metadata Field Invariants (Section 4.2 of Test Plan)
// ============================================================================

proptest! {
    /// INVARIANT: commit_hash must be non-empty ASCII hex
    #[test]
    fn prop_commit_hash_must_be_nonempty_ascii_hex(commit in "[a-fA-F0-9]{1,40}") {
        let metadata = capture_metadata(
            "test_bench",
            Some(Duration::from_micros(100)),
            Duration::from_micros(110),
            "cargo bench",
            &commit,
            "test-env",
            1000,
            0,
            0,
            0,
        );

        let metadata = metadata.map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert!(!metadata.commit_hash.is_empty());
        prop_assert!(metadata.commit_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// INVARIANT: environment must be non-empty
    #[test]
    fn prop_environment_must_be_nonempty(environment in "[a-zA-Z0-9_-]{1,64}") {
        let metadata = capture_metadata(
            "test_bench",
            Some(Duration::from_micros(100)),
            Duration::from_micros(110),
            "cargo bench",
            "abc123",
            &environment,
            1000,
            0,
            0,
            0,
        );

        let metadata = metadata.map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert!(!metadata.environment.is_empty());
    }

    /// INVARIANT: command must be non-empty
    #[test]
    fn prop_command_must_be_nonempty(command in "[a-zA-Z0-9_ -]{1,128}") {
        let metadata = capture_metadata(
            "test_bench",
            Some(Duration::from_micros(100)),
            Duration::from_micros(110),
            &command,
            "abc123",
            "test-env",
            1000,
            0,
            0,
            0,
        );

        let metadata = metadata.map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert!(!metadata.command.is_empty());
    }
}

// ============================================================================
// Regression Threshold Invariants (Section 4.3 of Test Plan)
// ============================================================================

proptest! {
    /// INVARIANT: regression delta computed correctly at boundary
    #[test]
    fn prop_regression_delta_computed_correctly(
        baseline_us in 1u64..1_000_000u64,
        threshold_pct in 1u64..50u64
    ) {
        let baseline = Duration::from_micros(baseline_us);
        let threshold = threshold_pct;

        // result == baseline + threshold (boundary case)
        // At exactly threshold, should NOT exceed
        let threshold_delta = baseline_us.saturating_mul(threshold) / 100;
        let result = Duration::from_micros(baseline_us.saturating_add(threshold_delta));

        let does_exceed = result_exceeds_threshold(result, baseline, threshold);

        // At the boundary (100 + threshold_pct)%, result does NOT exceed
        prop_assert!(!does_exceed, "At threshold boundary, result should NOT exceed threshold");
    }

    /// INVARIANT: 1% over threshold triggers detection
    #[test]
    fn prop_regression_1_percent_over_triggers(
        baseline_us in 100u64..1_000_000u64
    ) {
        let baseline = Duration::from_micros(baseline_us);
        let threshold_pct = 20u64;

        // result = baseline * 121 / 100 (just over 20% threshold)
        let result_us = (baseline_us.saturating_mul(121) / 100).max(baseline_us + 1);
        let result = Duration::from_micros(result_us);

        let threshold_delta = baseline_us.saturating_mul(threshold_pct) / 100;
        let delta = result_us.saturating_sub(baseline_us);

        // If delta > threshold_delta, should detect regression
        if delta > threshold_delta {
            prop_assert!(result_exceeds_threshold(result, baseline, threshold_pct),
                "delta {} > threshold {} should trigger regression detection", delta, threshold_delta);
        }
    }
}

// ============================================================================
// Budget Utilization Arithmetic Invariants
// ============================================================================

proptest! {
    /// INVARIANT: utilization is monotonic with elapsed time (for fixed budget)
    #[test]
    fn prop_utilization_monotonic_with_elapsed(
        elapsed1_us in 0u64..500_000_000u64,
        elapsed2_us in 0u64..500_000_000u64,
        budget_us in 1u64..1_000_000_000u64
    ) {
        let budget = budget_us;

        // Only test when both elapsed values are within budget
        if elapsed1_us <= budget_us && elapsed2_us <= budget_us {
            let elapsed1 = Duration::from_micros(elapsed1_us);
            let elapsed2 = Duration::from_micros(elapsed2_us);

            let util1 = budget_utilization_percent(elapsed1, budget);
            let util2 = budget_utilization_percent(elapsed2, budget);

            if elapsed1_us < elapsed2_us {
                prop_assert!(util1 <= util2,
                    "utilization should not decrease with elapsed time: {} <= {}", util1, util2);
            }
        }
    }

    /// INVARIANT: utilization doubles when elapsed doubles (for fixed budget)
    #[test]
    fn prop_utilization_doubles_with_elapsed(
        elapsed_us in 1u64..250_000_000u64,
        budget_us in 500_000_001u64..1_000_000_000u64
    ) {
        let elapsed = Duration::from_micros(elapsed_us);
        let doubled_elapsed = Duration::from_micros(elapsed_us.saturating_mul(2));
        let budget = budget_us;

        // Ensure doubled elapsed is still within budget
        if elapsed_us.saturating_mul(2) <= budget_us {
            let util1 = budget_utilization_percent(elapsed, budget);
            let util2 = budget_utilization_percent(doubled_elapsed, budget);

            let expected = util1.saturating_mul(2);
            let delta = util2.abs_diff(expected);
            prop_assert!(delta <= 1,
                "utilization should double within integer rounding tolerance: {} * 2 ~= {}", util1, util2);
        }
    }
}
