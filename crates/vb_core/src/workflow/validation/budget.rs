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
        BudgetError::WorkflowEntryOutOfBounds { .. } => "workflow_entry_out_of_bounds",
        BudgetError::WorkflowStepOutOfBounds { .. } => "workflow_step_out_of_bounds",
        BudgetError::WorkflowSlotOutOfBounds { .. } => "workflow_slot_out_of_bounds",
        BudgetError::WorkflowConstOutOfBounds { .. } => "workflow_const_out_of_bounds",
        BudgetError::WorkflowNodeIdMismatch { .. } => "workflow_node_id_mismatch",
        BudgetError::WorkflowExpression { .. } => "workflow_expression",
        BudgetError::ResourceContractExceeded { .. } => "workflow_resource_contract_exceeded",
        BudgetError::ResourceContractTooLarge { .. } => "workflow_resource_contract_too_large",
        BudgetError::EmptyBranchTable => "workflow_empty_branch_table",
        BudgetError::UnreachableNode { .. } => "workflow_unreachable_node",
        BudgetError::BackwardEdge { .. } => "workflow_backward_edge",
        BudgetError::ImproperLoopNesting { .. } => "workflow_improper_loop_nesting",
        BudgetError::BudgetPolicyExceeded { .. } => "workflow_budget_policy_exceeded",
        BudgetError::StepCountOverflow { .. } => "workflow_step_count_overflow",
        BudgetError::WorkflowSymbolOutOfBounds { .. } => "workflow_symbol_out_of_bounds",
        BudgetError::AccessorPathTooDeep { .. } => "workflow_accessor_path_too_deep",
        BudgetError::JumpCycle { .. } => "workflow_jump_cycle",
        BudgetError::InvalidCompiledWorkflow { .. } => "invalid_compiled_workflow",
    }
}
