//! Property test: runtime_code() is deterministic — same variant always
//! returns the same &'static str regardless of field values.
//!
//! PO-019 / PS-019: Cold formatting determinism — runtime_code stability.
//!
//! Invariants:
//!   - For variants with runtime_code() mappings, the returned str is invariant
//!     to field values.
//!   - For variants without mappings, runtime_code() always returns None.

use vb_core::errors::CoreError;
use vb_core::ids::{SlotIdx, StepIdx};

#[test]
fn runtime_code_determinism_const_out_of_bounds() {
    let a = CoreError::ConstOutOfBounds {
        index: vb_core::ids::ConstIdx::new(0),
    };
    let b = CoreError::ConstOutOfBounds {
        index: vb_core::ids::ConstIdx::new(999),
    };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::CONST_OUT_OF_BOUNDS_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_type_mismatch() {
    let a = CoreError::TypeMismatch {
        expected: "u64",
        found: "string",
    };
    let b = CoreError::TypeMismatch {
        expected: "bool",
        found: "f64",
    };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::INPUT_TYPE_MISMATCH_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_non_bool_condition() {
    let a = CoreError::NonBoolCondition {
        slot: SlotIdx::new(0),
    };
    let b = CoreError::NonBoolCondition {
        slot: SlotIdx::new(999),
    };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::INPUT_TYPE_MISMATCH_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_missing_output_slot() {
    let a = CoreError::MissingOutputSlot {
        step: StepIdx::new(0),
    };
    let b = CoreError::MissingOutputSlot {
        step: StepIdx::new(999),
    };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::MISSING_OUTPUT_SLOT_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_step_state_out_of_bounds() {
    let a = CoreError::StepStateOutOfBounds {
        step: StepIdx::new(0),
    };
    let b = CoreError::StepStateOutOfBounds {
        step: StepIdx::new(999),
    };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_expression_stack_overflow() {
    let a = CoreError::ExpressionStackOverflow { max: 64 };
    let b = CoreError::ExpressionStackOverflow { max: 128 };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::EXPRESSION_STACK_OVERFLOW_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_expression_stack_underflow() {
    let a = CoreError::ExpressionStackUnderflow;
    let b = CoreError::ExpressionStackUnderflow;
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::EXPRESSION_STACK_UNDERFLOW_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_invalid_compiled_workflow() {
    let a = CoreError::InvalidCompiledWorkflow { reason: "bad" };
    let b = CoreError::InvalidCompiledWorkflow { reason: "worse" };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::INVALID_COMPILED_WORKFLOW_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_internal_invariant() {
    let a = CoreError::InternalInvariantViolation { reason: "test" };
    let b = CoreError::InternalInvariantViolation { reason: "other" };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::INTERNAL_INVARIANT_VIOLATION_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_unsupported_primitive() {
    let a = CoreError::UnsupportedPrimitive { primitive: "op" };
    let b = CoreError::UnsupportedPrimitive { primitive: "wait" };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::UNSUPPORTED_PRIMITIVE_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_queue_full() {
    assert_eq!(
        CoreError::QueueFull.runtime_code(),
        CoreError::QueueFull.runtime_code()
    );
    assert_eq!(
        CoreError::QueueFull.runtime_code(),
        Some(CoreError::QUEUE_FULL_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_repeat_exhausted() {
    let a = CoreError::RepeatExhausted { max: 1 };
    let b = CoreError::RepeatExhausted { max: 100 };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::REPEAT_LIMIT_REACHED_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_collect_limits() {
    // CollectPageLimitExceeded, CollectItemLimitExceeded, CollectTimeLimitExceeded
    // all map to COLLECT_LIMIT_REACHED_RUNTIME_CODE
    let rt_code = Some(CoreError::COLLECT_LIMIT_REACHED_RUNTIME_CODE);
    assert_eq!(CoreError::CollectPageLimitExceeded.runtime_code(), rt_code);
    assert_eq!(CoreError::CollectItemLimitExceeded.runtime_code(), rt_code);
    assert_eq!(CoreError::CollectTimeLimitExceeded.runtime_code(), rt_code);
}

#[test]
fn runtime_code_determinism_budget_exceeded() {
    let a = CoreError::BudgetExceeded {
        budget: "cpu",
        limit: 100,
    };
    let b = CoreError::BudgetExceeded {
        budget: "memory",
        limit: 1000,
    };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::BUDGET_EXCEEDED_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_determinism_capability_denied() {
    let a = CoreError::CapabilityDenied {
        action: vb_core::ids::ActionId::new(1),
        required: vb_core::capability::Capability::new(
            Box::from("test"),
            vb_core::ids::ActionId::new(1),
        ),
        granted: vb_core::capability::CapabilitySet::empty(),
    };
    let b = CoreError::CapabilityDenied {
        action: vb_core::ids::ActionId::new(2),
        required: vb_core::capability::Capability::new(
            Box::from("other"),
            vb_core::ids::ActionId::new(2),
        ),
        granted: vb_core::capability::CapabilitySet::empty(),
    };
    assert_eq!(a.runtime_code(), b.runtime_code());
    assert_eq!(
        a.runtime_code(),
        Some(CoreError::CAPABILITY_DENIED_RUNTIME_CODE)
    );
}

#[test]
fn runtime_code_none_for_unmapped_variants() {
    // These variants have no runtime_code() mapping
    assert_eq!(
        CoreError::InvalidProgramCounter {
            step: StepIdx::new(1)
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::MissingNextStep {
            step: StepIdx::new(1)
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(1)
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::SlotUninitialized {
            slot: SlotIdx::new(1)
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::ExprOutOfBounds {
            expr: vb_core::ids::ExprIdx::new(1)
        }
        .runtime_code(),
        None
    );
    assert_eq!(CoreError::NonFiniteNumber.runtime_code(), None);
    assert_eq!(CoreError::DivisionByZero.runtime_code(), None);
    assert_eq!(CoreError::StepBudgetExhausted.runtime_code(), None);
    assert_eq!(CoreError::StepCounterOverflow.runtime_code(), None);
    assert_eq!(
        CoreError::ResourceLimitExceeded { resource: "cpu" }.runtime_code(),
        None
    );
    assert_eq!(CoreError::AllocationFailed.runtime_code(), None);
    assert_eq!(
        CoreError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "map",
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::ObjectFieldNotFound {
            field: vb_core::ids::SymbolId::new(0),
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::ListIndexOutOfBounds { index: 0 }.runtime_code(),
        None
    );
    assert_eq!(
        CoreError::SymbolOutOfBounds {
            symbol: vb_core::ids::SymbolId::new(0),
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::ListOutOfBounds {
            list: vb_core::ids::ListId::new(0),
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::ObjectOutOfBounds {
            object: vb_core::ids::ObjectId::new(0),
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::BlobOutOfBounds {
            blob: vb_core::ids::BlobId::new(0),
        }
        .runtime_code(),
        None
    );
    assert_eq!(
        CoreError::IterationLimitExceeded { resource: "cpu" }.runtime_code(),
        None
    );
    assert_eq!(
        CoreError::TogetherBranchLimitExceeded { max: 1 }.runtime_code(),
        None
    );
    assert_eq!(
        CoreError::BudgetParse { reason: "bad" }.runtime_code(),
        None
    );
    assert_eq!(
        CoreError::ParallelLimitExceeded { limit: 1 }.runtime_code(),
        None
    );
}
