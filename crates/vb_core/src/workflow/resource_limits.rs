#![forbid(unsafe_code)]
//! Resource contract limit validation (declared versus hard limit).

use super::{ContractResource, ExprProgram, ResourceContract, WorkflowError, WorkflowParts};

pub(super) fn validate_reduce_limit(limit: u32) -> Result<(), WorkflowError> {
    let limit = usize::try_from(limit).map_err(|_| WorkflowError::ResourceContractExceeded {
        resource: ContractResource::ListItems,
    })?;
    if limit > crate::limits::MAX_LIST_ITEMS_PER_VALUE {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ListItems,
        });
    }
    Ok(())
}
use crate::limits::{
    MAX_ACCESSORS, MAX_BLOB_BYTES, MAX_COLLECT_ITEMS, MAX_CONSTANTS, MAX_EXPRESSION_STACK,
    MAX_EXPRESSIONS, MAX_FANOUT, MAX_INPUT_BYTES, MAX_IPC_PAYLOAD_BYTES, MAX_JOURNAL_BATCH_BYTES,
    MAX_OUTPUT_BYTES, MAX_QUEUE_DEPTH, MAX_RETRY_ATTEMPTS, MAX_SLOTS_PER_WORKFLOW, MAX_STEP_BUDGET,
    MAX_STEPS_PER_WORKFLOW,
};

/// Validates every declared resource-contract limit against its hard limit.
///
/// This is the single canonical implementation of the declared-vs-hard-limit
/// (`TooLarge`) rule shared by admission (`validate_resource_contract`, below)
/// and the public engine pre-check
/// (`engine::validate::validate_resource_contract`). Checks run in
/// `ResourceContract` field order and report the first over-limit dimension.
/// Actual-usage (`Exceeded`) checks stay with the callers: admission applies
/// them after this gate, while the engine pre-check is bounds-only by design.
pub(crate) fn check_resource_contract_limits(
    contract: &ResourceContract,
) -> Result<(), WorkflowError> {
    if usize::from(contract.max_steps) > MAX_STEPS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Steps,
        });
    }
    if usize::from(contract.max_slots) > MAX_SLOTS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Slots,
        });
    }
    if usize::from(contract.max_constants) > MAX_CONSTANTS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Constants,
        });
    }
    if usize::from(contract.max_accessors) > MAX_ACCESSORS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Accessors,
        });
    }
    if usize::from(contract.max_expressions) > MAX_EXPRESSIONS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Expressions,
        });
    }
    if contract.max_expr_stack > MAX_EXPRESSION_STACK {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::ExprStack,
        });
    }
    if contract.max_step_budget_per_tick > MAX_STEP_BUDGET {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::StepBudgetPerTick,
        });
    }
    if contract.max_transitions_per_tick > MAX_STEP_BUDGET {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::TransitionsPerTick,
        });
    }
    if contract.max_input_bytes > MAX_INPUT_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::InputBytes,
        });
    }
    if contract.max_output_bytes > MAX_OUTPUT_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::OutputBytes,
        });
    }
    if contract.max_blob_bytes > MAX_BLOB_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::BlobBytes,
        });
    }
    if contract.max_ipc_payload_bytes > MAX_IPC_PAYLOAD_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::IpcPayloadBytes,
        });
    }
    if contract.max_retry_attempts > MAX_RETRY_ATTEMPTS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::RetryAttempts,
        });
    }
    if contract.max_fanout > MAX_FANOUT {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Fanout,
        });
    }
    if contract.max_collect_items > MAX_COLLECT_ITEMS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::CollectItems,
        });
    }
    if contract.max_queue_depth > MAX_QUEUE_DEPTH {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::QueueDepth,
        });
    }
    if contract.max_journal_batch_bytes > MAX_JOURNAL_BATCH_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::JournalBatchBytes,
        });
    }
    Ok(())
}

pub(super) fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    check_resource_contract_limits(&contract)?;
    validate_resource_usage(parts, contract)?;
    validate_expr_stack_usage(parts.expressions.as_ref(), contract.max_expr_stack)?;
    validate_transitions_nonzero(contract.max_transitions_per_tick)
}

/// Rejects a zero `max_transitions_per_tick` declaration.
///
/// The over-limit half of the old transitions check now lives in the
/// canonical [`check_resource_contract_limits`] gate above; only the
/// admission-only non-zero rule remains here.
fn validate_transitions_nonzero(max_transitions_per_tick: u64) -> Result<(), WorkflowError> {
    if max_transitions_per_tick == 0 {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::TransitionsPerTick,
        });
    }
    Ok(())
}

/// Validates actual resource usage against the declared contract.
///
/// Runs after the canonical [`check_resource_contract_limits`] gate, so every
/// `declared` value handled here is already known to sit within its hard
/// limit; only the actual-vs-declared (`Exceeded`) direction remains.
fn validate_resource_usage(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    check_actual_within_declared(
        ContractResource::Steps,
        parts.nodes.len(),
        usize::from(contract.max_steps),
    )?;
    check_actual_within_declared(
        ContractResource::Slots,
        usize::from(parts.slot_count),
        usize::from(contract.max_slots),
    )?;
    check_actual_within_declared(
        ContractResource::Constants,
        parts.constants.len(),
        usize::from(contract.max_constants),
    )?;
    check_actual_within_declared(
        ContractResource::Accessors,
        parts.accessors.len(),
        usize::from(contract.max_accessors),
    )?;
    check_actual_within_declared(
        ContractResource::Expressions,
        parts.expressions.len(),
        usize::from(contract.max_expressions),
    )
}

fn check_actual_within_declared(
    resource: ContractResource,
    actual: usize,
    declared: usize,
) -> Result<(), WorkflowError> {
    if actual > declared {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}

fn validate_expr_stack_usage(
    expressions: &[ExprProgram],
    max_expr_stack: u8,
) -> Result<(), WorkflowError> {
    if expressions
        .iter()
        .any(|expression| expression.max_stack > max_expr_stack)
    {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ExprStack,
        })
    } else {
        Ok(())
    }
}
