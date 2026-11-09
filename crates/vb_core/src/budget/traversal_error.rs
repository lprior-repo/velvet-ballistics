#![forbid(unsafe_code)]
//! Budget traversal failures: the narrow error type returned by the budget
//! traversal, and its conversions to workflow and budget errors.

use crate::ids::StepIdx;
use crate::workflow::WorkflowError;

use super::BudgetError;

/// Budget-local traversal failures. Deliberately excludes expression/core error
/// variants so proof harnesses do not pay for unrelated destructor graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BudgetTraversalError {
    EntryOutOfBounds {
        entry: StepIdx,
    },
    StepOutOfBounds {
        step: StepIdx,
    },
    StepCountOverflow {
        actual: u64,
    },
    /// `u16` nesting depth overflowed at `compute_child_depth`; carries the
    /// actual pre-overflow depth so diagnostics can report the real value
    /// instead of the lossy `u64::MAX` sentinel previously produced here.
    DepthOverflow {
        depth: u16,
    },
    JumpCycle {
        step: StepIdx,
        target: StepIdx,
    },
    BackwardScopeExit {
        from: StepIdx,
        to: StepIdx,
    },
}

impl From<BudgetTraversalError> for WorkflowError {
    fn from(error: BudgetTraversalError) -> Self {
        match error {
            BudgetTraversalError::EntryOutOfBounds { entry } => Self::EntryOutOfBounds { entry },
            BudgetTraversalError::StepOutOfBounds { step } => Self::StepOutOfBounds { step },
            BudgetTraversalError::StepCountOverflow { actual } => {
                Self::StepCountOverflow { actual }
            }
            BudgetTraversalError::DepthOverflow { depth } => Self::DepthOverflow { depth },
            BudgetTraversalError::JumpCycle { step, target } => Self::JumpCycle { step, target },
            BudgetTraversalError::BackwardScopeExit { from, to } => Self::BackwardEdge { from, to },
        }
    }
}

impl From<BudgetTraversalError> for BudgetError {
    fn from(_err: BudgetTraversalError) -> Self {
        BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }
    }
}
