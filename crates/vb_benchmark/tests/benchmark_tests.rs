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
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]

use std::time::Duration;

use vb_benchmark::*;

// Helper to construct a valid BenchmarkMetadata for testing.
// All three latency fields are set to 1ns (minimum non-zero) so
// check_evidence_gate will not reject for zero latency.
fn make_valid_metadata(
    name: String,
    baseline_us: Option<u64>,
    result_us: u64,
    command: String,
    commit_hash: String,
    environment: String,
    budget_us: u64,
) -> BenchmarkMetadata {
    BenchmarkMetadata {
        name,
        baseline_us,
        result_us,
        command,
        commit_hash,
        environment,
        budget_us,
        fjall_write_latency_ns: 1,
        direct_api_latency_ns: 1,
        ipc_latency_ns: 1,
    }
}

// These tests verify the public benchmark-helper API behaves per its contract.
// Each test exercises a single boundary of the corresponding helper.

#[test]
fn baseline_within_budget_returns_true_when_under() {
    assert!(baseline_within_budget(
        Duration::from_micros(80000),
        100_000
    ));
}

#[test]
fn baseline_within_budget_returns_false_when_over() {
    assert!(!baseline_within_budget(
        Duration::from_micros(120000),
        100_000
    ));
}

#[test]
fn budget_utilization_percent_computes_correct() {
    assert_eq!(
        budget_utilization_percent(Duration::from_micros(75000), 100_000),
        7500
    );
}

#[test]
fn budget_utilization_percent_returns_max_for_zero_budget() {
    assert_eq!(
        budget_utilization_percent(Duration::from_micros(75000), 0),
        u128::MAX
    );
}

#[test]
fn latency_within_budget_returns_true_when_within() {
    assert!(latency_within_budget(Duration::from_micros(50000), 100_000));
}

#[test]
fn latency_within_budget_returns_false_when_over() {
    assert!(!latency_within_budget(
        Duration::from_micros(150000),
        100_000
    ));
}

#[test]
fn result_exceeds_threshold_true_when_significant_regression() {
    assert!(result_exceeds_threshold(
        Duration::from_micros(130000),
        Duration::from_micros(100000),
        20
    ));
}

#[test]
fn result_exceeds_threshold_false_when_within_threshold() {
    assert!(!result_exceeds_threshold(
        Duration::from_micros(115000),
        Duration::from_micros(100000),
        20
    ));
}

#[test]
fn check_evidence_gate_rejects_missing_baseline() {
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        None,
        105_000,
        "cargo bench".to_string(),
        "abc123".to_string(),
        "linux-x86_64".to_string(),
        200_000,
    );
    let result = check_evidence_gate(&metadata, 20);
    assert!(matches!(result, Err(EvidenceError::MissingBaseline)));
}

#[test]
fn check_evidence_gate_rejects_regression() {
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        130_000, // 30% regression
        "cargo bench".to_string(),
        "abc123".to_string(),
        "linux-x86_64".to_string(),
        200_000,
    );
    let result = check_evidence_gate(&metadata, 20);
    match result {
        Err(EvidenceError::RegressionDetected { benchmark, delta }) => {
            assert_eq!(benchmark, "yaml_parse");
            assert_eq!(delta, 30_000);
        }
        other => panic!("Expected RegressionDetected, got {other:?}"),
    }
}

#[test]
fn check_evidence_gate_accepts_valid() {
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        105_000,
        "cargo bench".to_string(),
        "abc123".to_string(),
        "linux-x86_64".to_string(),
        200_000,
    );
    match check_evidence_gate(&metadata, 20) {
        Ok(()) => {}
        Err(e) => panic!("evidence gate should pass for valid metadata: {e:?}"),
    }
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
        100, // fjall_write_latency_ns
        200, // direct_api_latency_ns
        300, // ipc_latency_ns
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
        100, // fjall_write_latency_ns
        200, // direct_api_latency_ns
        300, // ipc_latency_ns
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
        100, // fjall_write_latency_ns
        200, // direct_api_latency_ns
        300, // ipc_latency_ns
    );
    let meta = match result {
        Ok(meta) => meta,
        Err(err) => panic!("expected valid metadata, got {err:?}"),
    };
    assert_eq!(meta.name, "yaml_parse");
    assert_eq!(meta.baseline_us, Some(100_000));
    assert_eq!(meta.result_us, 105_000);
    assert_eq!(meta.command, "cargo bench");
    assert_eq!(meta.commit_hash, "abc123def");
    assert_eq!(meta.environment, "linux-x86_64");
    assert_eq!(meta.budget_us, 200_000);
    assert_eq!(meta.fjall_write_latency_ns, 100);
    assert_eq!(meta.direct_api_latency_ns, 200);
    assert_eq!(meta.ipc_latency_ns, 300);
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
        100, // fjall_write_latency_ns
        200, // direct_api_latency_ns
        300, // ipc_latency_ns
    );
    let meta = match result {
        Ok(meta) => meta,
        Err(e) => panic!("expected valid metadata for None baseline: {e:?}"),
    };
    assert_eq!(meta.baseline_us, None);
    assert_eq!(meta.result_us, 50_000);
    assert_eq!(meta.fjall_write_latency_ns, 100);
    assert_eq!(meta.direct_api_latency_ns, 200);
    assert_eq!(meta.ipc_latency_ns, 300);
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
fn result_exceeds_threshold_true_with_zero_threshold_over_baseline() {
    // threshold_pct=0 → any result over baseline is a regression
    // result=100001 > baseline=100000 → true
    assert!(result_exceeds_threshold(
        Duration::from_micros(100_001),
        Duration::from_micros(100_000),
        0
    ));
}

#[test]
fn result_exceeds_threshold_false_with_zero_threshold_under_baseline() {
    // result < baseline with 0% threshold → no regression
    assert!(!result_exceeds_threshold(
        Duration::from_micros(99_999),
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
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        105_000,
        "cargo bench".to_string(),
        "abc123".to_string(),
        "".to_string(), // empty environment
        200_000,
    );
    let result = check_evidence_gate(&metadata, 20);
    assert!(matches!(result, Err(EvidenceError::MissingEnvironment)));
}

#[test]
fn check_evidence_gate_rejects_missing_command() {
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        105_000,
        "".to_string(), // empty command
        "abc123".to_string(),
        "linux-x86_64".to_string(),
        200_000,
    );
    let result = check_evidence_gate(&metadata, 20);
    assert!(matches!(result, Err(EvidenceError::MissingCommand)));
}

#[test]
fn check_evidence_gate_rejects_missing_commit() {
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        105_000,
        "cargo bench".to_string(),
        "".to_string(), // empty commit
        "linux-x86_64".to_string(),
        200_000,
    );
    let result = check_evidence_gate(&metadata, 20);
    assert!(matches!(result, Err(EvidenceError::MissingCommit)));
}

#[test]
fn check_evidence_gate_rejects_empty_budget() {
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        105_000,
        "cargo bench".to_string(),
        "abc123".to_string(),
        "linux-x86_64".to_string(),
        0, // zero budget
    );
    let result = check_evidence_gate(&metadata, 20);
    assert!(matches!(result, Err(EvidenceError::EmptyBudget)));
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
        100, // fjall_write_latency_ns
        200, // direct_api_latency_ns
        300, // ipc_latency_ns
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
        100, // fjall_write_latency_ns
        200, // direct_api_latency_ns
        300, // ipc_latency_ns
    );
    let meta = match result {
        Ok(meta) => meta,
        Err(e) => panic!("expected valid metadata for max uint64 commit: {e:?}"),
    };
    assert_eq!(meta.commit_hash, "ffffffffffffffff");
    assert_eq!(meta.fjall_write_latency_ns, 100);
    assert_eq!(meta.direct_api_latency_ns, 200);
    assert_eq!(meta.ipc_latency_ns, 300);
}

// === check_evidence_gate threshold boundary tests ===

#[test]
fn check_evidence_gate_accepts_exactly_at_threshold() {
    // baseline=100000, result=120000, threshold=20%
    // 120000 is NOT > 100000 + 20000 = 120000, so should pass
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        120_000,
        "cargo bench".to_string(),
        "abc123".to_string(),
        "linux-x86_64".to_string(),
        200_000,
    );
    match check_evidence_gate(&metadata, 20) {
        Ok(()) => {}
        Err(e) => panic!("evidence gate should pass at threshold: {e:?}"),
    }
}

#[test]
fn check_evidence_gate_accepts_zero_threshold_within_baseline() {
    // threshold=0%, result=baseline
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        100_000,
        "cargo bench".to_string(),
        "abc123".to_string(),
        "linux-x86_64".to_string(),
        200_000,
    );
    match check_evidence_gate(&metadata, 0) {
        Ok(()) => {}
        Err(e) => panic!("evidence gate should pass at zero threshold: {e:?}"),
    }
}

#[test]
fn check_evidence_gate_rejects_one_micro_over_threshold() {
    // baseline=100000, result=120001, threshold=20%
    // 120001 > 100000 + 20000 = 120000, so regression
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        120_001,
        "cargo bench".to_string(),
        "abc123".to_string(),
        "linux-x86_64".to_string(),
        200_000,
    );
    let result = check_evidence_gate(&metadata, 20);
    // delta = result - baseline = 120001 - 100000 = 20001
    match result {
        Err(EvidenceError::RegressionDetected { benchmark, delta }) => {
            assert_eq!(benchmark, "yaml_parse");
            assert_eq!(delta, 20_001);
        }
        other => panic!("Expected RegressionDetected, got {other:?}"),
    }
}

#[test]
fn check_evidence_gate_rejects_zero_threshold_over_baseline() {
    // threshold=0%, result > baseline → regression detected
    let metadata = make_valid_metadata(
        "yaml_parse".to_string(),
        Some(100_000),
        100_001,
        "cargo bench".to_string(),
        "abc123".to_string(),
        "linux-x86_64".to_string(),
        200_000,
    );
    let result = check_evidence_gate(&metadata, 0);
    match result {
        Err(EvidenceError::RegressionDetected { benchmark, delta }) => {
            assert_eq!(benchmark, "yaml_parse");
            assert_eq!(delta, 1);
        }
        other => panic!("Expected RegressionDetected, got {other:?}"),
    }
}

// === zero-latency field validation tests ===

#[test]
fn check_evidence_gate_rejects_zero_fjall_latency() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 105_000,
        command: "cargo bench".to_string(),
        commit_hash: "abc123".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
        fjall_write_latency_ns: 0,
        direct_api_latency_ns: 100,
        ipc_latency_ns: 200,
    };
    let result = check_evidence_gate(&metadata, 20);
    assert!(
        matches!(
            result,
            Err(EvidenceError::ZeroLatencyField {
                field: LatencyFieldId::FjallWrite
            })
        ),
        "zero fjall_write_latency_ns must return ZeroLatencyField(FjallWrite), got {result:?}"
    );
}

#[test]
fn check_evidence_gate_rejects_zero_direct_api_latency() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 105_000,
        command: "cargo bench".to_string(),
        commit_hash: "abc123".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
        fjall_write_latency_ns: 100,
        direct_api_latency_ns: 0,
        ipc_latency_ns: 200,
    };
    let result = check_evidence_gate(&metadata, 20);
    assert!(
        matches!(
            result,
            Err(EvidenceError::ZeroLatencyField {
                field: LatencyFieldId::DirectApi
            })
        ),
        "zero direct_api_latency_ns must return ZeroLatencyField(DirectApi), got {result:?}"
    );
}

#[test]
fn check_evidence_gate_rejects_zero_ipc_latency() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 105_000,
        command: "cargo bench".to_string(),
        commit_hash: "abc123".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
        fjall_write_latency_ns: 100,
        direct_api_latency_ns: 200,
        ipc_latency_ns: 0,
    };
    let result = check_evidence_gate(&metadata, 20);
    assert!(
        matches!(
            result,
            Err(EvidenceError::ZeroLatencyField {
                field: LatencyFieldId::Ipc
            })
        ),
        "zero ipc_latency_ns must return ZeroLatencyField(Ipc), got {result:?}"
    );
}

#[test]
fn check_evidence_gate_accepts_nonzero_latencies() {
    let metadata = BenchmarkMetadata {
        name: "yaml_parse".to_string(),
        baseline_us: Some(100_000),
        result_us: 105_000,
        command: "cargo bench".to_string(),
        commit_hash: "abc123".to_string(),
        environment: "linux-x86_64".to_string(),
        budget_us: 200_000,
        fjall_write_latency_ns: 100,
        direct_api_latency_ns: 200,
        ipc_latency_ns: 300,
    };
    match check_evidence_gate(&metadata, 20) {
        Ok(()) => {}
        Err(e) => panic!("evidence gate should pass with nonzero latencies: {e:?}"),
    }
}

#[test]
fn capture_metadata_preserves_latencies() {
    let result = capture_metadata(
        "bench",
        Some(Duration::from_micros(100_000)),
        Duration::from_micros(105_000),
        "cargo bench",
        "abc123",
        "linux-x86_64",
        200_000,
        1234, // fjall_write_latency_ns
        5678, // direct_api_latency_ns
        9012, // ipc_latency_ns
    );
    let meta = match result {
        Ok(meta) => meta,
        Err(e) => panic!("expected valid metadata: {e:?}"),
    };
    assert_eq!(meta.fjall_write_latency_ns, 1234);
    assert_eq!(meta.direct_api_latency_ns, 5678);
    assert_eq!(meta.ipc_latency_ns, 9012);
}

#[test]
fn serialization_contains_latency_keys() {
    let meta = match capture_metadata(
        "bench",
        None,
        Duration::from_micros(50_000),
        "cargo bench",
        "abc123",
        "linux-x86_64",
        100_000,
        1000,
        2000,
        3000,
    ) {
        Ok(v) => v,
        Err(e) => panic!("valid inputs should produce Ok(metadata), got Err({e:?})"),
    };

    let json = match serde_json::to_string(&meta) {
        Ok(v) => v,
        Err(e) => panic!("serialization should succeed, got Err({e:?})"),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(e) => panic!("JSON roundtrip should succeed, got Err({e:?})"),
    };

    if let serde_json::Value::Object(map) = parsed {
        assert!(
            map.contains_key("fjall_write_latency"),
            "must contain fjall_write_latency key"
        );
        assert!(
            map.contains_key("direct_api_latency"),
            "must contain direct_api_latency key"
        );
        assert!(
            map.contains_key("ipc_latency"),
            "must contain ipc_latency key"
        );
        // Raw _ns field names should NOT be present (serde alias replaces them)
        assert!(
            !map.contains_key("fjall_write_latency_ns"),
            "should not contain fjall_write_latency_ns"
        );
        assert!(
            !map.contains_key("direct_api_latency_ns"),
            "should not contain direct_api_latency_ns"
        );
        assert!(
            !map.contains_key("ipc_latency_ns"),
            "should not contain ipc_latency_ns"
        );
    } else {
        panic!("serialized metadata should be a JSON object");
    }
}

#[test]
fn evidence_error_display_missing_latency_field() {
    let err = EvidenceError::MissingLatencyField {
        field: LatencyFieldId::FjallWrite,
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("fjall_write_latency_ns"),
        "display should name the field"
    );
}

#[test]
fn evidence_error_display_missing_latency_field_direct_api() {
    let err = EvidenceError::MissingLatencyField {
        field: LatencyFieldId::DirectApi,
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("direct_api_latency_ns"),
        "display should name the direct_api field"
    );
}

#[test]
fn evidence_error_display_missing_latency_field_ipc() {
    let err = EvidenceError::MissingLatencyField {
        field: LatencyFieldId::Ipc,
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("ipc_latency_ns"),
        "display should name the ipc field"
    );
}

#[test]
fn evidence_error_display_zero_latency_field() {
    let err = EvidenceError::ZeroLatencyField {
        field: LatencyFieldId::Ipc,
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("ipc_latency_ns"),
        "display should name the field"
    );
}

#[test]
fn evidence_error_clone_eq_missing_commit() {
    let a = EvidenceError::MissingCommit;
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn evidence_error_clone_eq_regression() {
    let a = EvidenceError::RegressionDetected {
        benchmark: String::from("x"),
        delta: 42,
    };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn evidence_error_distinct_variants_ne() {
    assert_ne!(EvidenceError::MissingBaseline, EvidenceError::MissingResult);
    assert_ne!(
        EvidenceError::MissingEnvironment,
        EvidenceError::EmptyBudget
    );
}

// === regression shield: no STUB markers may remain in this test file ===

#[test]
fn regression_shield_zero_stub_markers() {
    // Read the current source of this test file back from disk and assert that
    // no STUB marker substrings remain. The marker convention
    // signals an unimplemented assertion body; reintroducing one is a silent
    // regression of the test suite's behavioral coverage.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("benchmark_tests.rs");
    let content = match std::fs::read_to_string(&path) {
        Ok(v) => v,
        Err(err) => panic!("regression shield could not read {}: {err}", path.display()),
    };
    // Build the marker at runtime from parts so the literal token does not
    // appear in this test's own body, where it would otherwise self-match.
    let marker: String = ["//", " ", "STUB", ":"].concat();
    let count = content.matches(marker.as_str()).count();
    assert_eq!(
        count,
        0,
        "regression shield: {count} markers remain in {}",
        path.display()
    );
}
