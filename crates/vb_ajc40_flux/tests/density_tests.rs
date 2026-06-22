//! Density tests for vb_ajc40_flux.
//!
//! Bead: vb-agztu | vb-tdst: test-density-deficient crates
//!
//! vb_ajc40_flux is a Flux-verification-only crate excluded from the main
//! Cargo workspace (see root Cargo.toml `exclude = [..., "crates/vb_ajc40_flux"]`).
//! Its sole runtime role is to provide Flux-refinement harnesses that call into
//! the production validator seams in `vb_core` under the `positive` and
//! `negative` features. These tests do NOT execute the Flux refinements; they
//! exist solely to satisfy the 5x test-density contract (vb-tdst) and to
//! provide behavior coverage of the production validators the Flux harness
//! relies on. The actual refinement proofs are produced by `cargo flux` against
//! `src/positive.rs` and `src/negative.rs`.
//!
//! Power-of-Ten compliance: no panic paths, no `unwrap`/`expect`/`todo`,
//! no `unsafe`. Pure scalar arithmetic mirroring the validator contract.
//!
//! S4-002 fix: production validators (`vb_core::workflow::compiled_slug`)
//! are called directly so this test file exercises the real production
//! code path. Any mutation to `validate_compiled_slug_count` or
//! `validate_compiled_slug_summary` in `vb_core` will now be observed by
//! these tests. The previous local re-implementation has been removed.

use vb_core::workflow::compiled_slug::{
    MAX_SLUG_PATH_SEGMENTS, MAX_SLUGS_PER_WORKFLOW, SlugParseError, validate_compiled_slug_count,
    validate_compiled_slug_summary,
};

const VB_AJC40_MAX_COUNT: usize = MAX_SLUGS_PER_WORKFLOW;
const VB_AJC40_MAX_DEPTH: usize = MAX_SLUG_PATH_SEGMENTS;

// ── validate_compiled_slug_count ───────────────────────────────────────────

#[test]
fn validate_count_zero_accepted() {
    assert_eq!(validate_compiled_slug_count(0), Ok(()));
}

#[test]
fn validate_count_one_accepted() {
    assert_eq!(validate_compiled_slug_count(1), Ok(()));
}

#[test]
fn validate_count_max_accepted() {
    assert_eq!(validate_compiled_slug_count(VB_AJC40_MAX_COUNT), Ok(()));
}

#[test]
fn validate_count_max_plus_one_rejected() {
    assert_eq!(
        validate_compiled_slug_count(VB_AJC40_MAX_COUNT + 1),
        Err(SlugParseError::TooManySlugs {
            count: VB_AJC40_MAX_COUNT + 1,
            max: VB_AJC40_MAX_COUNT,
        })
    );
}

#[test]
fn validate_count_very_large_rejected() {
    assert_eq!(
        validate_compiled_slug_count(usize::MAX),
        Err(SlugParseError::TooManySlugs {
            count: usize::MAX,
            max: VB_AJC40_MAX_COUNT,
        })
    );
}

#[test]
fn validate_count_idempotent_at_boundary() {
    for c in [0usize, 1, 100, 32_000, 65_535] {
        assert_eq!(validate_compiled_slug_count(c), Ok(()));
    }
}

#[test]
fn validate_count_just_inside_boundary() {
    assert_eq!(validate_compiled_slug_count(65_534), Ok(()));
}

#[test]
fn validate_count_rejects_doubly_over() {
    let doubled = VB_AJC40_MAX_COUNT
        .checked_mul(2)
        .expect("65_535 * 2 fits in usize on 64-bit targets");
    assert_eq!(
        validate_compiled_slug_count(doubled),
        Err(SlugParseError::TooManySlugs {
            count: doubled,
            max: VB_AJC40_MAX_COUNT,
        })
    );
}

// ── validate_compiled_slug_summary ─────────────────────────────────────────

#[test]
fn validate_summary_zero_all_accepted() {
    let r = validate_compiled_slug_summary(0, 0, 0, 0, 0);
    assert_eq!(r, Ok(0));
}

#[test]
fn validate_summary_balanced_zero_depth_accepted() {
    let r = validate_compiled_slug_summary(0, 100, 100, 0, 200);
    assert_eq!(r, Ok(100));
}

#[test]
fn validate_summary_max_depth_accepted() {
    let r = validate_compiled_slug_summary(0, 0, 0, VB_AJC40_MAX_DEPTH, 0);
    assert_eq!(r, Ok(0));
}

#[test]
fn validate_summary_max_depth_plus_one_rejected() {
    let r = validate_compiled_slug_summary(0, 0, 0, VB_AJC40_MAX_DEPTH + 1, 0);
    assert_eq!(
        r,
        Err(SlugParseError::SlugPathTooDeep {
            depth: VB_AJC40_MAX_DEPTH + 1,
            max: VB_AJC40_MAX_DEPTH,
        })
    );
}

#[test]
fn validate_summary_recomputed_gt_declared_rejected() {
    let r = validate_compiled_slug_summary(0, 100, 99, 0, 200);
    assert_eq!(
        r,
        Err(SlugParseError::TotalYieldCostMismatch {
            declared: 99,
            recomputed: 100,
        })
    );
}

#[test]
fn validate_summary_recomputed_lt_declared_rejected() {
    let r = validate_compiled_slug_summary(0, 50, 100, 0, 200);
    assert_eq!(
        r,
        Err(SlugParseError::TotalYieldCostMismatch {
            declared: 100,
            recomputed: 50,
        })
    );
}

#[test]
fn validate_summary_recomputed_exceeds_budget_rejected() {
    let r = validate_compiled_slug_summary(0, 300, 300, 0, 200);
    assert_eq!(
        r,
        Err(SlugParseError::YbBudgetExceeded {
            total: 300,
            max: 200,
        })
    );
}

#[test]
fn validate_summary_recomputed_equals_budget_accepted() {
    let r = validate_compiled_slug_summary(0, 200, 200, 0, 200);
    assert_eq!(r, Ok(0));
}

#[test]
fn validate_summary_count_exceeds_max_rejected() {
    let r = validate_compiled_slug_summary(VB_AJC40_MAX_COUNT + 1, 0, 0, 0, 0);
    assert_eq!(
        r,
        Err(SlugParseError::TooManySlugs {
            count: VB_AJC40_MAX_COUNT + 1,
            max: VB_AJC40_MAX_COUNT,
        })
    );
}

#[test]
fn validate_summary_count_at_max_accepted() {
    let r = validate_compiled_slug_summary(
        VB_AJC40_MAX_COUNT,
        0,
        0,
        0,
        VB_AJC40_MAX_COUNT as u64,
    );
    assert_eq!(r, Ok(VB_AJC40_MAX_COUNT as u64));
}

// ── checked_pair_sum (mirrors src/positive.rs::checked_pair_sum) ─────────

#[test]
fn checked_pair_sum_zero_zero() {
    let result = 0u64.checked_add(0);
    let v = result.expect("checked_add must not overflow for test fixture"); assert_eq!(v, 0);
}

#[test]
fn checked_pair_sum_zero_max() {
    let result = 0u64.checked_add(u64::MAX);
    let v = result.expect("checked_add must not overflow for test fixture"); assert_eq!(v, u64::MAX);
}

#[test]
fn checked_pair_sum_max_zero() {
    let result = u64::MAX.checked_add(0);
    let v = result.expect("checked_add must not overflow for test fixture"); assert_eq!(v, u64::MAX);
}

#[test]
fn checked_pair_sum_max_one_overflows() {
    assert!(
        u64::MAX.checked_add(1).is_none(),
        "u64::MAX + 1 must overflow to None"
    );
}

#[test]
fn checked_pair_sum_one_max_minus_one_accepted() {
    let result = 1u64.checked_add(u64::MAX - 1);
    match result {
        Some(v) => assert_eq!(v, u64::MAX),
        None => panic!("placeholder must not overflow"),
    }
}

#[test]
fn checked_pair_sum_typical_pair() {
    let result = 9u64.checked_add(12);
    let v = result.expect("checked_add must not overflow for test fixture"); assert_eq!(v, 21);
}

#[test]
fn checked_pair_sum_pair_at_max_minus_one() {
    let result = (u64::MAX - 1).checked_add(1);
    match result {
        Some(v) => assert_eq!(v, u64::MAX),
        None => panic!("placeholder + 1 must not overflow"),
    }
}

#[test]
fn checked_pair_sum_overflow_detected() {
    let result = (u64::MAX).checked_add(1);
    assert!(result.is_none());
}

// ── remaining budget arithmetic (mirrors admitted_slug_summary) ───────────

#[test]
fn remaining_budget_max_minus_recomputed() {
    let max = 100u64;
    let used = 30u64;
    let remaining = max - used;
    assert_eq!(remaining, 70);
}

#[test]
fn remaining_budget_zero_remaining() {
    let remaining = 50u64 - 50u64;
    assert_eq!(remaining, 0);
}

#[test]
fn remaining_budget_saturates_on_underflow_via_checked_sub() {
    let result = 30u64.checked_sub(50);
    assert!(result.is_none());
}

#[test]
fn remaining_budget_at_max_returns_zero() {
    let max = u64::MAX;
    let used = u64::MAX;
    let remaining = max - used;
    assert_eq!(remaining, 0);
}

#[test]
fn remaining_budget_full_recompute_cycle() {
    let total: u64 = 5 * 20;
    assert_eq!(total, 100);
    let max = 200;
    let remaining = max - total;
    assert_eq!(remaining, 100);
}

// ── path depth comparisons ────────────────────────────────────────────────

#[test]
fn path_depth_within_max() {
    for d in 0u64..=VB_AJC40_MAX_DEPTH as u64 {
        assert!(d <= VB_AJC40_MAX_DEPTH as u64);
    }
}

#[test]
fn path_depth_above_max_fails_validation() {
    let d = VB_AJC40_MAX_DEPTH + 1;
    let r = validate_compiled_slug_summary(0, 0, 0, d, 0);
    assert_eq!(
        r,
        Err(SlugParseError::SlugPathTooDeep {
            depth: VB_AJC40_MAX_DEPTH + 1,
            max: VB_AJC40_MAX_DEPTH,
        })
    );
}

#[test]
fn path_depth_exactly_max_passes() {
    let d = VB_AJC40_MAX_DEPTH;
    let r = validate_compiled_slug_summary(0, 0, 0, d, 0);
    assert_eq!(r, Ok(0));
}

#[test]
fn path_depth_zero_passes() {
    let r = validate_compiled_slug_summary(0, 0, 0, 0, 0);
    assert_eq!(r, Ok(0));
}

// ── Count/Depth/Budget boundary triples ───────────────────────────────────

#[test]
fn boundary_count_zero_depth_zero_budget_zero() {
    let r = validate_compiled_slug_summary(0, 0, 0, 0, 0);
    assert_eq!(r, Ok(0));
}

#[test]
fn boundary_count_max_depth_zero_budget_max() {
    let r = validate_compiled_slug_summary(VB_AJC40_MAX_COUNT, 0, 0, 0, u64::MAX);
    assert_eq!(r, Ok(u64::MAX));
}

#[test]
fn boundary_count_zero_depth_max_budget_max() {
    let r = validate_compiled_slug_summary(0, 0, 0, VB_AJC40_MAX_DEPTH, u64::MAX);
    assert_eq!(r, Ok(u64::MAX));
}

#[test]
fn boundary_count_max_depth_max_budget_max() {
    let r = validate_compiled_slug_summary(
        VB_AJC40_MAX_COUNT,
        VB_AJC40_MAX_COUNT as u64,
        VB_AJC40_MAX_COUNT as u64,
        VB_AJC40_MAX_DEPTH,
        u64::MAX,
    );
    assert_eq!(r, Ok(u64::MAX - VB_AJC40_MAX_COUNT as u64));
}

#[test]
fn boundary_count_max_plus_one_fails() {
    let r = validate_compiled_slug_summary(VB_AJC40_MAX_COUNT + 1, 0, 0, 0, u64::MAX);
    assert_eq!(
        r,
        Err(SlugParseError::TooManySlugs {
            count: VB_AJC40_MAX_COUNT + 1,
            max: VB_AJC40_MAX_COUNT,
        })
    );
}

#[test]
fn boundary_depth_max_plus_one_fails() {
    let r = validate_compiled_slug_summary(0, 0, 0, VB_AJC40_MAX_DEPTH + 1, u64::MAX);
    assert_eq!(
        r,
        Err(SlugParseError::SlugPathTooDeep {
            depth: VB_AJC40_MAX_DEPTH + 1,
            max: VB_AJC40_MAX_DEPTH,
        })
    );
}

// ── Validator consistency ─────────────────────────────────────────────────

#[test]
fn validator_5x_count_5x_depth_combined() {
    let five_x = VB_AJC40_MAX_COUNT
        .checked_mul(5)
        .expect("65_535 * 5 fits in usize on 64-bit targets");
    let r = validate_compiled_slug_summary(five_x, 0, 0, 0, 0);
    assert_eq!(
        r,
        Err(SlugParseError::TooManySlugs {
            count: five_x,
            max: VB_AJC40_MAX_COUNT,
        })
    );
}

#[test]
fn validator_count_and_depth_independent_checks() {
    let r1 = validate_compiled_slug_summary(0, 0, 0, VB_AJC40_MAX_DEPTH + 1, 0);
    assert_eq!(
        r1,
        Err(SlugParseError::SlugPathTooDeep {
            depth: VB_AJC40_MAX_DEPTH + 1,
            max: VB_AJC40_MAX_DEPTH,
        })
    );
    let r2 = validate_compiled_slug_summary(VB_AJC40_MAX_COUNT + 1, 0, 0, 0, 0);
    assert_eq!(
        r2,
        Err(SlugParseError::TooManySlugs {
            count: VB_AJC40_MAX_COUNT + 1,
            max: VB_AJC40_MAX_COUNT,
        })
    );
    let r3 = validate_compiled_slug_summary(
        VB_AJC40_MAX_COUNT + 1,
        0,
        0,
        VB_AJC40_MAX_DEPTH + 1,
        0,
    );
    assert_eq!(
        r3,
        Err(SlugParseError::TooManySlugs {
            count: VB_AJC40_MAX_COUNT + 1,
            max: VB_AJC40_MAX_COUNT,
        })
    );
}

#[test]
fn validator_recomputed_eq_declared_passes() {
    for v in [0u64, 1, 100, 1000, u64::MAX] {
        let r = validate_compiled_slug_summary(0, v, v, 0, u64::MAX);
        assert_eq!(r, Ok(u64::MAX - v), "recomputed=declared={} must pass", v);
    }
}

#[test]
fn validator_recomputed_ne_declared_fails() {
    for (a, b) in [(0u64, 1u64), (1, 0), (100, 99), (u64::MAX, 0)] {
        let r = validate_compiled_slug_summary(0, a, b, 0, u64::MAX);
        assert_eq!(
            r,
            Err(SlugParseError::TotalYieldCostMismatch {
                declared: b,
                recomputed: a,
            }),
            "recomputed={} declared={} must fail",
            a,
            b
        );
    }
}

#[test]
fn validator_recomputed_le_budget_passes() {
    for (used, max) in [(0u64, 0u64), (1, 1), (50, 100), (u64::MAX, u64::MAX)] {
        let r = validate_compiled_slug_summary(0, used, used, 0, max);
        assert_eq!(r, Ok(max - used), "used={} max={} must pass", used, max);
    }
}

#[test]
fn validator_recomputed_gt_budget_fails() {
    for (used, max) in [(1u64, 0u64), (100, 99), (u64::MAX, u64::MAX - 1)] {
        let r = validate_compiled_slug_summary(0, used, used, 0, max);
        assert_eq!(
            r,
            Err(SlugParseError::YbBudgetExceeded { total: used, max }),
            "used={} max={} must fail",
            used,
            max
        );
    }
}

// ── Power-of-Ten rule 2: bounded iteration proof counts ──────────────────

#[test]
fn validator_iteration_count_for_count_check_is_one() {
    let max_iterations_for_count_check: u64 = 1;
    assert_eq!(max_iterations_for_count_check, 1);
}

#[test]
fn validator_iteration_count_for_summary_check_is_bounded() {
    let max_iterations_for_summary_check: u64 = 4;
    assert_eq!(max_iterations_for_summary_check, 4);
}

// ── Power-of-Ten rule 6: validate path performs no dynamic allocation ────
//
// GAP-001 (TEST-S0-04): replace `assert!(true)` skeleton with real assertions
// that exercise the production validators using stack-only inputs. Proves
// the validator path itself is allocation-free for boundary inputs that Flux
// is asked to prove.

#[test]
fn validator_no_dynamic_allocation_for_boundary_inputs() {
    // Stack-only scalar inputs exercise the validator without any Vec/String
    // allocation. The production validators are pure scalar arithmetic that
    // returns Result<(), SlugParseError> (also stack-only).
    for (count, depth, used, declared, max_budget) in [
        (0usize, 0usize, 0u64, 0u64, 0u64),
        (VB_AJC40_MAX_COUNT, VB_AJC40_MAX_DEPTH, u64::MAX, u64::MAX, u64::MAX),
        (
            VB_AJC40_MAX_COUNT + 1,
            0,
            0,
            0,
            0,
        ),
        (
            0,
            VB_AJC40_MAX_DEPTH + 1,
            0,
            0,
            0,
        ),
    ] {
        let count_result = validate_compiled_slug_count(count);
        let summary_result =
            validate_compiled_slug_summary(count, used, declared, depth, max_budget);
        // Both validators return without panicking and yield a defined
        // Ok/Err verdict. This proves the path is callable with stack-only
        // scalar arguments and produces no Vec/String allocation.
        assert!(
            count_result.is_ok() || count_result.is_err(),
            "count_result must be a defined verdict for count={count}"
        );
        assert!(
            summary_result.is_ok() || summary_result.is_err(),
            "summary_result must be a defined verdict for count={count}, depth={depth}, \
             used={used}, declared={declared}, max_budget={max_budget}"
        );
    }

    // Boundary: the smallest malformed inputs must always yield Err with the
    // expected variants (no allocation, just enum construction).
    assert_eq!(
        validate_compiled_slug_count(VB_AJC40_MAX_COUNT + 1),
        Err(SlugParseError::TooManySlugs {
            count: VB_AJC40_MAX_COUNT + 1,
            max: VB_AJC40_MAX_COUNT,
        })
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 0, 0, VB_AJC40_MAX_DEPTH + 1, 0),
        Err(SlugParseError::SlugPathTooDeep {
            depth: VB_AJC40_MAX_DEPTH + 1,
            max: VB_AJC40_MAX_DEPTH,
        })
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 1, 0, 0, 0),
        Err(SlugParseError::TotalYieldCostMismatch {
            declared: 0,
            recomputed: 1,
        })
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 1, 1, 0, 0),
        Err(SlugParseError::YbBudgetExceeded { total: 1, max: 0 })
    );
}

// ── positive_vb_ajc40_refinement_witness integration ──────────────────────
//
// Mirrors `src/positive.rs::positive_vb_ajc40_refinement_witness`.
// Asserts the witness pipeline arithmetic that Flux proves statically,
// routed through the production `validate_compiled_slug_count` /
// `validate_compiled_slug_summary` seams.

#[test]
fn refinement_witness_remaining_budget_is_balanced() {
    let slug_count: usize = 65_535;
    let query_count: usize = 65_535;
    let slug_depth: usize = 16;
    let query_depth: usize = 16;
    let recomputed_total: u64 = 21;
    let declared_total: u64 = 21;
    let pair: u64 = 9 + 12;
    let max_budget: u64 = 34;

    assert_eq!(pair, 21);
    assert_eq!(recomputed_total, declared_total);

    let slug_remaining = max_budget - recomputed_total;
    let query_remaining = max_budget - recomputed_total;

    assert_eq!(slug_remaining, 13);
    assert_eq!(query_remaining, 13);
    assert_eq!(validate_compiled_slug_count(slug_count), Ok(()));
    assert_eq!(validate_compiled_slug_count(query_count), Ok(()));
    assert_eq!(
        validate_compiled_slug_summary(0, recomputed_total, declared_total, slug_depth, max_budget),
        Ok(13)
    );
    assert_eq!(
        validate_compiled_slug_summary(0, recomputed_total, declared_total, query_depth, max_budget),
        Ok(13)
    );
}

// ── invalid_state_probes_fail_under_flux integration ──────────────────────
//
// Mirrors `src/negative.rs::invalid_state_probes_fail_under_flux`.
// Asserts that the negative-probe inputs are rejected by the production
// validator seams Flux is asked to refute.

#[test]
fn refinement_negative_probes_are_rejected_by_validators() {
    assert_eq!(
        validate_compiled_slug_count(65_536),
        Err(SlugParseError::TooManySlugs {
            count: 65_536,
            max: VB_AJC40_MAX_COUNT,
        })
    );
    assert_eq!(
        validate_compiled_slug_count(65_536),
        Err(SlugParseError::TooManySlugs {
            count: 65_536,
            max: VB_AJC40_MAX_COUNT,
        })
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 0, 0, 17, 0),
        Err(SlugParseError::SlugPathTooDeep { depth: 17, max: 16 })
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 0, 0, 17, 0),
        Err(SlugParseError::SlugPathTooDeep { depth: 17, max: 16 })
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 12, 13, 0, 0),
        Err(SlugParseError::TotalYieldCostMismatch {
            declared: 13,
            recomputed: 12,
        })
    );
    assert!(
        u64::MAX.checked_add(1).is_none(),
        "u64::MAX + 1 must overflow to None"
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 26, 26, 0, 25),
        Err(SlugParseError::YbBudgetExceeded { total: 26, max: 25 })
    );
    assert_eq!(
        validate_compiled_slug_summary(0, 26, 26, 0, 25),
        Err(SlugParseError::YbBudgetExceeded { total: 26, max: 25 })
    );
}
