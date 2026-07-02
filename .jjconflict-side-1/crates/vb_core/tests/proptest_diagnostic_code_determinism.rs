//! Property test: diagnostic_code() is deterministic — same variant always
//! returns the same code regardless of field values.
//!
//! PO-018 / PS-018: Cold formatting determinism — diagnostic_code stability.
//!
//! Uses proptest strategies to generate random field values for CoreError
//! variants and verifies that the diagnostic_code() is invariant to the
//! specific field values (depends only on the variant discriminant).

use proptest::prelude::*;
use vb_core::DiagnosticCode;
use vb_core::errors::CoreError;
use vb_core::ids::{SlotIdx, StepIdx};

// Strategy: generate arbitrary SlotIdx and StepIdx values
fn arb_slot_idx() -> impl Strategy<Value = SlotIdx> {
    (0u16..1000u16).prop_map(SlotIdx::new)
}

fn arb_step_idx() -> impl Strategy<Value = StepIdx> {
    (0u16..1000u16).prop_map(StepIdx::new)
}

proptest! {
    #[test]
    fn invalid_program_counter_diagnostic_code_deterministic(
        step in arb_step_idx(),
        step2 in arb_step_idx(),
    ) {
        let err1 = CoreError::InvalidProgramCounter { step };
        let err2 = CoreError::InvalidProgramCounter { step: step2 };
        prop_assert_eq!(err1.diagnostic_code(), err2.diagnostic_code());
        prop_assert_eq!(err1.diagnostic_code(), DiagnosticCode::new(0x1001));
    }

    #[test]
    fn slot_out_of_bounds_diagnostic_code_deterministic(
        slot in arb_slot_idx(),
        slot2 in arb_slot_idx(),
    ) {
        let err1 = CoreError::SlotOutOfBounds { slot };
        let err2 = CoreError::SlotOutOfBounds { slot: slot2 };
        prop_assert_eq!(err1.diagnostic_code(), err2.diagnostic_code());
        prop_assert_eq!(err1.diagnostic_code(), DiagnosticCode::new(0x1011));
    }

    #[test]
    fn missing_output_slot_diagnostic_code_deterministic(
        step in arb_step_idx(),
        step2 in arb_step_idx(),
    ) {
        let err1 = CoreError::MissingOutputSlot { step };
        let err2 = CoreError::MissingOutputSlot { step: step2 };
        prop_assert_eq!(err1.diagnostic_code(), err2.diagnostic_code());
        prop_assert_eq!(err1.diagnostic_code(), DiagnosticCode::new(0x1305));
    }

    #[test]
    fn step_state_out_of_bounds_diagnostic_code_deterministic(
        step in arb_step_idx(),
        step2 in arb_step_idx(),
    ) {
        let err1 = CoreError::StepStateOutOfBounds { step };
        let err2 = CoreError::StepStateOutOfBounds { step: step2 };
        prop_assert_eq!(err1.diagnostic_code(), err2.diagnostic_code());
        prop_assert_eq!(err1.diagnostic_code(), DiagnosticCode::new(0x1306));
    }
}

#[test]
fn diagnostic_code_determinism_for_all_variants_with_field_variation() {
    // For each variant that carries fields, construct multiple instances
    // with different field values and verify code is invariant.

    // InvalidProgramCounter with different step values
    let a = CoreError::InvalidProgramCounter {
        step: StepIdx::new(0),
    };
    let b = CoreError::InvalidProgramCounter {
        step: StepIdx::new(999),
    };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // SlotOutOfBounds with different slot values
    let a = CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(1),
    };
    let b = CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(999),
    };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // TypeMismatch with different expected/found strings
    let a = CoreError::TypeMismatch {
        expected: "u64",
        found: "string",
    };
    let b = CoreError::TypeMismatch {
        expected: "bool",
        found: "f64",
    };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // ResourceLimitExceeded with different resource names
    let a = CoreError::ResourceLimitExceeded { resource: "cpu" };
    let b = CoreError::ResourceLimitExceeded { resource: "memory" };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // ExpressionStackOverflow with different max values
    let a = CoreError::ExpressionStackOverflow { max: 64 };
    let b = CoreError::ExpressionStackOverflow { max: 128 };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // UnsupportedAccessorTraversal with different segment/found values
    let a = CoreError::UnsupportedAccessorTraversal {
        segment: "field",
        found: "map",
    };
    let b = CoreError::UnsupportedAccessorTraversal {
        segment: "index",
        found: "string",
    };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // ObjectFieldNotFound with different field values
    let a = CoreError::ObjectFieldNotFound {
        field: vb_core::ids::SymbolId::new(0),
    };
    let b = CoreError::ObjectFieldNotFound {
        field: vb_core::ids::SymbolId::new(5),
    };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // ListIndexOutOfBounds with different index values
    let a = CoreError::ListIndexOutOfBounds { index: 0 };
    let b = CoreError::ListIndexOutOfBounds { index: 9999 };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // IterationLimitExceeded with different resource names
    let a = CoreError::IterationLimitExceeded { resource: "cpu" };
    let b = CoreError::IterationLimitExceeded { resource: "memory" };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // RepeatExhausted with different max values
    let a = CoreError::RepeatExhausted { max: 1 };
    let b = CoreError::RepeatExhausted { max: 100 };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // BudgetExceeded with different budget/limit values
    let a = CoreError::BudgetExceeded {
        budget: "cpu",
        limit: 100,
    };
    let b = CoreError::BudgetExceeded {
        budget: "memory",
        limit: 1000,
    };
    assert_eq!(a.diagnostic_code(), b.diagnostic_code());

    // Fieldless variants: same variant always same code (trivially true)
    assert_eq!(
        CoreError::DivisionByZero.diagnostic_code(),
        CoreError::DivisionByZero.diagnostic_code()
    );
    assert_eq!(
        CoreError::QueueFull.diagnostic_code(),
        CoreError::QueueFull.diagnostic_code()
    );
    assert_eq!(
        CoreError::AllocationFailed.diagnostic_code(),
        CoreError::AllocationFailed.diagnostic_code()
    );
}
