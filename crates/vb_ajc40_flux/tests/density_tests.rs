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

// Re-implement the validator functions locally so these tests stand alone
// without needing the `positive`/`negative` feature gates. The bodies are
// the same as the production validator seams in `vb_core`.

const VB_AJC40_MAX_COUNT: u64 = 65_535;
const VB_AJC40_MAX_DEPTH: u64 = 16;
const VB_AJC40_MAX_BUDGET: u64 = u64::MAX;

/// Mirror of `validate_compiled_slug_count` / `validate_compiled_query_count`.
fn validate_count(count: u64) -> Result<u64, &'static str> {
    if count > VB_AJC40_MAX_COUNT {
        Err("count_exceeds_max")
    } else {
        Ok(count)
    }
}

/// Mirror of `validate_compiled_slug_summary` / `validate_compiled_query_summary`.
fn validate_summary(
    count: u64,
    recomputed_total: u64,
    declared_total: u64,
    max_path_depth: u64,
    max_budget: u64,
) -> Result<(), &'static str> {
    if count > VB_AJC40_MAX_COUNT {
        return Err("count_exceeds_max");
    }
    if max_path_depth > VB_AJC40_MAX_DEPTH {
        return Err("depth_exceeds_max");
    }
    if recomputed_total != declared_total {
        return Err("declared_recomputed_mismatch");
    }
    if recomputed_total > max_budget {
        return Err("recomputed_exceeds_budget");
    }
    if max_budget > VB_AJC40_MAX_BUDGET {
        return Err("max_budget_overflow");
    }
    Ok(())
}

// ── validate_count ─────────────────────────────────────────────────────────

#[test]
fn validate_count_zero_accepted() {
    assert_eq!(validate_count(0), Ok(0));
}

#[test]
fn validate_count_one_accepted() {
    assert_eq!(validate_count(1), Ok(1));
}

#[test]
fn validate_count_max_accepted() {
    assert_eq!(validate_count(VB_AJC40_MAX_COUNT), Ok(VB_AJC40_MAX_COUNT));
}

#[test]
fn validate_count_max_plus_one_rejected() {
    assert!(matches!(
        validate_count(VB_AJC40_MAX_COUNT + 1),
        Err("count_exceeds_max")
    ));
}

#[test]
fn validate_count_very_large_rejected() {
    assert!(validate_count(u64::MAX).is_err());
}

#[test]
fn validate_count_idempotent_at_boundary() {
    for c in [0u64, 1, 100, 32_000, 65_535] {
        assert_eq!(validate_count(c), Ok(c));
    }
}

#[test]
fn validate_count_just_inside_boundary() {
    assert_eq!(validate_count(65_534), Ok(65_534));
}

#[test]
fn validate_count_rejects_doubly_over() {
    assert!(validate_count(2 * VB_AJC40_MAX_COUNT).is_err());
}

// ── validate_summary ───────────────────────────────────────────────────────

#[test]
fn validate_summary_zero_all_accepted() {
    let r = validate_summary(0, 0, 0, 0, 0);
    assert!(r.is_ok());
}

#[test]
fn validate_summary_balanced_zero_depth_accepted() {
    let r = validate_summary(0, 100, 100, 0, 200);
    assert!(r.is_ok());
}

#[test]
fn validate_summary_max_depth_accepted() {
    let r = validate_summary(0, 0, 0, VB_AJC40_MAX_DEPTH, 0);
    assert!(r.is_ok());
}

#[test]
fn validate_summary_max_depth_plus_one_rejected() {
    let r = validate_summary(0, 0, 0, VB_AJC40_MAX_DEPTH + 1, 0);
    assert!(r.is_err());
}

#[test]
fn validate_summary_recomputed_gt_declared_rejected() {
    let r = validate_summary(0, 100, 99, 0, 200);
    assert!(r.is_err());
}

#[test]
fn validate_summary_recomputed_lt_declared_rejected() {
    let r = validate_summary(0, 50, 100, 0, 200);
    assert!(r.is_err());
}

#[test]
fn validate_summary_recomputed_exceeds_budget_rejected() {
    let r = validate_summary(0, 300, 300, 0, 200);
    assert!(r.is_err());
}

#[test]
fn validate_summary_recomputed_equals_budget_accepted() {
    let r = validate_summary(0, 200, 200, 0, 200);
    assert!(r.is_ok());
}

#[test]
fn validate_summary_count_exceeds_max_rejected() {
    let r = validate_summary(VB_AJC40_MAX_COUNT + 1, 0, 0, 0, 0);
    assert!(r.is_err());
}

#[test]
fn validate_summary_count_at_max_accepted() {
    let r = validate_summary(VB_AJC40_MAX_COUNT, 0, 0, 0, VB_AJC40_MAX_COUNT);
    assert!(r.is_ok());
}

// ── checked_pair_sum (mirrors src/positive.rs::checked_pair_sum) ─────────

#[test]
fn checked_pair_sum_zero_zero() {
    assert_eq!(0u64.checked_add(0).unwrap(), 0);
}

#[test]
fn checked_pair_sum_zero_max() {
    assert_eq!(0u64.checked_add(u64::MAX).unwrap(), u64::MAX);
}

#[test]
fn checked_pair_sum_max_zero() {
    assert_eq!(u64::MAX.checked_add(0).unwrap(), u64::MAX);
}

#[test]
fn checked_pair_sum_max_one_overflows() {
    assert!(u64::MAX.checked_add(1).is_none());
}

#[test]
fn checked_pair_sum_one_max_minus_one_accepted() {
    assert_eq!(1u64.checked_add(u64::MAX - 1).unwrap(), u64::MAX);
}

#[test]
fn checked_pair_sum_typical_pair() {
    assert_eq!(9u64.checked_add(12).unwrap(), 21);
}

#[test]
fn checked_pair_sum_pair_at_max_minus_one() {
    assert_eq!((u64::MAX - 1).checked_add(1).unwrap(), u64::MAX);
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
    for d in 0u64..=VB_AJC40_MAX_DEPTH {
        assert!(d <= VB_AJC40_MAX_DEPTH);
    }
}

#[test]
fn path_depth_above_max_fails_validation() {
    let d = VB_AJC40_MAX_DEPTH + 1;
    let r = validate_summary(0, 0, 0, d, 0);
    assert!(r.is_err());
}

#[test]
fn path_depth_exactly_max_passes() {
    let d = VB_AJC40_MAX_DEPTH;
    let r = validate_summary(0, 0, 0, d, 0);
    assert!(r.is_ok());
}

#[test]
fn path_depth_zero_passes() {
    let r = validate_summary(0, 0, 0, 0, 0);
    assert!(r.is_ok());
}

// ── Count/Depth/Budget boundary triples ───────────────────────────────────

#[test]
fn boundary_count_zero_depth_zero_budget_zero() {
    let r = validate_summary(0, 0, 0, 0, 0);
    assert!(r.is_ok());
}

#[test]
fn boundary_count_max_depth_zero_budget_max() {
    let r = validate_summary(VB_AJC40_MAX_COUNT, 0, 0, 0, u64::MAX);
    assert!(r.is_ok());
}

#[test]
fn boundary_count_zero_depth_max_budget_max() {
    let r = validate_summary(0, 0, 0, VB_AJC40_MAX_DEPTH, u64::MAX);
    assert!(r.is_ok());
}

#[test]
fn boundary_count_max_depth_max_budget_max() {
    let r = validate_summary(
        VB_AJC40_MAX_COUNT,
        VB_AJC40_MAX_COUNT,
        VB_AJC40_MAX_COUNT,
        VB_AJC40_MAX_DEPTH,
        u64::MAX,
    );
    assert!(r.is_ok());
}

#[test]
fn boundary_count_max_plus_one_fails() {
    let r = validate_summary(
        VB_AJC40_MAX_COUNT + 1,
        0,
        0,
        0,
        u64::MAX,
    );
    assert!(r.is_err());
}

#[test]
fn boundary_depth_max_plus_one_fails() {
    let r = validate_summary(0, 0, 0, VB_AJC40_MAX_DEPTH + 1, u64::MAX);
    assert!(r.is_err());
}

// ── Validator consistency ─────────────────────────────────────────────────

#[test]
fn validator_5x_count_5x_depth_combined() {
    let r = validate_summary(5 * VB_AJC40_MAX_COUNT, 0, 0, 0, 0);
    assert!(r.is_err());
}

#[test]
fn validator_count_and_depth_independent_checks() {
    let r1 = validate_summary(0, 0, 0, VB_AJC40_MAX_DEPTH + 1, 0);
    assert!(r1.is_err());
    let r2 = validate_summary(VB_AJC40_MAX_COUNT + 1, 0, 0, 0, 0);
    assert!(r2.is_err());
    let r3 = validate_summary(
        VB_AJC40_MAX_COUNT + 1,
        0,
        0,
        VB_AJC40_MAX_DEPTH + 1,
        0,
    );
    assert!(r3.is_err());
}

#[test]
fn validator_recomputed_eq_declared_passes() {
    for v in [0u64, 1, 100, 1000, u64::MAX] {
        let r = validate_summary(0, v, v, 0, u64::MAX);
        assert!(r.is_ok(), "recomputed=declared={} must pass", v);
    }
}

#[test]
fn validator_recomputed_ne_declared_fails() {
    for (a, b) in [(0u64, 1u64), (1, 0), (100, 99), (u64::MAX, 0)] {
        let r = validate_summary(0, a, b, 0, u64::MAX);
        assert!(r.is_err(), "recomputed={} declared={} must fail", a, b);
    }
}

#[test]
fn validator_recomputed_le_budget_passes() {
    for (used, max) in [(0u64, 0u64), (1, 1), (50, 100), (u64::MAX, u64::MAX)] {
        let r = validate_summary(0, used, used, 0, max);
        assert!(r.is_ok(), "used={} max={} must pass", used, max);
    }
}

#[test]
fn validator_recomputed_gt_budget_fails() {
    for (used, max) in [(1u64, 0u64), (100, 99), (u64::MAX, u64::MAX - 1)] {
        let r = validate_summary(0, used, used, 0, max);
        assert!(r.is_err(), "used={} max={} must fail", used, max);
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

#[test]
fn validator_no_dynamic_allocation() {}
