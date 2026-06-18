#![forbid(unsafe_code)]
//! Workflow budget validation.
//!
//! Computes the whole-workflow budget and checks it against the declared
//! boundedness policy.

use crate::budget::{BoundednessPolicy, BudgetError, WholeWorkflowBudget};
use crate::workflow::{WorkflowError, WorkflowParts};

/// Validates the workflow budget against the default boundedness policy.
pub fn validate_budget(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;

    validate_budget_result(BoundednessPolicy::DEFAULT.validate(&budget))
}

/// Maps a budget-check result into a [`WorkflowError`].
pub fn validate_budget_result(result: Result<(), BudgetError>) -> Result<(), WorkflowError> {
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: budget_error_detail(&error),
        }),
    }
}

/// Translates a [`BudgetError`] variant into a static error-detail label.
pub(crate) fn budget_error_detail(error: &BudgetError) -> &'static str {
    match error {
        BudgetError::TotalStepsExceeded { .. } => "max_total_steps",
        BudgetError::TotalSlotsExceeded { .. } => "max_total_slots",
        BudgetError::FanoutExceeded { .. } => "max_fanout",
        BudgetError::NestingDepthExceeded { .. } => "max_nesting_depth",
        BudgetError::ParallelExceeded { .. } => "max_parallel_in_flight",
        BudgetError::ActionTicketsExceeded { .. } => "max_action_tickets",
        BudgetError::RunTimeExceeded { .. } => "max_run_time_seconds",
        BudgetError::ResultBytesExceeded { .. } => "max_result_bytes",
        BudgetError::StepsExecutableExceeded { .. } => "max_steps_executable",
        BudgetError::TimerEntriesExceeded { .. } => "max_timer_entries",
        BudgetError::TraceEventsExceeded { .. } => "max_trace_events",
        BudgetError::JournalBatchBytesExceeded { .. } => "max_journal_batch_bytes",
        BudgetError::QueueDepthExceeded { .. } => "max_queue_depth",
        BudgetError::IpcPayloadBytesExceeded { .. } => "max_ipc_payload_bytes",
        BudgetError::BlobBytesExceeded { .. } => "max_blob_bytes",
        BudgetError::InputBytesExceeded { .. } => "max_input_bytes",
    }
}
