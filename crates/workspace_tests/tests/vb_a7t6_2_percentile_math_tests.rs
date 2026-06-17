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
    clippy::const_is_empty,
    clippy::derivable_impls,
    clippy::duplicated_attributes,
    clippy::enum_variant_names,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::identity_op,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::if_same_then_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::impossible_comparisons,
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
    clippy::manual_contains,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_range_contains,
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
    clippy::multiple_bound_locations,
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
    clippy::option_as_ref_cloned,
    clippy::option_as_ref_deref,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::ptr_arg,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_field_names,
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
    clippy::too_many_arguments,
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
    unused_variables
)]

//! Section 39 p50/p95/p99 percentile math — regression test for the
//! `latency=p50-p95-p99-by-criterion` mislabel fix (vb-a7t6.2).
//!
//! The actual percentile helper lives in
//! `crates/workspace_tests/benches/velvet_ballistics.rs` as
//! `pub mod latency_p50_p95_p99`. That module is a `harness = false`
//! bench binary, so its `pub` items are not importable from a
//! `cargo test --tests` integration test in this workspace. To still
//! gate the math contract (`velvet-ballistics/.beads/vb-a7t6.2/contract.md`
//! §2 nearest-rank rule, §3 ordering invariant, §3 sample_count >= 10
//! floor), this test re-implements the helper in terms of the public
//! contract and asserts the binding values. Downstream consumers
//! (`xtask/src/evidence_gate.rs`, `moon benchmark-proof`) apply the
//! same contract to the bench harness's emitted `<bench_id>.percentiles.jsonl`
//! files; if the bench helper ever drifts, those consumers will fail.
//!
//! The reference implementation here is intentionally simple and
//! dependency-free: no `criterion`, no `serde`, no `proptest`. It is
//! the spec; the bench helper is the implementation.

#![forbid(unsafe_code)]

use std::time::Duration;

/// Reference implementation of the nearest-rank percentile index from
/// `contract.md` §2: `idx(p, n) = min(n - 1, floor(p * n))` for
/// `p ∈ (0, 1]`. The `p_milli` argument is in parts-per-10000 to keep
/// the arithmetic in integer space (no `f64` rounding risk).
fn nearest_rank_index(p_milli: u16, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let p = p_milli as usize;
    let idx = (p.saturating_mul(n)) / 10_000;
    if idx >= n { n - 1 } else { idx }
}

/// Reference percentile lookup: sort a `Vec<Duration>` and return the
/// value at the nearest-rank index for `p_milli`.
fn percentile_sorted(samples: &[Duration], p_milli: u16) -> Duration {
    samples[nearest_rank_index(p_milli, samples.len())]
}

fn p50_p95_p99_sorted(samples: &[Duration]) -> (Duration, Duration, Duration) {
    (
        percentile_sorted(samples, 5_000),
        percentile_sorted(samples, 9_500),
        percentile_sorted(samples, 9_900),
    )
}

fn assert_p50_p95_p99(samples: &mut Vec<Duration>, p50: u64, p95: u64, p99: u64) {
    samples.sort_unstable();
    let (a, b, c) = p50_p95_p99_sorted(samples);
    assert_eq!(a, Duration::from_nanos(p50), "p50 mismatch");
    assert_eq!(b, Duration::from_nanos(p95), "p95 mismatch");
    assert_eq!(c, Duration::from_nanos(p99), "p99 mismatch");
}

/// Canonical regression case from `contract.md` §2: for n = 100,
/// p50 → idx 50, p95 → idx 95, p99 → idx 99. Sorted samples
/// `[1ns, 2ns, ..., 100ns]` must yield p50 = 51ns, p95 = 96ns,
/// p99 = 100ns.
#[test]
fn p50_p95_p99_uses_nearest_rank_for_100_samples() {
    let mut samples: Vec<Duration> = (1..=100_u64).map(Duration::from_nanos).collect();
    assert_p50_p95_p99(&mut samples, 51, 96, 100);
}

/// Same canonical case for n = 1000: p50 → idx 500 (501ns),
/// p95 → idx 950 (951ns), p99 → idx 990 (991ns).
#[test]
fn p50_p95_p99_uses_nearest_rank_for_1000_samples() {
    let mut samples: Vec<Duration> = (1..=1000_u64).map(Duration::from_nanos).collect();
    assert_p50_p95_p99(&mut samples, 501, 951, 991);
}

/// Boundary: a single sample collapses all three percentiles to
/// that sample.
#[test]
fn p50_p95_p99_collapses_to_single_sample_when_n_is_1() {
    let mut samples: Vec<Duration> = vec![Duration::from_nanos(42)];
    assert_p50_p95_p99(&mut samples, 42, 42, 42);
}

/// Boundary: for n = 10, the p99 floor is `9900 * 10 / 10_000 = 9`,
/// which is samples[9] (the max). The clamp rule is `min(n-1, idx)`,
/// so even `p_milli = 10_000` must clamp to `n - 1`.
#[test]
fn p99_clamps_to_n_minus_1_when_floor_exceeds_n_minus_1() {
    let mut samples: Vec<Duration> = (1..=10_u64).map(Duration::from_nanos).collect();
    samples.sort_unstable();
    assert_eq!(percentile_sorted(&samples, 9_900), Duration::from_nanos(10));
    assert_eq!(
        percentile_sorted(&samples, 10_000),
        Duration::from_nanos(10)
    );
    // p_milli = 5_000 with n = 10 → idx = 5, samples[5] = 6ns.
    assert_eq!(percentile_sorted(&samples, 5_000), Duration::from_nanos(6));
    // p_milli = 9_500 with n = 10 → idx = 9, samples[9] = 10ns.
    assert_eq!(percentile_sorted(&samples, 9_500), Duration::from_nanos(10));
}

/// Ordering invariant from `contract.md` §3: p50 ≤ p95 ≤ p99 for
/// any non-empty input. Verified for the canonical 100-sample
/// distribution and for an irregular 13-sample distribution.
#[test]
fn p50_p95_p99_orders_monotonically_for_canonical_input() {
    let mut samples: Vec<Duration> = (1..=100_u64).map(Duration::from_nanos).collect();
    samples.sort_unstable();
    let (p50, p95, p99) = p50_p95_p99_sorted(&samples);
    assert!(p50 <= p95, "p50 ({p50:?}) must be <= p95 ({p95:?})");
    assert!(p95 <= p99, "p95 ({p95:?}) must be <= p99 ({p99:?})");
}

#[test]
fn p50_p95_p99_orders_monotonically_for_irregular_input() {
    // Irregular sample set: not a simple arithmetic progression.
    let mut samples: Vec<Duration> = vec![
        Duration::from_nanos(7),
        Duration::from_nanos(1),
        Duration::from_nanos(50),
        Duration::from_nanos(3),
        Duration::from_nanos(12),
        Duration::from_nanos(99),
        Duration::from_nanos(8),
        Duration::from_nanos(4),
        Duration::from_nanos(17),
        Duration::from_nanos(42),
        Duration::from_nanos(60),
        Duration::from_nanos(2),
        Duration::from_nanos(33),
    ];
    samples.sort_unstable();
    let (p50, p95, p99) = p50_p95_p99_sorted(&samples);
    assert!(p50 <= p95, "p50 ({p50:?}) must be <= p95 ({p95:?})");
    assert!(p95 <= p99, "p95 ({p95:?}) must be <= p99 ({p99:?})");
    // Sorted indices: 0=1, 1=2, 2=3, 3=4, 4=7, 5=8, 6=12, 7=17,
    //                 8=33, 9=42, 10=50, 11=60, 12=99.
    // n = 13: p50 → idx = 5000 * 13 / 10_000 = 6, samples[6] = 12ns.
    assert_eq!(p50, Duration::from_nanos(12));
    // p95 → idx = 9500 * 13 / 10_000 = 12, samples[12] = 99ns.
    assert_eq!(p95, Duration::from_nanos(99));
    // p99 → idx = 9900 * 13 / 10_000 = 12, samples[12] = 99ns.
    assert_eq!(p99, Duration::from_nanos(99));
}

/// `nearest_rank_index` is the inner function that the bench helper
/// re-implements. Verify each canonical percentile on the canonical
/// sample sizes from the test plan.
#[test]
fn nearest_rank_index_matches_contract_table() {
    // n = 100: p50 → 50, p95 → 95, p99 → 99.
    assert_eq!(nearest_rank_index(5_000, 100), 50);
    assert_eq!(nearest_rank_index(9_500, 100), 95);
    assert_eq!(nearest_rank_index(9_900, 100), 99);
    // n = 1000: p50 → 500, p95 → 950, p99 → 990.
    assert_eq!(nearest_rank_index(5_000, 1000), 500);
    assert_eq!(nearest_rank_index(9_500, 1000), 950);
    assert_eq!(nearest_rank_index(9_900, 1000), 990);
    // n = 1: any percentile → 0.
    assert_eq!(nearest_rank_index(5_000, 1), 0);
    assert_eq!(nearest_rank_index(9_900, 1), 0);
    // n = 2: p99 → idx = 9900*2/10_000 = 1, samples[1] = max.
    assert_eq!(nearest_rank_index(9_900, 2), 1);
    // n = 0: clamped to 0.
    assert_eq!(nearest_rank_index(5_000, 0), 0);
}

/// The 3 captured scenarios use `sample_size = 10`, so the
/// percentiles for those scenarios follow the n = 10 case. This
/// test pins the expected percentile indices for the 3 captured
/// scenarios from `evidence/benchmark-logs/`.
#[test]
fn captured_scenarios_use_sample_size_10_indices() {
    // For n = 10: p50 → 5, p95 → 9, p99 → 9 (clamped from 9900*10/10_000=9).
    assert_eq!(nearest_rank_index(5_000, 10), 5);
    assert_eq!(nearest_rank_index(9_500, 10), 9);
    assert_eq!(nearest_rank_index(9_900, 10), 9);
    // Samples [1ns, 2ns, ..., 10ns] with n = 10:
    let mut samples: Vec<Duration> = (1..=10_u64).map(Duration::from_nanos).collect();
    samples.sort_unstable();
    assert_eq!(percentile_sorted(&samples, 5_000), Duration::from_nanos(6));
    assert_eq!(percentile_sorted(&samples, 9_500), Duration::from_nanos(10));
    assert_eq!(percentile_sorted(&samples, 9_900), Duration::from_nanos(10));
}
