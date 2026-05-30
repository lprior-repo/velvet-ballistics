#![forbid(unsafe_code)]
//! Workflow validation - resource contract validation.

use crate::errors::CoreError;
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx};
use crate::limits::{
    MAX_ACCESSORS, MAX_BLOB_BYTES, MAX_COLLECT_ITEMS, MAX_CONSTANTS, MAX_EXPRESSIONS,
    MAX_FANOUT, MAX_INPUT_BYTES, MAX_IPC_PAYLOAD_BYTES, MAX_JOURNAL_BATCH_BYTES,
    MAX_OUTPUT_BYTES, MAX_QUEUE_DEPTH, MAX_RETRY_ATTEMPTS, MAX_SLOTS_PER_WORKFLOW,
    MAX_STEP_BUDGET, MAX_STEPS_PER_WORKFLOW,
};

use crate::accessors::AccessorProgram;
use crate::workflow::ResourceContract;
use crate::expressions::ExprProgram;
use crate::workflow::WorkflowParts;
use crate::validation::WorkflowError;

pub(crate) fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    validate_resource_counts(parts, contract)?;
    validate_expr_stack_contract(parts.expressions.as_ref(), contract.max_expr_stack)?;
    validate_nonzero_u64("max_transitions_per_tick", contract.max_transitions_per_tick, MAX_STEP_BUDGET)?;
    validate_nonzero_u64("max_step_budget_per_tick", contract.max_step_budget_per_tick, MAX_STEP_BUDGET)?;
    validate_nonzero_u32("max_input_bytes", contract.max_input_bytes, MAX_INPUT_BYTES)?;
    validate_nonzero_u32("max_output_bytes", contract.max_output_bytes, MAX_OUTPUT_BYTES)?;
    validate_nonzero_u64("max_blob_bytes", contract.max_blob_bytes, MAX_BLOB_BYTES)?;
    validate_nonzero_u32("max_ipc_payload_bytes", contract.max_ipc_payload_bytes, MAX_IPC_PAYLOAD_BYTES)?;
    validate_nonzero_u32("max_retry_attempts", u32::from(contract.max_retry_attempts), u32::from(MAX_RETRY_ATTEMPTS))?;
    validate_nonzero_u32("max_fanout", u32::from(contract.max_fanout), u32::from(MAX_FANOUT))?;
    validate_nonzero_u32("max_collect_items", contract.max_collect_items, MAX_COLLECT_ITEMS)?;
    validate_nonzero_u32("max_queue_depth", contract.max_queue_depth, MAX_QUEUE_DEPTH)?;
    validate_nonzero_u32("max_journal_batch_bytes", contract.max_journal_batch_bytes, MAX_JOURNAL_BATCH_BYTES)?;
    Ok(())
}

fn validate_nonzero_u32(resource: &'static str, declared: u32, hard_limit: u32) -> Result<(), WorkflowError> {
    if declared == 0 {
        return Err(WorkflowError::ResourceContractExceeded { resource });
    }
    if declared > hard_limit {
        return Err(WorkflowError::ResourceContractTooLarge { resource });
    }
    Ok(())
}

fn validate_nonzero_u64(resource: &'static str, declared: u64, hard_limit: u64) -> Result<(), WorkflowError> {
    if declared == 0 {
        return Err(WorkflowError::ResourceContractExceeded { resource });
    }
    if declared > hard_limit {
        return Err(WorkflowError::ResourceContractTooLarge { resource });
    }
    Ok(())
}

pub(crate) fn validate_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_primary_resource_counts(parts, contract)?;
    validate_expression_resource_counts(parts, contract)
}

pub(crate) fn validate_primary_resource_counts(
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

pub(crate) fn validate_expression_resource_counts(
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

pub(crate) fn validate_contract_limit(
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

pub(crate) fn validate_expr_stack_contract(
    expressions: &[ExprProgram],
    max_expr_stack: u8,
) -> Result<(), WorkflowError> {
    if max_expr_stack > crate::limits::MAX_EXPRESSION_STACK {
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

pub(crate) fn validate_expressions(
    expressions: &[ExprProgram],
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for expression in expressions {
        ExprProgram::try_from_parts(expression.ops.clone(), expression.max_stack)?;
        validate_expression_accessors(expression, accessor_count)?;
    }
    Ok(())
}

fn validate_expression_accessors(
    expression: &ExprProgram,
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for op in expression.ops.as_ref() {
        if let crate::expressions::ExprOp::LoadAccessor(accessor) = op {
            validate_accessor(*accessor, accessor_count)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_accessors(accessors: &[AccessorProgram], slot_count: u16) -> Result<(), WorkflowError> {
    for accessor in accessors {
        validate_slot(accessor.root, slot_count)?;
    }
    Ok(())
}

fn validate_accessor(accessor: AccessorIdx, accessor_count: usize) -> Result<(), WorkflowError> {
    if accessor.as_usize() < accessor_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(
            CoreError::InvalidCompiledWorkflow {
                reason: "accessor index out of bounds",
            },
        ))
    }
}

/// Validates that accessor path index segments do not use the reserved u32::MAX value.
pub(crate) fn validate_accessor_path_symbols(accessors: &[AccessorProgram]) -> Result<(), WorkflowError> {
    for accessor in accessors {
        for segment in accessor.path.as_ref() {
            if let crate::accessors::PathSegment::Index(index) = *segment && index == u32::MAX {
                return Err(WorkflowError::Expression(
                    CoreError::InvalidCompiledWorkflow {
                        reason: "accessor path index uses reserved value u32::MAX",
                    },
                ));
            }
        }
    }
    Ok(())
}

// Primitive validation helpers

pub(crate) fn validate_entry(entry: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    validate_step(entry, node_count).map_err(|_| WorkflowError::EntryOutOfBounds { entry })
}

pub(crate) fn validate_expr(expr: ExprIdx, expression_count: usize) -> Result<(), WorkflowError> {
    if expr.as_usize() < expression_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(CoreError::ExprOutOfBounds {
            expr,
        }))
    }
}

pub(crate) fn validate_step(step: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    if step.as_usize() < node_count {
        Ok(())
    } else {
        Err(WorkflowError::StepOutOfBounds { step })
    }
}

pub(crate) fn validate_slot(slot: SlotIdx, slot_count: u16) -> Result<(), WorkflowError> {
    if slot.as_usize() < usize::from(slot_count) {
        Ok(())
    } else {
        Err(WorkflowError::SlotOutOfBounds { slot })
    }
}

pub(crate) fn validate_const(constant: ConstIdx, const_count: usize) -> Result<(), WorkflowError> {
    if constant.as_usize() < const_count {
        Ok(())
    } else {
        Err(WorkflowError::ConstOutOfBounds { constant })
    }
}

pub(crate) fn validate_optional_slot(slot: Option<SlotIdx>, slot_count: u16) -> Result<(), WorkflowError> {
    slot.map_or(Ok(()), |value| validate_slot(value, slot_count))
}

pub(crate) fn validate_optional_step(step: Option<StepIdx>, node_count: usize) -> Result<(), WorkflowError> {
    step.map_or(Ok(()), |target| validate_step(target, node_count))
}
