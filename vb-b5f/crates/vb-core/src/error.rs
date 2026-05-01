//! Typed engine failures.

use crate::ids::{ConstIdx, SlotIdx, StepIdx};
use thiserror::Error;

/// Failures emitted by the in-memory engine loop.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    /// Program counter pointed outside the compiled node array.
    #[error("invalid program counter {step:?}")]
    InvalidProgramCounter {
        /// Invalid step index.
        step: StepIdx,
    },

    /// A node referenced a missing constant-pool entry.
    #[error("constant index {constant:?} is outside the constant pool")]
    ConstOutOfBounds {
        /// Invalid constant index.
        constant: ConstIdx,
    },

    /// A node referenced a missing slot.
    #[error("slot index {slot:?} is outside the run frame")]
    SlotOutOfBounds {
        /// Invalid slot index.
        slot: SlotIdx,
    },

    /// A choose node was compiled against a non-boolean condition slot.
    #[error("choose condition slot {slot:?} does not contain a boolean")]
    NonBoolCondition {
        /// Condition slot.
        slot: SlotIdx,
    },

    /// Per-call step budget was exhausted before the run blocked or finished.
    #[error("step budget exhausted")]
    StepBudgetExhausted,

    /// Run step counter overflowed.
    #[error("step counter overflow")]
    StepCounterOverflow,

    /// Caller supplied a zero execution budget.
    #[error("step budget must be greater than zero")]
    EmptyStepBudget,
}
