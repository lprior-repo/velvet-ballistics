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
    use super::CoreError;
    use crate::ids::{BlobId, ConstIdx, ExprIdx, ListId, ObjectId, SlotIdx, StepIdx, SymbolId};

    #[test]
    fn runtime_diagnostic_codes_are_unique() -> Result<(), String> {
        let errors = [
            CoreError::InvalidProgramCounter {
                step: StepIdx::new(0),
            },
            CoreError::MissingNextStep {
                step: StepIdx::new(0),
            },
            CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(0),
            },
            CoreError::ExprOutOfBounds {
                expr: ExprIdx::new(0),
            },
            CoreError::ConstOutOfBounds {
                index: ConstIdx::new(0),
            },
            CoreError::MissingOutputSlot {
                step: StepIdx::new(0),
            },
            CoreError::StepStateOutOfBounds {
                step: StepIdx::new(0),
            },
            CoreError::TypeMismatch {
                expected: "a",
                found: "b",
            },
            CoreError::NonBoolCondition {
                slot: SlotIdx::new(0),
            },
            CoreError::DivisionByZero,
            CoreError::NonFiniteNumber,
            CoreError::StepBudgetExhausted,
            CoreError::StepCounterOverflow,
            CoreError::QueueFull,
            CoreError::ResourceLimitExceeded { resource: "r" },
            CoreError::AllocationFailed,
            CoreError::ExpressionStackOverflow { max: 1 },
            CoreError::ExpressionStackUnderflow,
            CoreError::InvalidCompiledWorkflow { reason: "r" },
            CoreError::UnsupportedPrimitive { primitive: "p" },
            CoreError::UnsupportedAccessorTraversal {
                segment: "s",
                found: "f",
            },
            CoreError::ObjectFieldNotFound {
                field: SymbolId::new(0),
            },
            CoreError::ListIndexOutOfBounds { index: 0 },
            CoreError::InternalInvariantViolation { reason: "r" },
            CoreError::SymbolOutOfBounds {
                symbol: SymbolId::new(0),
            },
            CoreError::ListOutOfBounds {
                list: ListId::new(0),
            },
            CoreError::ObjectOutOfBounds {
                object: ObjectId::new(0),
            },
            CoreError::BlobOutOfBounds {
                blob: BlobId::new(0),
            },
            CoreError::IterationLimitExceeded { resource: "r" },
            CoreError::RepeatExhausted { max: 1 },
            CoreError::CollectPageLimitExceeded,
            CoreError::CollectItemLimitExceeded,
            CoreError::TogetherBranchLimitExceeded { max: 1 },
        ];

        let mut left = 0usize;
        while left < errors.len() {
            let mut right = left.saturating_add(1);
            while right < errors.len() {
                let left_code = errors[left].diagnostic_code().code();
                let right_code = errors[right].diagnostic_code().code();
                if left_code == right_code {
                    return Err(format!("duplicate diagnostic code E{left_code:04X}"));
                }
                right = right.saturating_add(1);
            }
            left = left.saturating_add(1);
        }
        Ok(())
    }
}
