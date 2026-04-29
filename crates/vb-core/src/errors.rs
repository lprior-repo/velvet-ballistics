#![forbid(unsafe_code)]

//! Typed core failures with stable diagnostic codes.

use crate::diagnostic::DiagnosticCode;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
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
    #[error("constant index out of bounds: {constant:?}")]
    ConstOutOfBounds {
        /// Invalid constant index.
        constant: ConstIdx,
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
    /// Caller supplied a zero execution budget.
    #[error("step budget must be greater than zero")]
    EmptyStepBudget,
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
    /// Non-finite number diagnostic code.
    pub const NON_FINITE_NUMBER_CODE: DiagnosticCode = DiagnosticCode::new(0x0202);
    /// Division by zero diagnostic code.
    pub const DIVISION_BY_ZERO_CODE: DiagnosticCode = DiagnosticCode::new(0x0203);
    /// Step budget exhausted diagnostic code.
    pub const STEP_BUDGET_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x0301);
    /// Step counter overflow diagnostic code.
    pub const STEP_COUNTER_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x0302);
    /// Empty step budget diagnostic code.
    pub const EMPTY_STEP_BUDGET_CODE: DiagnosticCode = DiagnosticCode::new(0x0303);
    /// Queue full diagnostic code.
    pub const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x0401);
    /// Resource limit exceeded diagnostic code.
    pub const RESOURCE_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x0402);
    /// Allocation failed diagnostic code.
    pub const ALLOCATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x0403);

    /// Returns the stable diagnostic code for this error.
    #[must_use]
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::InvalidProgramCounter { .. } => Self::INVALID_PROGRAM_COUNTER_CODE,
            Self::MissingNextStep { .. } => Self::MISSING_NEXT_STEP_CODE,
            Self::SlotOutOfBounds { .. } => Self::SLOT_OUT_OF_BOUNDS_CODE,
            Self::ExprOutOfBounds { .. } => Self::EXPR_OUT_OF_BOUNDS_CODE,
            Self::ConstOutOfBounds { .. } => Self::CONST_OUT_OF_BOUNDS_CODE,
            Self::TypeMismatch { .. } | Self::NonBoolCondition { .. } => Self::TYPE_MISMATCH_CODE,
            Self::NonFiniteNumber => Self::NON_FINITE_NUMBER_CODE,
            Self::DivisionByZero => Self::DIVISION_BY_ZERO_CODE,
            Self::StepBudgetExhausted => Self::STEP_BUDGET_EXHAUSTED_CODE,
            Self::StepCounterOverflow => Self::STEP_COUNTER_OVERFLOW_CODE,
            Self::EmptyStepBudget => Self::EMPTY_STEP_BUDGET_CODE,
            Self::QueueFull => Self::QUEUE_FULL_CODE,
            Self::ResourceLimitExceeded { .. } => Self::RESOURCE_LIMIT_EXCEEDED_CODE,
            Self::AllocationFailed => Self::ALLOCATION_FAILED_CODE,
        }
    }
}
