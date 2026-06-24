#![forbid(unsafe_code)]
//! Resource limit validation for workflow documents.

#![allow(unreachable_pub)]
//!
//! Validates that a workflow's declared resource contract stays within
//! protocol hard limits.

use crate::{ValidationError, ValidationResult};

use crate::type_sigs::{ResourceLimits, WorkflowTypes};

/// Validates resource contract bounds against protocol hard limits.
pub fn validate_resource_limits(
    workflow: &WorkflowTypes,
    hard_limits: &ResourceLimits,
) -> ValidationResult<()> {
    check_resource_bound(
        "max_steps",
        workflow.steps.len(),
        workflow.resource_contract.max_steps,
        hard_limits.max_steps,
    )?;
    // `WorkflowTypes` does not carry the actual slot count, so we can only
    // check the declared bound against the protocol hard limit. Using
    // `workflow.steps.len()` here would be a copy-paste error (it is the
    // step count, not the slot count) and would produce false-positive
    // `LimitExceeded` errors whenever steps.len() > max_slots.
    check_declared_bound(
        "max_slots",
        workflow.resource_contract.max_slots,
        hard_limits.max_slots,
    )?;
    // `WorkflowTypes` does not carry the actual constants count, so the
    // actual-vs-declared check is not possible. Passing `0` here would make
    // the `actual > declared` check always false, silently masking any
    // actual-exceeds-declared violation. The declared bound check is the
    // strongest check we can perform on this struct.
    check_declared_bound(
        "max_constants",
        workflow.resource_contract.max_constants,
        hard_limits.max_constants,
    )?;
    check_declared_bound(
        "max_accessors",
        workflow.resource_contract.max_accessors,
        hard_limits.max_accessors,
    )?;
    check_declared_bound(
        "max_expressions",
        workflow.resource_contract.max_expressions,
        hard_limits.max_expressions,
    )?;
    check_declared_bound(
        "max_expr_stack",
        workflow.resource_contract.max_expr_stack,
        hard_limits.max_expr_stack,
    )?;
    check_declared_bound(
        "max_step_budget_per_tick",
        workflow.resource_contract.max_step_budget_per_tick,
        hard_limits.max_step_budget_per_tick,
    )?;
    check_declared_bound(
        "max_input_bytes",
        workflow.resource_contract.max_input_bytes,
        hard_limits.max_input_bytes,
    )?;
    check_declared_bound(
        "max_output_bytes",
        workflow.resource_contract.max_output_bytes,
        hard_limits.max_output_bytes,
    )?;
    check_declared_bound(
        "max_blob_bytes",
        workflow.resource_contract.max_blob_bytes,
        hard_limits.max_blob_bytes,
    )?;
    check_declared_bound(
        "max_ipc_payload_bytes",
        workflow.resource_contract.max_ipc_payload_bytes,
        hard_limits.max_ipc_payload_bytes,
    )?;
    check_declared_bound(
        "max_retry_attempts",
        workflow.resource_contract.max_retry_attempts,
        hard_limits.max_retry_attempts,
    )?;
    check_declared_bound(
        "max_fanout",
        workflow.resource_contract.max_fanout,
        hard_limits.max_fanout,
    )?;
    check_declared_bound(
        "max_collect_items",
        workflow.resource_contract.max_collect_items,
        hard_limits.max_collect_items,
    )?;
    check_declared_bound(
        "max_queue_depth",
        workflow.resource_contract.max_queue_depth,
        hard_limits.max_queue_depth,
    )?;
    check_declared_bound(
        "max_journal_batch_bytes",
        workflow.resource_contract.max_journal_batch_bytes,
        hard_limits.max_journal_batch_bytes,
    )
}

fn check_resource_bound(
    resource: &str,
    actual: usize,
    declared: usize,
    hard_limit: usize,
) -> ValidationResult<()> {
    check_declared_bound(resource, declared, hard_limit)?;
    if actual > declared {
        return Err(ValidationError::LimitExceeded {
            resource: resource.to_owned(),
        });
    }
    Ok(())
}

fn check_declared_bound(
    resource: &str,
    declared: usize,
    hard_limit: usize,
) -> ValidationResult<()> {
    if declared == 0 {
        return Err(ValidationError::LimitRequired {
            resource: resource.to_owned(),
        });
    }
    if declared > hard_limit {
        return Err(ValidationError::LimitExceeded {
            resource: resource.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[cfg(test)]
#[path = "secret_leak/tests.rs"]
mod tests;
