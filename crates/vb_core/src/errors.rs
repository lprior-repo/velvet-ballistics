#![forbid(unsafe_code)]

//! Typed core failures with stable diagnostic codes.

use crate::diagnostic::DiagnosticCode;
use crate::ids::{BlobId, ConstIdx, ExprIdx, ListId, ObjectId, SlotIdx, StepIdx, SymbolId};
use thiserror::Error;

/// Result alias for core operations.
pub type CoreResult<T> = Result<T, CoreError>;

/// Backward-compatible engine error name.
pub type EngineError = CoreError;

/// Failures emitted by core validation and execution code.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Program counter pointed outside the compiled node array.
    #[error("invalid program counter: {step:?}")]
    InvalidProgramCounter {
        /// Invalid step index.
        step: StepIdx,
    },
    /// Step did not have a required next transition.
    #[error("missing next step for {step:?}")]
    MissingNextStep {
        /// Step missing a next transition.
        step: StepIdx,
    },
    /// A node referenced a missing slot.
    #[error("slot index out of bounds: {slot:?}")]
    SlotOutOfBounds {
        /// Invalid slot index.
        slot: SlotIdx,
    },
    /// A node referenced a missing expression program.
    #[error("expression index out of bounds: {expr:?}")]
    ExprOutOfBounds {
        /// Invalid expression index.
        expr: ExprIdx,
    },
    /// A node referenced a missing constant-pool entry.
    #[error("constant index out of bounds: {index:?}")]
    ConstOutOfBounds {
        /// Invalid constant index.
        index: ConstIdx,
    },
    /// A node requiring an output slot did not carry one.
    #[error("missing output slot for {step:?}")]
    MissingOutputSlot {
        /// Step missing an output slot.
        step: StepIdx,
    },
    /// A step-state index was outside the run frame.
    #[error("step state index out of bounds: {step:?}")]
    StepStateOutOfBounds {
        /// Invalid step index.
        step: StepIdx,
    },
    /// A value had the wrong runtime type.
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        /// Required type name.
        expected: &'static str,
        /// Actual type name.
        found: &'static str,
    },
    /// A choose node was compiled against a non-boolean condition slot.
    #[error("type mismatch: expected boolean, found slot {slot:?}")]
    NonBoolCondition {
        /// Condition slot.
        slot: SlotIdx,
    },
    /// Arithmetic attempted to divide by zero.
    #[error("division by zero")]
    DivisionByZero,
    /// Non-finite numeric values are rejected.
    #[error("non-finite number is not allowed")]
    NonFiniteNumber,
    /// Per-call step budget was exhausted before the run blocked or finished.
    #[error("step budget exhausted")]
    StepBudgetExhausted,
    /// Run step counter overflowed.
    #[error("step counter overflow")]
    StepCounterOverflow,
    /// Bounded queue was full.
    #[error("queue full")]
    QueueFull,
    /// A bounded resource limit was exceeded.
    #[error("resource limit exceeded: {resource}")]
    ResourceLimitExceeded {
        /// Resource name.
        resource: &'static str,
    },
    /// Allocation failed at a fallible allocation boundary.
    #[error("allocation failed")]
    AllocationFailed,
    /// Expression bytecode exceeded its declared or global stack capacity.
    #[error("expression stack overflow: max {max}")]
    ExpressionStackOverflow {
        /// Maximum allowed stack entries.
        max: u8,
    },
    /// Expression bytecode attempted to consume a missing stack value.
    #[error("expression stack underflow")]
    ExpressionStackUnderflow,
    /// Compiled workflow failed an internal consistency check.
    #[error("invalid compiled workflow: {reason}")]
    InvalidCompiledWorkflow {
        /// Stable validation failure reason.
        reason: &'static str,
    },
    /// Runtime reached a valid IR primitive not implemented by this build slice.
    #[error("unsupported primitive: {primitive}")]
    UnsupportedPrimitive {
        /// Primitive name.
        primitive: &'static str,
    },
    /// Accessor traversal needs cold arena data unavailable to the hot frame.
    #[error("unsupported accessor traversal: {segment} on {found}")]
    UnsupportedAccessorTraversal {
        /// Path segment kind.
        segment: &'static str,
        /// Runtime type being traversed.
        found: &'static str,
    },
    /// An object accessor field was not present in the object arena payload.
    #[error("object field not found: {field:?}")]
    ObjectFieldNotFound {
        /// Missing interned field name.
        field: SymbolId,
    },
    /// A list accessor index was outside the list arena payload.
    #[error("list index out of bounds: {index}")]
    ListIndexOutOfBounds {
        /// Missing list index.
        index: u32,
    },
    /// Internal invariant violation.
    #[error("internal invariant violation: {reason}")]
    InternalInvariantViolation {
        /// Stable invariant reason.
        reason: &'static str,
    },
    /// A symbol handle did not resolve in the cold value store.
    #[error("symbol id out of bounds: {symbol:?}")]
    SymbolOutOfBounds {
        /// Invalid symbol handle.
        symbol: SymbolId,
    },
    /// A list handle did not resolve in the cold value store.
    #[error("list id out of bounds: {list:?}")]
    ListOutOfBounds {
        /// Invalid list handle.
        list: ListId,
    },
    /// An object handle did not resolve in the cold value store.
    #[error("object id out of bounds: {object:?}")]
    ObjectOutOfBounds {
        /// Invalid object handle.
        object: ObjectId,
    },
    /// A blob handle did not resolve in the cold value store.
    #[error("blob id out of bounds: {blob:?}")]
    BlobOutOfBounds {
        /// Invalid blob handle.
        blob: BlobId,
    },
    /// An iteration limit was exceeded.
    #[error("iteration limit exceeded: {resource}")]
    IterationLimitExceeded {
        /// Resource name.
        resource: &'static str,
    },
    /// A repeat loop exhausted its maximum attempts.
    #[error("repeat exhausted max attempts: {max}")]
    RepeatExhausted {
        /// Maximum attempts.
        max: u16,
    },
    /// A collection pagination limit was exceeded.
    #[error("collect page limit exceeded")]
    CollectPageLimitExceeded,
    /// A collection item limit was exceeded.
    #[error("collect item limit exceeded")]
    CollectItemLimitExceeded,
    /// Together branch count exceeded the bound.
    #[error("together branch limit exceeded: {max}")]
    TogetherBranchLimitExceeded {
        /// Maximum branches.
        max: u16,
    },
}

impl CoreError {
    /// Historical invalid program-counter code.
    pub const DIAGNOSTIC_CODE: u16 = 0x0101;
    /// Invalid program counter diagnostic code.
    pub const INVALID_PROGRAM_COUNTER_CODE: DiagnosticCode = DiagnosticCode::new(0x0101);
    /// Missing next step diagnostic code.
    pub const MISSING_NEXT_STEP_CODE: DiagnosticCode = DiagnosticCode::new(0x0102);
    /// Slot out-of-bounds diagnostic code.
    pub const SLOT_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x0111);
    /// Expression out-of-bounds diagnostic code.
    pub const EXPR_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x0112);
    /// Constant out-of-bounds diagnostic code.
    pub const CONST_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x0113);
    /// Type mismatch diagnostic code.
    pub const TYPE_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x0201);
    /// Non-boolean condition diagnostic code.
    pub const NON_BOOL_CONDITION_CODE: DiagnosticCode = DiagnosticCode::new(0x0204);
    /// Non-finite number diagnostic code.
    pub const NON_FINITE_NUMBER_CODE: DiagnosticCode = DiagnosticCode::new(0x0202);
    /// Division by zero diagnostic code.
    pub const DIVISION_BY_ZERO_CODE: DiagnosticCode = DiagnosticCode::new(0x0203);
    /// Step budget exhausted diagnostic code.
    pub const STEP_BUDGET_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x0301);
    /// Step counter overflow diagnostic code.
    pub const STEP_COUNTER_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x0302);
    /// Queue full diagnostic code.
    pub const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x0401);
    /// Resource limit exceeded diagnostic code.
    pub const RESOURCE_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x0402);
    /// Allocation failed diagnostic code.
    pub const ALLOCATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x0403);
    /// Expression stack overflow diagnostic code.
    pub const EXPRESSION_STACK_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x0404);
    /// Missing output slot diagnostic code.
    pub const MISSING_OUTPUT_SLOT_CODE: DiagnosticCode = DiagnosticCode::new(0x0405);
    /// Step state out-of-bounds diagnostic code.
    pub const STEP_STATE_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x0406);
    /// Invalid compiled workflow diagnostic code.
    pub const INVALID_COMPILED_WORKFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x0407);
    /// Unsupported primitive diagnostic code.
    pub const UNSUPPORTED_PRIMITIVE_CODE: DiagnosticCode = DiagnosticCode::new(0x0408);
    /// Internal invariant diagnostic code.
    pub const INTERNAL_INVARIANT_CODE: DiagnosticCode = DiagnosticCode::new(0x0409);
    /// Unsupported accessor traversal diagnostic code.
    pub const UNSUPPORTED_ACCESSOR_TRAVERSAL_CODE: DiagnosticCode = DiagnosticCode::new(0x040A);
    /// Object accessor field not found diagnostic code.
    pub const OBJECT_FIELD_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x040C);
    /// List accessor index out-of-bounds diagnostic code.
    pub const LIST_INDEX_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x040D);
    /// Expression stack underflow diagnostic code.
    pub const EXPRESSION_STACK_UNDERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x040B);
    /// Symbol handle out-of-bounds diagnostic code.
    pub const SYMBOL_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x0411);
    /// List handle out-of-bounds diagnostic code.
    pub const LIST_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x0412);
    /// Object handle out-of-bounds diagnostic code.
    pub const OBJECT_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x0413);
    /// Blob handle out-of-bounds diagnostic code.
    pub const BLOB_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x0414);
    /// Iteration limit exceeded diagnostic code.
    pub const ITERATION_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x0501);
    /// Repeat exhausted diagnostic code.
    pub const REPEAT_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x0502);
    /// Collect page limit exceeded diagnostic code.
    pub const COLLECT_PAGE_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x0503);
    /// Collect item limit exceeded diagnostic code.
    pub const COLLECT_ITEM_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x0504);
    /// Together branch limit exceeded diagnostic code.
    pub const TOGETHER_BRANCH_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x0505);

    /// Returns the stable diagnostic code for this error.
    #[must_use]
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::InvalidProgramCounter { .. } => Self::INVALID_PROGRAM_COUNTER_CODE,
            Self::MissingNextStep { .. } => Self::MISSING_NEXT_STEP_CODE,
            Self::SlotOutOfBounds { .. } => Self::SLOT_OUT_OF_BOUNDS_CODE,
            Self::ExprOutOfBounds { .. } => Self::EXPR_OUT_OF_BOUNDS_CODE,
            Self::ConstOutOfBounds { .. } => Self::CONST_OUT_OF_BOUNDS_CODE,
            Self::MissingOutputSlot { .. } => Self::MISSING_OUTPUT_SLOT_CODE,
            Self::StepStateOutOfBounds { .. } => Self::STEP_STATE_OUT_OF_BOUNDS_CODE,
            Self::TypeMismatch { .. } => Self::TYPE_MISMATCH_CODE,
            Self::NonBoolCondition { .. } => Self::NON_BOOL_CONDITION_CODE,
            Self::NonFiniteNumber => Self::NON_FINITE_NUMBER_CODE,
            Self::DivisionByZero => Self::DIVISION_BY_ZERO_CODE,
            Self::StepBudgetExhausted => Self::STEP_BUDGET_EXHAUSTED_CODE,
            Self::StepCounterOverflow => Self::STEP_COUNTER_OVERFLOW_CODE,
            Self::QueueFull => Self::QUEUE_FULL_CODE,
            Self::ResourceLimitExceeded { .. } => Self::RESOURCE_LIMIT_EXCEEDED_CODE,
            Self::AllocationFailed => Self::ALLOCATION_FAILED_CODE,
            Self::ExpressionStackOverflow { .. } => Self::EXPRESSION_STACK_OVERFLOW_CODE,
            Self::ExpressionStackUnderflow => Self::EXPRESSION_STACK_UNDERFLOW_CODE,
            Self::InvalidCompiledWorkflow { .. } => Self::INVALID_COMPILED_WORKFLOW_CODE,
            Self::UnsupportedPrimitive { .. } => Self::UNSUPPORTED_PRIMITIVE_CODE,
            Self::UnsupportedAccessorTraversal { .. } => Self::UNSUPPORTED_ACCESSOR_TRAVERSAL_CODE,
            Self::ObjectFieldNotFound { .. } => Self::OBJECT_FIELD_NOT_FOUND_CODE,
            Self::ListIndexOutOfBounds { .. } => Self::LIST_INDEX_OUT_OF_BOUNDS_CODE,
            Self::InternalInvariantViolation { .. } => Self::INTERNAL_INVARIANT_CODE,
            Self::SymbolOutOfBounds { .. } => Self::SYMBOL_OUT_OF_BOUNDS_CODE,
            Self::ListOutOfBounds { .. } => Self::LIST_OUT_OF_BOUNDS_CODE,
            Self::ObjectOutOfBounds { .. } => Self::OBJECT_OUT_OF_BOUNDS_CODE,
            Self::BlobOutOfBounds { .. } => Self::BLOB_OUT_OF_BOUNDS_CODE,
            Self::IterationLimitExceeded { .. } => Self::ITERATION_LIMIT_EXCEEDED_CODE,
            Self::RepeatExhausted { .. } => Self::REPEAT_EXHAUSTED_CODE,
            Self::CollectPageLimitExceeded => Self::COLLECT_PAGE_LIMIT_CODE,
            Self::CollectItemLimitExceeded => Self::COLLECT_ITEM_LIMIT_CODE,
            Self::TogetherBranchLimitExceeded { .. } => Self::TOGETHER_BRANCH_LIMIT_CODE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreError, DiagnosticCode};
    use crate::ids::{
        BlobId, ConstIdx, ExprIdx, ListId, ObjectId, SlotIdx, StepIdx, SymbolId,
    };

    // -- diagnostic_code is correct for every variant --

    #[test]
    fn core_error_diagnostic_code_invalid_program_counter() {
        let error = CoreError::InvalidProgramCounter {
            step: StepIdx::new(5),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0101)
        );
        assert_eq!(error.to_string(), "invalid program counter: StepIdx(5)");
    }

    #[test]
    fn core_error_diagnostic_code_missing_next_step() {
        let error = CoreError::MissingNextStep {
            step: StepIdx::new(3),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0102)
        );
        assert_eq!(error.to_string(), "missing next step for StepIdx(3)");
    }

    #[test]
    fn core_error_diagnostic_code_slot_out_of_bounds() {
        let error = CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(99),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0111)
        );
        assert_eq!(error.to_string(), "slot index out of bounds: SlotIdx(99)");
    }

    #[test]
    fn core_error_diagnostic_code_expr_out_of_bounds() {
        let error = CoreError::ExprOutOfBounds {
            expr: ExprIdx::new(7),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0112)
        );
        assert_eq!(error.to_string(), "expression index out of bounds: ExprIdx(7)");
    }

    #[test]
    fn core_error_diagnostic_code_const_out_of_bounds() {
        let error = CoreError::ConstOutOfBounds {
            index: ConstIdx::new(12),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0113)
        );
        assert_eq!(error.to_string(), "constant index out of bounds: ConstIdx(12)");
    }

    #[test]
    fn core_error_diagnostic_code_missing_output_slot() {
        let error = CoreError::MissingOutputSlot {
            step: StepIdx::new(2),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0405)
        );
        assert_eq!(error.to_string(), "missing output slot for StepIdx(2)");
    }

    #[test]
    fn core_error_diagnostic_code_step_state_out_of_bounds() {
        let error = CoreError::StepStateOutOfBounds {
            step: StepIdx::new(200),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0406)
        );
        assert_eq!(error.to_string(), "step state index out of bounds: StepIdx(200)");
    }

    #[test]
    fn core_error_diagnostic_code_type_mismatch() {
        let error = CoreError::TypeMismatch {
            expected: "number",
            found: "boolean",
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0201)
        );
        assert_eq!(error.to_string(), "type mismatch: expected number, found boolean");
    }

    #[test]
    fn core_error_diagnostic_code_non_bool_condition() {
        let error = CoreError::NonBoolCondition {
            slot: SlotIdx::new(4),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0204)
        );
        assert_eq!(
            error.to_string(),
            "type mismatch: expected boolean, found slot SlotIdx(4)"
        );
    }

    #[test]
    fn core_error_diagnostic_code_division_by_zero() {
        let error = CoreError::DivisionByZero;
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0203)
        );
        assert_eq!(error.to_string(), "division by zero");
    }

    #[test]
    fn core_error_diagnostic_code_non_finite_number() {
        let error = CoreError::NonFiniteNumber;
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0202)
        );
        assert_eq!(error.to_string(), "non-finite number is not allowed");
    }

    #[test]
    fn core_error_diagnostic_code_step_budget_exhausted() {
        let error = CoreError::StepBudgetExhausted;
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0301)
        );
        assert_eq!(error.to_string(), "step budget exhausted");
    }

    #[test]
    fn core_error_diagnostic_code_step_counter_overflow() {
        let error = CoreError::StepCounterOverflow;
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0302)
        );
        assert_eq!(error.to_string(), "step counter overflow");
    }

    #[test]
    fn core_error_diagnostic_code_queue_full() {
        let error = CoreError::QueueFull;
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0401)
        );
        assert_eq!(error.to_string(), "queue full");
    }

    #[test]
    fn core_error_diagnostic_code_resource_limit_exceeded() {
        let error = CoreError::ResourceLimitExceeded {
            resource: "memory",
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0402)
        );
        assert_eq!(error.to_string(), "resource limit exceeded: memory");
    }

    #[test]
    fn core_error_diagnostic_code_allocation_failed() {
        let error = CoreError::AllocationFailed;
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0403)
        );
        assert_eq!(error.to_string(), "allocation failed");
    }

    #[test]
    fn core_error_diagnostic_code_expression_stack_overflow() {
        let error = CoreError::ExpressionStackOverflow { max: 64 };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0404)
        );
        assert_eq!(error.to_string(), "expression stack overflow: max 64");
    }

    #[test]
    fn core_error_diagnostic_code_expression_stack_underflow() {
        let error = CoreError::ExpressionStackUnderflow;
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x040B)
        );
        assert_eq!(error.to_string(), "expression stack underflow");
    }

    #[test]
    fn core_error_diagnostic_code_invalid_compiled_workflow() {
        let error = CoreError::InvalidCompiledWorkflow {
            reason: "bad node",
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0407)
        );
        assert_eq!(error.to_string(), "invalid compiled workflow: bad node");
    }

    #[test]
    fn core_error_diagnostic_code_unsupported_primitive() {
        let error = CoreError::UnsupportedPrimitive {
            primitive: "fancy_op",
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0408)
        );
        assert_eq!(error.to_string(), "unsupported primitive: fancy_op");
    }

    #[test]
    fn core_error_diagnostic_code_unsupported_accessor_traversal() {
        let error = CoreError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "list",
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x040A)
        );
        assert_eq!(
            error.to_string(),
            "unsupported accessor traversal: field on list"
        );
    }

    #[test]
    fn core_error_diagnostic_code_object_field_not_found() {
        let error = CoreError::ObjectFieldNotFound {
            field: SymbolId::new(42),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x040C)
        );
        assert_eq!(error.to_string(), "object field not found: SymbolId(42)");
    }

    #[test]
    fn core_error_diagnostic_code_list_index_out_of_bounds() {
        let error = CoreError::ListIndexOutOfBounds { index: 10 };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x040D)
        );
        assert_eq!(error.to_string(), "list index out of bounds: 10");
    }

    #[test]
    fn core_error_diagnostic_code_internal_invariant_violation() {
        let error = CoreError::InternalInvariantViolation {
            reason: "impossible",
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0409)
        );
        assert_eq!(error.to_string(), "internal invariant violation: impossible");
    }

    #[test]
    fn core_error_diagnostic_code_symbol_out_of_bounds() {
        let error = CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(100),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0411)
        );
        assert_eq!(error.to_string(), "symbol id out of bounds: SymbolId(100)");
    }

    #[test]
    fn core_error_diagnostic_code_list_out_of_bounds() {
        let error = CoreError::ListOutOfBounds {
            list: ListId::new(7),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0412)
        );
        assert_eq!(error.to_string(), "list id out of bounds: ListId(7)");
    }

    #[test]
    fn core_error_diagnostic_code_object_out_of_bounds() {
        let error = CoreError::ObjectOutOfBounds {
            object: ObjectId::new(3),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0413)
        );
        assert_eq!(error.to_string(), "object id out of bounds: ObjectId(3)");
    }

    #[test]
    fn core_error_diagnostic_code_blob_out_of_bounds() {
        let error = CoreError::BlobOutOfBounds {
            blob: BlobId::new(9),
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0414)
        );
        assert_eq!(error.to_string(), "blob id out of bounds: BlobId(9)");
    }

    #[test]
    fn core_error_diagnostic_code_iteration_limit_exceeded() {
        let error = CoreError::IterationLimitExceeded {
            resource: "for_each",
        };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0501)
        );
        assert_eq!(error.to_string(), "iteration limit exceeded: for_each");
    }

    #[test]
    fn core_error_diagnostic_code_repeat_exhausted() {
        let error = CoreError::RepeatExhausted { max: 5 };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0502)
        );
        assert_eq!(error.to_string(), "repeat exhausted max attempts: 5");
    }

    #[test]
    fn core_error_diagnostic_code_collect_page_limit_exceeded() {
        let error = CoreError::CollectPageLimitExceeded;
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0503)
        );
        assert_eq!(error.to_string(), "collect page limit exceeded");
    }

    #[test]
    fn core_error_diagnostic_code_collect_item_limit_exceeded() {
        let error = CoreError::CollectItemLimitExceeded;
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0504)
        );
        assert_eq!(error.to_string(), "collect item limit exceeded");
    }

    #[test]
    fn core_error_diagnostic_code_together_branch_limit_exceeded() {
        let error = CoreError::TogetherBranchLimitExceeded { max: 32 };
        assert_eq!(
            error.diagnostic_code(),
            DiagnosticCode::new(0x0505)
        );
        assert_eq!(error.to_string(), "together branch limit exceeded: 32");
    }

    // -- exact variant field assertions for variants with fields --

    #[test]
    fn core_error_invalid_program_counter_exact_variant() {
        let error = CoreError::InvalidProgramCounter {
            step: StepIdx::new(42),
        };
        let CoreError::InvalidProgramCounter { step } = error else {
            panic!("expected InvalidProgramCounter variant");
        };
        assert_eq!(step, StepIdx::new(42));
    }

    #[test]
    fn core_error_missing_next_step_exact_variant() {
        let error = CoreError::MissingNextStep {
            step: StepIdx::new(10),
        };
        let CoreError::MissingNextStep { step } = error else {
            panic!("expected MissingNextStep variant");
        };
        assert_eq!(step, StepIdx::new(10));
    }

    #[test]
    fn core_error_slot_out_of_bounds_exact_variant() {
        let error = CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(255),
        };
        let CoreError::SlotOutOfBounds { slot } = error else {
            panic!("expected SlotOutOfBounds variant");
        };
        assert_eq!(slot, SlotIdx::new(255));
    }

    #[test]
    fn core_error_expr_out_of_bounds_exact_variant() {
        let error = CoreError::ExprOutOfBounds {
            expr: ExprIdx::new(8),
        };
        let CoreError::ExprOutOfBounds { expr } = error else {
            panic!("expected ExprOutOfBounds variant");
        };
        assert_eq!(expr, ExprIdx::new(8));
    }

    #[test]
    fn core_error_const_out_of_bounds_exact_variant() {
        let error = CoreError::ConstOutOfBounds {
            index: ConstIdx::new(99),
        };
        let CoreError::ConstOutOfBounds { index } = error else {
            panic!("expected ConstOutOfBounds variant");
        };
        assert_eq!(index, ConstIdx::new(99));
    }

    #[test]
    fn core_error_missing_output_slot_exact_variant() {
        let error = CoreError::MissingOutputSlot {
            step: StepIdx::new(1),
        };
        let CoreError::MissingOutputSlot { step } = error else {
            panic!("expected MissingOutputSlot variant");
        };
        assert_eq!(step, StepIdx::new(1));
    }

    #[test]
    fn core_error_step_state_out_of_bounds_exact_variant() {
        let error = CoreError::StepStateOutOfBounds {
            step: StepIdx::new(500),
        };
        let CoreError::StepStateOutOfBounds { step } = error else {
            panic!("expected StepStateOutOfBounds variant");
        };
        assert_eq!(step, StepIdx::new(500));
    }

    #[test]
    fn core_error_type_mismatch_exact_variant() {
        let error = CoreError::TypeMismatch {
            expected: "i64",
            found: "bool",
        };
        let CoreError::TypeMismatch { expected, found } = error else {
            panic!("expected TypeMismatch variant");
        };
        assert_eq!(expected, "i64");
        assert_eq!(found, "bool");
    }

    #[test]
    fn core_error_non_bool_condition_exact_variant() {
        let error = CoreError::NonBoolCondition {
            slot: SlotIdx::new(3),
        };
        let CoreError::NonBoolCondition { slot } = error else {
            panic!("expected NonBoolCondition variant");
        };
        assert_eq!(slot, SlotIdx::new(3));
    }

    #[test]
    fn core_error_resource_limit_exceeded_exact_variant() {
        let error = CoreError::ResourceLimitExceeded {
            resource: "slots",
        };
        let CoreError::ResourceLimitExceeded { resource } = error else {
            panic!("expected ResourceLimitExceeded variant");
        };
        assert_eq!(resource, "slots");
    }

    #[test]
    fn core_error_expression_stack_overflow_exact_variant() {
        let error = CoreError::ExpressionStackOverflow { max: 128 };
        let CoreError::ExpressionStackOverflow { max } = error else {
            panic!("expected ExpressionStackOverflow variant");
        };
        assert_eq!(max, 128);
    }

    #[test]
    fn core_error_invalid_compiled_workflow_exact_variant() {
        let error = CoreError::InvalidCompiledWorkflow {
            reason: "missing entry",
        };
        let CoreError::InvalidCompiledWorkflow { reason } = error else {
            panic!("expected InvalidCompiledWorkflow variant");
        };
        assert_eq!(reason, "missing entry");
    }

    #[test]
    fn core_error_unsupported_primitive_exact_variant() {
        let error = CoreError::UnsupportedPrimitive {
            primitive: "async_await",
        };
        let CoreError::UnsupportedPrimitive { primitive } = error else {
            panic!("expected UnsupportedPrimitive variant");
        };
        assert_eq!(primitive, "async_await");
    }

    #[test]
    fn core_error_unsupported_accessor_traversal_exact_variant() {
        let error = CoreError::UnsupportedAccessorTraversal {
            segment: "index",
            found: "object",
        };
        let CoreError::UnsupportedAccessorTraversal { segment, found } = error else {
            panic!("expected UnsupportedAccessorTraversal variant");
        };
        assert_eq!(segment, "index");
        assert_eq!(found, "object");
    }

    #[test]
    fn core_error_object_field_not_found_exact_variant() {
        let error = CoreError::ObjectFieldNotFound {
            field: SymbolId::new(7),
        };
        let CoreError::ObjectFieldNotFound { field } = error else {
            panic!("expected ObjectFieldNotFound variant");
        };
        assert_eq!(field, SymbolId::new(7));
    }

    #[test]
    fn core_error_list_index_out_of_bounds_exact_variant() {
        let error = CoreError::ListIndexOutOfBounds { index: 999 };
        let CoreError::ListIndexOutOfBounds { index } = error else {
            panic!("expected ListIndexOutOfBounds variant");
        };
        assert_eq!(index, 999);
    }

    #[test]
    fn core_error_internal_invariant_violation_exact_variant() {
        let error = CoreError::InternalInvariantViolation {
            reason: "corrupted",
        };
        let CoreError::InternalInvariantViolation { reason } = error else {
            panic!("expected InternalInvariantViolation variant");
        };
        assert_eq!(reason, "corrupted");
    }

    #[test]
    fn core_error_symbol_out_of_bounds_exact_variant() {
        let error = CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(55),
        };
        let CoreError::SymbolOutOfBounds { symbol } = error else {
            panic!("expected SymbolOutOfBounds variant");
        };
        assert_eq!(symbol, SymbolId::new(55));
    }

    #[test]
    fn core_error_list_out_of_bounds_exact_variant() {
        let error = CoreError::ListOutOfBounds {
            list: ListId::new(33),
        };
        let CoreError::ListOutOfBounds { list } = error else {
            panic!("expected ListOutOfBounds variant");
        };
        assert_eq!(list, ListId::new(33));
    }

    #[test]
    fn core_error_object_out_of_bounds_exact_variant() {
        let error = CoreError::ObjectOutOfBounds {
            object: ObjectId::new(21),
        };
        let CoreError::ObjectOutOfBounds { object } = error else {
            panic!("expected ObjectOutOfBounds variant");
        };
        assert_eq!(object, ObjectId::new(21));
    }

    #[test]
    fn core_error_blob_out_of_bounds_exact_variant() {
        let error = CoreError::BlobOutOfBounds {
            blob: BlobId::new(11),
        };
        let CoreError::BlobOutOfBounds { blob } = error else {
            panic!("expected BlobOutOfBounds variant");
        };
        assert_eq!(blob, BlobId::new(11));
    }

    #[test]
    fn core_error_iteration_limit_exceeded_exact_variant() {
        let error = CoreError::IterationLimitExceeded {
            resource: "collect",
        };
        let CoreError::IterationLimitExceeded { resource } = error else {
            panic!("expected IterationLimitExceeded variant");
        };
        assert_eq!(resource, "collect");
    }

    #[test]
    fn core_error_repeat_exhausted_exact_variant() {
        let error = CoreError::RepeatExhausted { max: 10 };
        let CoreError::RepeatExhausted { max } = error else {
            panic!("expected RepeatExhausted variant");
        };
        assert_eq!(max, 10);
    }

    #[test]
    fn core_error_together_branch_limit_exceeded_exact_variant() {
        let error = CoreError::TogetherBranchLimitExceeded { max: 64 };
        let CoreError::TogetherBranchLimitExceeded { max } = error else {
            panic!("expected TogetherBranchLimitExceeded variant");
        };
        assert_eq!(max, 64);
    }
}
