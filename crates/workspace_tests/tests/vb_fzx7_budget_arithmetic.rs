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
    clippy::enum_variant_names,
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

//! Budget Arithmetic Tests
//!
//! Tests for pure arithmetic functions: `budget_utilization_percent`,
//! `latency_within_budget`, `result_exceeds_threshold`, `baseline_within_budget`.
//!
//! # RED PHASE
//! These tests COMPILE but FAIL because the implementation contains intentional bugs.

use std::time::Duration;
use vb_benchmark::{
    baseline_within_budget, budget_utilization_percent, latency_within_budget,
    result_exceeds_threshold,
};

// ============================================================================
// budget_utilization_percent Tests
// ============================================================================

#[test]
fn budget_utilization_100_percent() {
    // When elapsed == budget, utilization should be 10000 (100% in basis points)
    let utilization = budget_utilization_percent(Duration::from_micros(100000), 100000);
    assert_eq!(utilization, 10000);
}

#[test]
fn budget_utilization_50_percent() {
    let utilization = budget_utilization_percent(Duration::from_micros(50000), 100000);
    assert_eq!(utilization, 5000);
}

#[test]
fn budget_utilization_0_percent() {
    let utilization = budget_utilization_percent(Duration::from_micros(0), 100000);
    assert_eq!(utilization, 0);
}

#[test]
fn budget_utilization_over_100_percent() {
    // When elapsed > budget, utilization should exceed 10000
    let utilization = budget_utilization_percent(Duration::from_micros(150000), 100000);
    assert!(utilization > 10000);
    assert_eq!(utilization, 15000); // 150000/100000 * 10000 = 15000
}

#[test]
fn budget_utilization_zero_budget_returns_max() {
    let utilization = budget_utilization_percent(Duration::from_micros(50000), 0);
    assert_eq!(utilization, u128::MAX);
}

#[test]
fn budget_utilization_small_values() {
    // Test with small values to ensure no overflow
    let utilization = budget_utilization_percent(Duration::from_micros(1), 1000);
    assert_eq!(utilization, 10); // 1/1000 * 10000 = 10
}

// ============================================================================
// latency_within_budget Tests
// ============================================================================

#[test]
fn latency_within_budget_exactly_at_budget() {
    // At exactly budget boundary, should return true
    assert!(latency_within_budget(Duration::from_micros(100000), 100000));
}

#[test]
fn latency_within_budget_under_budget() {
    assert!(latency_within_budget(Duration::from_micros(50000), 100000));
}

#[test]
fn latency_within_budget_over_budget() {
    assert!(!latency_within_budget(
        Duration::from_micros(100001),
        100000
    ));
}

#[test]
fn latency_within_budget_zero_elapsed() {
    assert!(latency_within_budget(Duration::from_micros(0), 100000));
}

#[test]
fn latency_within_budget_zero_budget_always_false() {
    assert!(!latency_within_budget(Duration::from_micros(0), 0));
    assert!(!latency_within_budget(Duration::from_micros(1), 0));
    assert!(!latency_within_budget(Duration::from_micros(100000), 0));
}

// ============================================================================
// result_exceeds_threshold Tests
// ============================================================================

#[test]
fn result_exceeds_threshold_exactly_at_baseline() {
    // result == baseline should NOT exceed threshold
    assert!(!result_exceeds_threshold(
        Duration::from_micros(100000),
        Duration::from_micros(100000),
        20
    ));
}

#[test]
fn result_exceeds_threshold_just_under_threshold() {
    // result = baseline + threshold - 1 should NOT exceed
    let baseline = Duration::from_micros(100000);
    let threshold_pct = 20u64;
    let threshold_delta = 100000u64 * threshold_pct / 100; // 20000
    let result = Duration::from_micros(baseline.as_micros() as u64 + threshold_delta - 1);

    assert!(!result_exceeds_threshold(result, baseline, threshold_pct));
}

#[test]
fn result_exceeds_threshold_exactly_at_threshold() {
    // result = baseline + threshold should NOT exceed (boundary case)
    let baseline = Duration::from_micros(100000);
    let threshold_pct = 20u64;
    let threshold_delta = 100000u64 * threshold_pct / 100; // 20000
    let result = Duration::from_micros(baseline.as_micros() as u64 + threshold_delta);

    assert!(!result_exceeds_threshold(result, baseline, threshold_pct));
}

#[test]
fn result_exceeds_threshold_just_over_threshold() {
    // result = baseline + threshold + 1 should exceed
    let baseline = Duration::from_micros(100000);
    let threshold_pct = 20u64;
    let threshold_delta = 100000u64 * threshold_pct / 100; // 20000
    let result = Duration::from_micros(baseline.as_micros() as u64 + threshold_delta + 1);

    assert!(result_exceeds_threshold(result, baseline, threshold_pct));
}

#[test]
fn result_exceeds_threshold_double_the_baseline() {
    // result = 2 * baseline should definitely exceed any reasonable threshold
    assert!(result_exceeds_threshold(
        Duration::from_micros(200000),
        Duration::from_micros(100000),
        20
    ));
}

#[test]
fn result_exceeds_threshold_zero_threshold() {
    // With 0% threshold, any increase should be detected
    assert!(result_exceeds_threshold(
        Duration::from_micros(100001),
        Duration::from_micros(100000),
        0
    ));
}

#[test]
fn result_exceeds_threshold_result_less_than_baseline() {
    // result < baseline should never exceed threshold
    assert!(!result_exceeds_threshold(
        Duration::from_micros(90000),
        Duration::from_micros(100000),
        20
    ));
}

// ============================================================================
// baseline_within_budget Tests
// ============================================================================

#[test]
fn baseline_within_budget_exactly_at_budget() {
    assert!(baseline_within_budget(
        Duration::from_micros(100000),
        100000
    ));
}

#[test]
fn baseline_within_budget_under_budget() {
    assert!(baseline_within_budget(Duration::from_micros(50000), 100000));
}

#[test]
fn baseline_within_budget_over_budget() {
    assert!(!baseline_within_budget(
        Duration::from_micros(100001),
        100000
    ));
}

#[test]
fn baseline_within_budget_zero_baseline() {
    assert!(baseline_within_budget(Duration::from_micros(0), 100000));
}

#[test]
fn baseline_within_budget_zero_budget() {
    // With zero budget, nothing should be within budget (not even zero baseline)
    // Actually, zero baseline IS within zero budget (0 <= 0)
    assert!(baseline_within_budget(Duration::from_micros(0), 0));
    assert!(!baseline_within_budget(Duration::from_micros(1), 0));
}

// ============================================================================
// Consistency Tests
// ============================================================================

#[test]
fn budget_utilization_and_latency_consistency() {
    // If latency_within_budget returns true, budget_utilization should be <= 10000
    let elapsed = Duration::from_micros(50000);
    let budget_us = 100000u64;

    if latency_within_budget(elapsed, budget_us) {
        let utilization = budget_utilization_percent(elapsed, budget_us);
        assert!(
            utilization <= 10000,
            "utilization {} should be <= 10000 when within budget",
            utilization
        );
    }
}

#[test]
fn baseline_and_latency_consistency() {
    // baseline_within_budget and latency_within_budget should be consistent
    // for the same duration values
    let baseline = Duration::from_micros(80000);
    let budget_us = 100000u64;

    let baseline_within = baseline_within_budget(baseline, budget_us);
    let latency_within = latency_within_budget(baseline, budget_us);

    assert_eq!(
        baseline_within, latency_within,
        "baseline_within_budget and latency_within_budget should return same value for same inputs"
    );
}
