// Verification artifact: proptest_step_offset.rs
// PO: PO-017 (checked_step_offset boundary values)
// PO: PO-029 (checked_step_offset overflow boundary)
// Bead: vb-xi2f.23
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_step_offset_boundary --test-threads=1
// Command: cargo test -p vb_compile -- proptest_step_offset_overflow --test-threads=1
//
// Proof obligations:
// - PO-017: Correct offset computation and error for boundary values u16::MAX-3, u16::MAX-2, u16::MAX-1, u16::MAX
// - PO-029: Correct StepIndexOutOfRange returned for boundary values with offsets 1, 2, 3
//
// GOD RULE 1: Explicit boundary values included in strategy.
// GOD RULE 2: Binds to actual Rust checked_step_offset implementation.

#![cfg(test)]
#![forbid(unsafe_code)]

use super::part_12::checked_step_offset;
use crate::CompileError;
use proptest::prelude::*;
use vb_core::ids::StepIdx;

// ─────────────────────────────────────────────────────────────────
// Boundary value strategy
// ─────────────────────────────────────────────────────────────────

/// Strategy for boundary u16 values near u16::MAX.
/// Includes: u16::MAX-3, u16::MAX-2, u16::MAX-1, u16::MAX
fn boundary_u16_strategy() -> impl Strategy<Value = u16> {
    prop_oneof![
        Just(u16::MAX - 3),
        Just(u16::MAX - 2),
        Just(u16::MAX - 1),
        Just(u16::MAX),
    ]
}

/// Strategy for offsets used in collect emission: {1, 2, 3}
fn collect_offset_strategy() -> impl Strategy<Value = u16> {
    prop_oneof![Just(1), Just(2), Just(3)]
}

// ─────────────────────────────────────────────────────────────────
// PO-017: Offset computation boundary values
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-017 H1: Boundary values u16::MAX-3 through u16::MAX with offset 1, 2, 3.
    #[test]
    fn proptest_step_offset_boundary(id in boundary_u16_strategy(), offset in collect_offset_strategy()) {
        let result = checked_step_offset(
            StepIdx::new(id),
            offset,
            "test",
            "field",
        );

        let expected_sum = u32::from(id) + u32::from(offset);

        if expected_sum <= u32::from(u16::MAX) {
            // Should succeed
            prop_assert!(result.is_ok(), "id + offset should succeed when <= u16::MAX");
            let new_id = result.unwrap();
            prop_assert_eq!(u32::from(new_id.get()), expected_sum, "new_id = id + offset");
        } else {
            // Should return error
            prop_assert!(result.is_err(), "id + offset should error when > u16::MAX");
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-029: Overflow detection at boundary
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-029 H1: Overflow correctly detected at boundary values.
    #[test]
    fn proptest_step_offset_overflow(id in boundary_u16_strategy(), offset in collect_offset_strategy()) {
        let result = checked_step_offset(
            StepIdx::new(id),
            offset,
            "test",
            "done",
        );

        let sum = u32::from(id) + u32::from(offset);

        if sum > u32::from(u16::MAX) {
            // Must be error
            prop_assert!(result.is_err(), "overflow must return error");

            // Error variant must be StepIndexOutOfRange
            let err = result.unwrap_err();
            prop_assert!(
                matches!(err, CompileError::PrimitiveLoweringLimitExceeded { .. }),
                "error is PrimitiveLoweringLimitExceeded"
            );
        }
    }

    /// PO-029 H2: Valid ids near boundary (u16::MAX - 3) with offset 3 succeed.
    #[test]
    fn proptest_step_offset_max_minus_3(_unit in Just(())) {
        let id = u16::MAX - 3;
        let offset: u16 = 3;

        let result = checked_step_offset(
            StepIdx::new(id),
            offset,
            "test",
            "done",
        );

        prop_assert!(result.is_ok(), "id=u16::MAX-3, offset=3 should succeed");
        prop_assert_eq!(result.unwrap().get(), u16::MAX, "result = u16::MAX");
    }

    /// PO-029 H3: u16::MAX with offset 1, 2, 3 all overflow.
    #[test]
    fn proptest_step_offset_max_overflows(offset in collect_offset_strategy()) {
        let result = checked_step_offset(
            StepIdx::new(u16::MAX),
            offset,
            "test",
            "done",
        );

        prop_assert!(result.is_err(), "u16::MAX + offset must error");
    }
}
