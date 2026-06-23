#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use crate::ids::StepIdx;
use crate::workflow::WorkflowError;

/// Budget-local traversal failures. Deliberately excludes expression/core error
/// variants so proof harnesses do not pay for unrelated destructor graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BudgetTraversalError {
    EntryOutOfBounds { entry: StepIdx },
    StepOutOfBounds { step: StepIdx },
    StepCountOverflow { actual: u64 },
    JumpCycle { step: StepIdx, target: StepIdx },
    InvalidCompiledWorkflow { reason: &'static str },
}

impl From<BudgetTraversalError> for WorkflowError {
    fn from(error: BudgetTraversalError) -> Self {
        match error {
            BudgetTraversalError::EntryOutOfBounds { entry } => Self::EntryOutOfBounds { entry },
            BudgetTraversalError::StepOutOfBounds { step } => Self::StepOutOfBounds { step },
            BudgetTraversalError::StepCountOverflow { actual } => {
                Self::StepCountOverflow { actual }
            }
            BudgetTraversalError::JumpCycle { step, target } => Self::JumpCycle { step, target },
            BudgetTraversalError::InvalidCompiledWorkflow { reason } => {
                Self::Expression(crate::errors::CoreError::InvalidCompiledWorkflow { reason })
            }
        }
    }
}
