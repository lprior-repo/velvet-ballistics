//! Proptest property suite for `vb_proof_kernels::resource_budget`.
//!
//! Bead: vb-z280t — closure of the resource_budget bridge between the
//! Verus spec (`spec_sat_mul_u64`, `spec_loop_mul_field_at`,
//! `lemma_loop_mul_saturated_eq_production`) and the production
//! `cargo_kernel::Budget::loop_mul` saturating arithmetic.
//!
//! Each property in this file mirrors a Verus spec/lemma from
//! `crates/vb_proof_kernels/src/resource_budget/spec.rs`:
//!
//! - spec_sat_mul_u64 (`u64_max_int`):
//!   `result = if a*b fits in u64 then a*b else u64::MAX`.
//! - spec_loop_mul_field_at: index into 12-field Budget product.
//! - lemma_loop_mul_saturated_eq_production:
//!   For every field index 0..12, `spec_loop_mul_field_at(body, iter, i) >= 0`.
//! - spec_loop_mul zero/one-iteration lemmas:
//!   `spec_loop_mul(body, 0) == empty` and `spec_loop_mul(body, 1) == body`.
//!
//! Production binding: `loop_mul` here is the public
//! `cargo_kernel::Budget::loop_mul` (which uses `u64::saturating_mul`).
//! Each property exercises the production exec fn and asserts the
//! saturating semantics match the Verus spec on both the
//! non-overflowing and overflowing regimes.

#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_proof_kernels::resource_budget::Budget;

/// `u64::MAX` as a literal `u64`. Matches the Verus `u64_max_int()` spec.
const U64_MAX: u64 = 18_446_744_073_709_551_615;

/// Strategy: a u64 with bias toward boundary values that exercise
/// saturating arithmetic (0, 1, u64::MAX/2, u64::MAX).
fn arb_u64_biased() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(0_u64),
        Just(1_u64),
        Just(2_u64),
        Just(u64::MAX),
        Just(u64::MAX / 2),
        Just(u64::MAX / 4),
        0u64..u64::MAX,
    ]
}

/// Strategy: any 12-field `Budget` with bias toward zero and u64::MAX
/// (boundary cases for saturating arithmetic).
fn arb_budget() -> impl Strategy<Value = Budget> {
    prop_oneof![
        Just(Budget::new()),
        (
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
            arb_u64_biased(),
        )
            .prop_map(|(a, b, c, d, e, f, g, h, i, j, k, l)| Budget {
                steps: a,
                actions: b,
                parallel: c,
                retries: d,
                gather_pages: e,
                gather_items: f,
                for_each_iters: g,
                together_branches: h,
                repeat_attempts: i,
                run_time_secs: j,
                result_bytes: k,
                slots_written: l,
            })
    ]
}

/// Reference `spec_sat_mul_u64`: `result = if a*b fits in u64 then a*b else u64::MAX`.
fn spec_sat_mul_u64(a: u64, b: u64) -> u64 {
    a.checked_mul(b).map_or(u64::MAX, |v| v)
}

// ----------------------------------------------------------------------------
// spec_sat_mul_u64 (u64_max_int): saturating multiplication reference.
// ----------------------------------------------------------------------------

proptest! {
    /// `spec_sat_mul_u64` equals `u64::saturating_mul` for all u64 inputs.
    #[test]
    fn spec_sat_mul_u64_matches_saturating_mul(a in arb_u64_biased(), b in arb_u64_biased()) {
        let spec = spec_sat_mul_u64(a, b);
        let prod = a.saturating_mul(b);
        prop_assert_eq!(spec, prod, "spec_sat_mul_u64 must equal u64::saturating_mul");
    }

    /// `spec_sat_mul_u64` never overflows: result fits in u64.
    /// (u64 fields are always <= u64::MAX by type, so this is a tautology
    /// checked at runtime via `prop_assert_eq!` against the type's upper bound.)
    #[test]
    fn spec_sat_mul_u64_bounded(a in arb_u64_biased(), b in arb_u64_biased()) {
        let result = spec_sat_mul_u64(a, b);
        // The saturation property is captured by `saturating_mul` equivalence:
        // if a*b would overflow, `spec_sat_mul_u64` clamps to u64::MAX,
        // which is the maximum representable value of the u64 type.
        prop_assert_eq!(result, a.saturating_mul(b));
    }

    /// Boundary: spec_sat_mul_u64(u64::MAX, 1) == u64::MAX (no overflow path).
    #[test]
    fn spec_sat_mul_u64_max_times_one(_unit in Just(())) {
        prop_assert_eq!(spec_sat_mul_u64(u64::MAX, 1), u64::MAX);
        prop_assert_eq!(spec_sat_mul_u64(1, u64::MAX), u64::MAX);
    }

    /// Boundary: spec_sat_mul_u64(0, x) == 0 (zero identity).
    #[test]
    fn spec_sat_mul_u64_zero_identity(x in arb_u64_biased()) {
        prop_assert_eq!(spec_sat_mul_u64(0, x), 0);
        prop_assert_eq!(spec_sat_mul_u64(x, 0), 0);
    }

    /// Boundary: spec_sat_mul_u64(u64::MAX, u64::MAX) saturates to u64::MAX.
    #[test]
    fn spec_sat_mul_u64_max_saturates(_unit in Just(())) {
        prop_assert_eq!(spec_sat_mul_u64(u64::MAX, u64::MAX), u64::MAX);
    }
}

// ----------------------------------------------------------------------------
// lemma_loop_mul_saturated_eq_production: loop_mul field-wise saturating
// multiplication matches `spec_sat_mul_u64` for every Budget field.
// ----------------------------------------------------------------------------

proptest! {
    /// Production `Budget::loop_mul(iter)` equals field-wise
    /// `spec_sat_mul_u64(body.field_i, iter)` for every Budget field.
    /// This is the production binding of
    /// `lemma_loop_mul_saturated_eq_production`.
    #[test]
    fn loop_mul_fieldwise_equals_spec_sat_mul_u64(body in arb_budget(), iter in arb_u64_biased()) {
        let mut b = body.clone();
        b.loop_mul(iter);
        prop_assert_eq!(b.steps, spec_sat_mul_u64(body.steps, iter));
        prop_assert_eq!(b.actions, spec_sat_mul_u64(body.actions, iter));
        prop_assert_eq!(b.parallel, spec_sat_mul_u64(body.parallel, iter));
        prop_assert_eq!(b.retries, spec_sat_mul_u64(body.retries, iter));
        prop_assert_eq!(b.gather_pages, spec_sat_mul_u64(body.gather_pages, iter));
        prop_assert_eq!(b.gather_items, spec_sat_mul_u64(body.gather_items, iter));
        prop_assert_eq!(b.for_each_iters, spec_sat_mul_u64(body.for_each_iters, iter));
        prop_assert_eq!(b.together_branches, spec_sat_mul_u64(body.together_branches, iter));
        prop_assert_eq!(b.repeat_attempts, spec_sat_mul_u64(body.repeat_attempts, iter));
        prop_assert_eq!(b.run_time_secs, spec_sat_mul_u64(body.run_time_secs, iter));
        prop_assert_eq!(b.result_bytes, spec_sat_mul_u64(body.result_bytes, iter));
        prop_assert_eq!(b.slots_written, spec_sat_mul_u64(body.slots_written, iter));
    }

    /// Every loop_mul result field equals the spec_sat_mul_u64 reference.
    /// This implicitly verifies the saturation property: when the
    /// multiplication would overflow u64, the production loop_mul saturates
    /// to u64::MAX (the maximum representable value of the field type),
    /// matching the Verus spec.
    #[test]
    fn loop_mul_never_overflows(body in arb_budget(), iter in arb_u64_biased()) {
        let original = body.clone();
        let mut b = body;
        b.loop_mul(iter);
        // Each field must equal the saturating multiplication reference.
        // u64 fields are always <= u64::MAX by type, so the saturation
        // property is captured by the equality with `spec_sat_mul_u64`.
        prop_assert_eq!(b.steps, spec_sat_mul_u64(original.steps, iter));
        prop_assert_eq!(b.actions, spec_sat_mul_u64(original.actions, iter));
        prop_assert_eq!(b.parallel, spec_sat_mul_u64(original.parallel, iter));
        prop_assert_eq!(b.retries, spec_sat_mul_u64(original.retries, iter));
        prop_assert_eq!(b.gather_pages, spec_sat_mul_u64(original.gather_pages, iter));
        prop_assert_eq!(b.gather_items, spec_sat_mul_u64(original.gather_items, iter));
        prop_assert_eq!(b.for_each_iters, spec_sat_mul_u64(original.for_each_iters, iter));
        prop_assert_eq!(b.together_branches, spec_sat_mul_u64(original.together_branches, iter));
        prop_assert_eq!(b.repeat_attempts, spec_sat_mul_u64(original.repeat_attempts, iter));
        prop_assert_eq!(b.run_time_secs, spec_sat_mul_u64(original.run_time_secs, iter));
        prop_assert_eq!(b.result_bytes, spec_sat_mul_u64(original.result_bytes, iter));
        prop_assert_eq!(b.slots_written, spec_sat_mul_u64(original.slots_written, iter));
    }

    /// loop_mul(0) leaves a zero budget unchanged (since 0*x == 0).
    #[test]
    fn loop_mul_zero_iterations_zero_budget(_unit in Just(())) {
        let mut b = Budget::new();
        b.loop_mul(0);
        prop_assert_eq!(b.steps, 0);
        prop_assert_eq!(b.actions, 0);
        prop_assert_eq!(b.parallel, 0);
        prop_assert_eq!(b.retries, 0);
        prop_assert_eq!(b.gather_pages, 0);
        prop_assert_eq!(b.gather_items, 0);
        prop_assert_eq!(b.for_each_iters, 0);
        prop_assert_eq!(b.together_branches, 0);
        prop_assert_eq!(b.repeat_attempts, 0);
        prop_assert_eq!(b.run_time_secs, 0);
        prop_assert_eq!(b.result_bytes, 0);
        prop_assert_eq!(b.slots_written, 0);
    }

    /// loop_mul(1) is the identity (since x*1 == x).
    #[test]
    fn loop_mul_one_iteration_identity(body in arb_budget()) {
        let mut b = body.clone();
        b.loop_mul(1);
        prop_assert_eq!(b.steps, body.steps);
        prop_assert_eq!(b.actions, body.actions);
        prop_assert_eq!(b.parallel, body.parallel);
        prop_assert_eq!(b.retries, body.retries);
        prop_assert_eq!(b.gather_pages, body.gather_pages);
        prop_assert_eq!(b.gather_items, body.gather_items);
        prop_assert_eq!(b.for_each_iters, body.for_each_iters);
        prop_assert_eq!(b.together_branches, body.together_branches);
        prop_assert_eq!(b.repeat_attempts, body.repeat_attempts);
        prop_assert_eq!(b.run_time_secs, body.run_time_secs);
        prop_assert_eq!(b.result_bytes, body.result_bytes);
        prop_assert_eq!(b.slots_written, body.slots_written);
    }

    /// Boundary: body.steps = u64::MAX, iter = 2 → steps saturates to u64::MAX.
    #[test]
    fn loop_mul_saturates_at_max_boundary(_unit in Just(())) {
        let mut b = Budget::new();
        b.steps = u64::MAX;
        b.loop_mul(2);
        prop_assert_eq!(b.steps, u64::MAX);
    }

    /// Boundary: body.steps = u64::MAX, iter = u64::MAX → steps saturates.
    #[test]
    fn loop_mul_saturates_at_max_squared(_unit in Just(())) {
        let mut b = Budget::new();
        b.steps = u64::MAX;
        b.loop_mul(u64::MAX);
        prop_assert_eq!(b.steps, u64::MAX);
    }

    /// Monotonicity: if body1 <= body2 field-wise and iter > 0, then
    /// loop_mul(body1, iter) <= loop_mul(body2, iter) field-wise.
    /// This mirrors the monotonicity property implicit in
    /// `spec_loop_mul`'s mathematical monotonicity.
    #[test]
    fn loop_mul_monotone_fieldwise(
        a1 in arb_u64_biased(),
        a2 in arb_u64_biased(),
        iter in 1u64..1024,
    ) {
        let (lo, hi) = if a1 <= a2 { (a1, a2) } else { (a2, a1) };
        let mut b_lo = Budget::new();
        b_lo.steps = lo;
        b_lo.loop_mul(iter);
        let mut b_hi = Budget::new();
        b_hi.steps = hi;
        b_hi.loop_mul(iter);
        prop_assert!(b_lo.steps <= b_hi.steps);
    }
}

// ----------------------------------------------------------------------------
// spec_loop_mul_field_at: 12-field Budget product access.
// ----------------------------------------------------------------------------

proptest! {
    /// Field index 0 maps to `steps`. Mirrors the first branch of
    /// `spec_loop_mul_field_at`.
    #[test]
    fn spec_loop_mul_field_at_zero_is_steps(body in arb_budget(), iter in arb_u64_biased()) {
        let mut b = body.clone();
        b.loop_mul(iter);
        let expected = spec_sat_mul_u64(body.steps, iter);
        prop_assert_eq!(b.steps, expected);
    }

    /// All 12 fields are addressed via the same `spec_sat_mul_u64` formula
    /// after `loop_mul(iter)`. Index sweep over 0..12.
    #[test]
    fn spec_loop_mul_field_at_all_twelve_indices(body in arb_budget(), iter in arb_u64_biased()) {
        let original = body.clone();
        let mut b = body;
        b.loop_mul(iter);

        let fields = [
            (original.steps, b.steps),
            (original.actions, b.actions),
            (original.parallel, b.parallel),
            (original.retries, b.retries),
            (original.gather_pages, b.gather_pages),
            (original.gather_items, b.gather_items),
            (original.for_each_iters, b.for_each_iters),
            (original.together_branches, b.together_branches),
            (original.repeat_attempts, b.repeat_attempts),
            (original.run_time_secs, b.run_time_secs),
            (original.result_bytes, b.result_bytes),
            (original.slots_written, b.slots_written),
        ];

        for (i, (pre, post)) in fields.iter().enumerate() {
            let expected = spec_sat_mul_u64(*pre, iter);
            prop_assert_eq!(
                *post,
                expected,
                "field index {} mismatch: expected spec_sat_mul_u64({}, {}) = {}, got {}",
                i,
                pre,
                iter,
                expected,
                post
            );
        }
    }
}
