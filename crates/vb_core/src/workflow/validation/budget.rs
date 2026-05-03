//! Whole-workflow budget validation.

use crate::budget::{BoundednessPolicy, BudgetError, WholeWorkflowBudget};

use super::super::types::{WorkflowError, WorkflowParts};

/// Validates that the whole-workflow budget satisfies the boundedness policy.
pub(crate) fn validate_budget(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let budget = WholeWorkflowBudget::compute(
        &parts.nodes,
        parts.entry,
        &parts.resource_contract,
    )?;

    match BoundednessPolicy::DEFAULT.validate(&budget) {
        Ok(()) => Ok(()),
        Err(BudgetError::TotalStepsExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_total_steps",
        }),
        Err(BudgetError::TotalSlotsExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_total_slots",
        }),
        Err(BudgetError::FanoutExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_fanout",
        }),
        Err(BudgetError::NestingDepthExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_nesting_depth",
        }),
        Err(BudgetError::ParallelExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_parallel",
        }),
        Err(BudgetError::ActionTicketsExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_action_tickets",
        }),
        Err(BudgetError::RunTimeExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_runtime",
        }),
        Err(BudgetError::ResultBytesExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_result_bytes",
        }),
        Err(BudgetError::StepsExecutableExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_steps_executable",
        }),
    }
}
