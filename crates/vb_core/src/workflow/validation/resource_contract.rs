#![forbid(unsafe_code)]
//! Resource contract validation.
//!
//! Validates that declared resource contract bounds are sane and that
//! actual counts stay within those declared bounds.

use crate::limits::{
    MAX_ACCESSORS, MAX_CONSTANTS, MAX_EXPRESSION_STACK, MAX_EXPRESSIONS, MAX_SLOTS_PER_WORKFLOW,
    MAX_STEPS_PER_WORKFLOW,
};
use crate::workflow::{ExprProgram, ResourceContract, WorkflowError, WorkflowParts};

/// Validates resource contract bounds against hard limits.
///
/// CANONICAL HOME for `validate_resource_contract` — re-exported via
/// `engine.rs` as `vb_core::validate_resource_contract`.
pub fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    validate_resource_counts(parts, contract)?;
    validate_expr_stack_contract(parts.expressions.as_ref(), contract.max_expr_stack)?;
    validate_transitions_per_tick(contract.max_transitions_per_tick)
}

/// Validates that primary resource counts (steps, slots, constants) are
/// within both the declared contract and the hard protocol limits.
fn validate_primary_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_contract_limit(
        "max_steps",
        parts.nodes.len(),
        usize::from(contract.max_steps),
        MAX_STEPS_PER_WORKFLOW,
    )?;
    validate_contract_limit(
        "max_slots",
        usize::from(parts.slot_count),
        usize::from(contract.max_slots),
        MAX_SLOTS_PER_WORKFLOW,
    )?;
    validate_contract_limit(
        "max_constants",
        parts.constants.len(),
        usize::from(contract.max_constants),
        MAX_CONSTANTS,
    )
}

/// Validates that expression-level resource counts (accessors, expressions)
/// are within both the declared contract and the hard protocol limits.
fn validate_expression_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_contract_limit(
        "max_accessors",
        parts.accessors.len(),
        usize::from(contract.max_accessors),
        MAX_ACCESSORS,
    )?;
    validate_contract_limit(
        "max_expressions",
        parts.expressions.len(),
        usize::from(contract.max_expressions),
        MAX_EXPRESSIONS,
    )
}

/// Validates all resource counts in one place.
fn validate_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_primary_resource_counts(parts, contract)?;
    validate_expression_resource_counts(parts, contract)
}

/// Validates a single declared-vs-actual-vs-hard-limit constraint.
fn validate_contract_limit(
    resource: &'static str,
    actual: usize,
    declared: usize,
    hard_limit: usize,
) -> Result<(), WorkflowError> {
    if declared > hard_limit {
        return Err(WorkflowError::ResourceContractTooLarge { resource });
    }
    if actual > declared {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}

/// Validates the expression stack depth contract.
fn validate_expr_stack_contract(
    expressions: &[ExprProgram],
    max_expr_stack: u8,
) -> Result<(), WorkflowError> {
    if max_expr_stack > MAX_EXPRESSION_STACK {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_expr_stack",
        });
    }
    if expressions
        .iter()
        .any(|expression| expression.max_stack > max_expr_stack)
    {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_expr_stack",
        })
    } else {
        Ok(())
    }
}

/// Validates that `max_transitions_per_tick` is within acceptable bounds.
/// Must be at least 1 (non-zero) and must not exceed the protocol hard limit.
fn validate_transitions_per_tick(max_transitions_per_tick: u64) -> Result<(), WorkflowError> {
    use crate::limits::MAX_STEP_BUDGET;
    if max_transitions_per_tick == 0 {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "max_transitions_per_tick",
        });
    }
    if max_transitions_per_tick > MAX_STEP_BUDGET {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_transitions_per_tick",
        });
    }
    Ok(())
}
