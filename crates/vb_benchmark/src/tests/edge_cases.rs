#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
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
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
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

//! Edge-case unit tests for benchmark metadata, budget, and evidence-gate functions.
//!
//! Coverage targets:
//!  - `capture_metadata` with empty name/command, duration overflow, empty environment
//!  - `budget_utilization_percent` with `u64::MAX` and zero budgets
//!  - `check_evidence_gate` with zero-latency fields in priority order
//!  - `baseline_within_budget` boundary conditions
//!  - `result_exceeds_threshold` at the exact 10% boundary

#![forbid(unsafe_code)]

use crate::*;
use std::time::Duration;

fn expect_metadata(result: Result<BenchmarkMetadata, EvidenceError>) -> BenchmarkMetadata {
    match result {
        Ok(v) => v,
        Err(e) => panic!("capture_metadata should succeed, got Err({e:?})"),
    }
}

// ============================================================================
// capture_metadata — empty name and command
// ============================================================================

/// `capture_metadata` with empty `name` and empty `command` must still succeed.
/// The struct stores empty strings verbatim; semantic emptiness is checked by
/// `check_evidence_gate`, not by `capture_metadata`.
#[test]
fn capture_metadata_empty_name_and_command() {
    let meta = expect_metadata(capture_metadata(
        "",             // name
        None,           // baseline (new benchmark)
        Duration::ZERO, // result
        "",             // command
        "abc123",       // commit_hash
        "linux-x86_64", // environment
        200_000,        // budget_us
        1,              // fjall_write_latency_ns
        1,              // direct_api_latency_ns
        1,              // ipc_latency_ns
    ));

    assert!(
        meta.name.is_empty(),
        "name should be empty string, got '{}'",
        meta.name
    );
    assert!(
        meta.command.is_empty(),
        "command should be empty string, got '{}'",
        meta.command
    );
    assert_eq!(
        meta.baseline_us, None,
        "baseline should be None for new benchmark"
    );
    assert_eq!(
        meta.result_us, 0,
        "result_us should be 0 for Duration::ZERO"
    );
}

// ============================================================================
// capture_metadata — Duration::from_secs(u64::MAX) overflow guard
// ============================================================================

/// When `baseline` is `Duration::from_secs(u64::MAX)`, `as_micros()` produces a
/// value far larger than `u64::MAX`, so `try_from` fails and `unwrap_or` returns
/// `u64::MAX`.  The result path (non-overflowing Duration::ZERO) should give 0.
#[test]
fn capture_metadata_duration_overflow_baseline() {
    let overflow_baseline = Duration::from_secs(u64::MAX);

    let meta = expect_metadata(capture_metadata(
        "overflow_test",
        Some(overflow_baseline),
        Duration::ZERO,
        "baseline-overflow",
        "def456",
        "test-env",
        100_000,
        1,
        1,
        1,
    ));

    assert_eq!(
        meta.baseline_us,
        Some(u64::MAX),
        "baseline_us should be u64::MAX when Duration::as_micros overflows"
    );
    // result_us is 0 because Duration::ZERO was passed (not the overflow duration).
    // This test verifies the overflow guard on the *baseline* path.
    assert_eq!(
        meta.result_us, 0,
        "result_us should be 0 for Duration::ZERO (non-overflowing path)"
    );
}

/// `result` at `Duration::from_secs(u64::MAX)` should also saturate to `u64::MAX`.
#[test]
fn capture_metadata_result_duration_overflow() {
    let overflow_result = Duration::from_secs(u64::MAX);

    let meta = expect_metadata(capture_metadata(
        "result_overflow",
        None,
        overflow_result,
        "result-overflow",
        "abc012",
        "test-env",
        50_000,
        5,
        3,
        2,
    ));

    assert_eq!(
        meta.result_us,
        u64::MAX,
        "result_us should be u64::MAX on result Duration overflow"
    );
}

// ============================================================================
// budget_utilization_percent — extreme budgets
// ============================================================================

/// 1 μs elapsed against a `u64::MAX` budget: the ratio is essentially zero.
#[test]
fn budget_utilization_percent_max_budget_returns_zero() {
    let pct = budget_utilization_percent(Duration::from_micros(1), u64::MAX);
    assert_eq!(
        pct, 0,
        "1 μs / u64::MAX budget should yield ~0 basis points, got {pct}"
    );
}

/// A zero budget must return `u128::MAX` (the "infinity" sentinel).
#[test]
fn budget_utilization_percent_zero_budget_returns_max() {
    let pct = budget_utilization_percent(Duration::from_micros(1), 0);
    assert_eq!(
        pct,
        u128::MAX,
        "zero budget must return u128::MAX sentinel, got {pct}"
    );
}

// ============================================================================
// check_evidence_gate — zero-latency field priority order
// ============================================================================

/// When all three latency fields are zero, the gate must return the FIRST
/// matching error: `ZeroLatencyField(FjallWrite)` (check order is Fjall →
/// DirectApi → Ipc).
#[test]
fn check_evidence_gate_all_zero_latencies_returns_first() {
    let meta = BenchmarkMetadata {
        name: String::from("all_zero"),
        baseline_us: Some(100),
        result_us: 100,
        command: String::from("run"),
        commit_hash: String::from("aabbcc11"),
        environment: String::from("test"),
        budget_us: 10_000,
        fjall_write_latency_ns: 0,
        direct_api_latency_ns: 0,
        ipc_latency_ns: 0,
    };

    let err = match check_evidence_gate(&meta, 10) {
        Err(e) => e,
        Ok(_) => panic!("expected Err, got Ok"),
    };
    match err {
        EvidenceError::ZeroLatencyField { field } => {
            assert_eq!(
                field,
                LatencyFieldId::FjallWrite,
                "first zero field check must return FjallWrite, got {field:?}"
            );
        }
        other => panic!("expected ZeroLatencyField(FjallWrite), got {other:?}"),
    }
}

/// Only `direct_api_latency_ns == 0`: must return `ZeroLatencyField(DirectApi)`.
#[test]
fn check_evidence_gate_only_direct_api_zero() {
    let meta = BenchmarkMetadata {
        name: String::from("direct_zero"),
        baseline_us: Some(100),
        result_us: 100,
        command: String::from("run"),
        commit_hash: String::from("aabbcc11"),
        environment: String::from("test"),
        budget_us: 10_000,
        fjall_write_latency_ns: 50,
        direct_api_latency_ns: 0,
        ipc_latency_ns: 30,
    };

    let err = match check_evidence_gate(&meta, 10) {
        Err(e) => e,
        Ok(_) => panic!("expected Err, got Ok"),
    };
    match err {
        EvidenceError::ZeroLatencyField { field } => {
            assert_eq!(
                field,
                LatencyFieldId::DirectApi,
                "should return DirectApi when it is the only zero latency field"
            );
        }
        other => panic!("expected ZeroLatencyField(DirectApi), got {other:?}"),
    }
}

/// Only `ipc_latency_ns == 0`: must return `ZeroLatencyField(Ipc)`.
#[test]
fn check_evidence_gate_only_ipc_zero() {
    let meta = BenchmarkMetadata {
        name: String::from("ipc_zero"),
        baseline_us: Some(100),
        result_us: 100,
        command: String::from("run"),
        commit_hash: String::from("aabbcc11"),
        environment: String::from("test"),
        budget_us: 10_000,
        fjall_write_latency_ns: 50,
        direct_api_latency_ns: 30,
        ipc_latency_ns: 0,
    };

    let err = match check_evidence_gate(&meta, 10) {
        Err(e) => e,
        Ok(_) => panic!("expected Err, got Ok"),
    };
    match err {
        EvidenceError::ZeroLatencyField { field } => {
            assert_eq!(
                field,
                LatencyFieldId::Ipc,
                "should return Ipc when it is the only zero latency field"
            );
        }
        other => panic!("expected ZeroLatencyField(Ipc), got {other:?}"),
    }
}

// ============================================================================
// capture_metadata + check_evidence_gate — empty environment chain
// ============================================================================

/// `capture_metadata` accepts an empty environment string (returns `Ok`),
/// but `check_evidence_gate` must reject it with `MissingEnvironment`.
#[test]
fn capture_metadata_empty_environment_passes_gate_rejection() {
    let meta = expect_metadata(capture_metadata(
        "env_test",
        Some(Duration::from_micros(500)),
        Duration::from_micros(520),
        "run-1",
        "deadbeef",
        "", // empty environment
        10_000,
        100,
        50,
        30,
    ));

    assert!(
        meta.environment.is_empty(),
        "environment should be empty string"
    );

    // Downstream gate must reject.
    let err = match check_evidence_gate(&meta, 10) {
        Err(e) => e,
        Ok(_) => panic!("expected Err, got Ok"),
    };
    match err {
        EvidenceError::MissingEnvironment => {}
        other => panic!(
            "check_evidence_gate with empty environment must return MissingEnvironment, got {other:?}"
        ),
    }
}

// ============================================================================
// baseline_within_budget — boundary conditions
// ============================================================================

/// 0 μs baseline ≤ 1000 μs budget → true.
#[test]
fn baseline_within_budget_zero_under_1000() {
    assert!(
        baseline_within_budget(Duration::ZERO, 1000),
        "0 μs should be within a 1000 μs budget"
    );
}

/// Exactly 1000 μs baseline ≤ 1000 μs budget → true (boundary, inclusive).
#[test]
fn baseline_within_budget_exactly_at_boundary() {
    assert!(
        baseline_within_budget(Duration::from_micros(1000), 1000),
        "1000 μs should be within a 1000 μs budget (inclusive boundary)"
    );
}

/// 1001 μs baseline > 1000 μs budget → false.
#[test]
fn baseline_within_budget_one_over_boundary() {
    assert!(
        !baseline_within_budget(Duration::from_micros(1001), 1000),
        "1001 μs must exceed a 1000 μs budget"
    );
}

// ============================================================================
// result_exceeds_threshold — at-exact boundary
// ============================================================================

/// Result exactly at baseline + 10% threshold → NOT exceeded (strict `>`).
#[test]
fn result_exceeds_threshold_exactly_at_10_percent_not_exceeded() {
    let baseline = Duration::from_micros(1_000);
    let threshold_pct: u64 = 10;

    // 10% of 1000 = 100 → threshold ceiling = 1100 μs
    // result = 1100 μs is NOT strictly greater than 1100
    let result = Duration::from_micros(1_100);

    assert!(
        !result_exceeds_threshold(result, baseline, threshold_pct),
        "result at exactly 110% of baseline must NOT exceed 10% threshold"
    );
}

/// Result one microsecond past the 10% threshold → exceeded.
#[test]
fn result_exceeds_threshold_one_past_boundary_is_exceeded() {
    let baseline = Duration::from_micros(1_000);
    let threshold_pct: u64 = 10;

    // 10% of 1000 = 100 → ceiling = 1100 μs
    // result = 1101 μs is strictly greater than 1100
    let result = Duration::from_micros(1_101);

    assert!(
        result_exceeds_threshold(result, baseline, threshold_pct),
        "1101 μs must exceed 10% threshold over 1000 μs baseline"
    );
}
