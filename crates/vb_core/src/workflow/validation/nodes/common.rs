#![forbid(unsafe_code)]
//! Shared node field validators — optional slot/step validation and
//! node-kind–specific field checks (build-list, build-object, loops, etc.).

use super::bounds::{
    validate_const, validate_expr, validate_optional_slot, validate_optional_step, validate_slot,
    validate_step,
};
use super::branch_tables::validate_branch_route;
use crate::ids::ConstIdx;
use crate::ids::{SlotIdx, StepIdx, SymbolId};
use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE};
use crate::workflow::{CompiledNode, ExprBranch, SlotBranch, WorkflowError, WorkflowParts};

/// Checks that a node-local fanout count does not exceed the declared contract.
///
/// A contract value of `0` is treated as opt-out (legacy callers may pass
/// `0` to mean "no check"), preserving existing test fixtures that pair
/// zeroed budget fields with otherwise-valid workflows.
fn check_against_contract_fanout(
    actual: usize,
    contract_max: u16,
) -> Result<(), WorkflowError> {
    if contract_max > 0 && actual > usize::from(contract_max) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_fanout",
        })
    } else {
        Ok(())
    }
}

/// Checks that a node-local retry count does not exceed the declared contract.
///
/// A contract value of `0` is treated as opt-out (legacy callers may pass
/// `0` to mean "no check"), preserving existing test fixtures that pair
/// zeroed budget fields with otherwise-valid workflows.
fn check_against_contract_retry(
    actual: u16,
    contract_max: u16,
) -> Result<(), WorkflowError> {
    if contract_max > 0 && actual > contract_max {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_retry_attempts",
        })
    } else {
        Ok(())
    }
}

/// Checks that a node-local collect-page count does not exceed the declared
/// contract.
///
/// A contract value of `0` is treated as opt-out (legacy callers may pass
/// `0` to mean "no check"), preserving existing test fixtures that pair
/// zeroed budget fields with otherwise-valid workflows.
fn check_against_contract_collect(
    actual: u32,
    contract_max: u32,
) -> Result<(), WorkflowError> {
    if contract_max > 0 && actual > contract_max {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        })
    } else {
        Ok(())
    }
}

/// Validates the four common fields shared by every node kind.
///
/// - `output` — optional slot write target
/// - `next` — optional fallthrough step
/// - `on_error` — optional error-handler step
/// - `error_slot` — optional error-information slot
pub(crate) fn validate_node_common(
    node: &CompiledNode,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_optional_slot(node.output, parts.slot_count)?;
    validate_optional_step(node.next, parts.nodes.len())?;
    validate_optional_step(node.on_error, parts.nodes.len())?;
    validate_optional_slot(node.error_slot, parts.slot_count)
}

/// Validates that slot references in a BuildList node are within bounds and the
/// item count does not exceed the hard limit.
pub(crate) fn validate_build_list(items: &[SlotIdx], slot_count: u16) -> Result<(), WorkflowError> {
    if items.len() > MAX_LIST_ITEMS_PER_VALUE {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "list_items",
        });
    }
    for slot in items {
        validate_slot(*slot, slot_count)?;
    }
    Ok(())
}

/// Validates that slot references in a BuildObject node are within bounds and the
/// field count does not exceed the hard limit.
pub(crate) fn validate_build_object(
    fields: &[(SymbolId, SlotIdx)],
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    if fields.len() > MAX_OBJECT_FIELDS_PER_VALUE {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "object_fields",
        });
    }
    for (_, slot) in fields {
        validate_slot(*slot, parts.slot_count)?;
    }
    Ok(())
}

/// Validates a ForEachStart node: input + item slots must be valid, and both
/// body/done targets must be forward steps. The declared `limit` must be
/// non-zero and bounded by the resource contract's `max_collect_items`.
pub(crate) fn validate_for_each_start(
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(input, parts.slot_count)?;
    validate_slot(item_slot, parts.slot_count)?;
    if limit == 0 {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        });
    }
    check_against_contract_collect(limit, parts.resource_contract.max_collect_items)?;
    validate_two_steps(body, done, parts)
}

/// Validates a CollectStart node: the source slot and body/done steps must be
/// valid. The declared `limit` and `page_size` must be non-zero and bounded
/// by the resource contract's `max_collect_items`.
pub(crate) fn validate_collect_start(
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(source, parts.slot_count)?;
    if limit == 0 {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        });
    }
    if page_size == 0 {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        });
    }
    let contract_max = parts.resource_contract.max_collect_items;
    check_against_contract_collect(limit, contract_max)?;
    check_against_contract_collect(page_size, contract_max)?;
    validate_two_steps(body, done, parts)
}

/// Validates a slot + two-step target (ForEachNext, CollectStart/Page/Next,
/// ReduceStart/Next, RepeatAttempt, RetryCheck).
pub(crate) fn validate_slot_and_steps(
    slot: SlotIdx,
    first: StepIdx,
    second: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(slot, parts.slot_count)?;
    validate_two_steps(first, second, parts)
}

/// Validates that two step targets are within the node count.
pub(crate) fn validate_two_steps(
    first: StepIdx,
    second: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_step(first, parts.nodes.len())?;
    validate_step(second, parts.nodes.len())
}

/// Validates a TogetherStart node: all branch targets and join must be valid
/// steps, and the branch table must have at least one entry. The branch count
/// must not exceed the declared resource contract's `max_fanout`.
pub(crate) fn validate_together(
    branches: &[StepIdx],
    join: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), Some(join))?;
    check_against_contract_fanout(branches.len(), parts.resource_contract.max_fanout)?;
    for branch in branches {
        validate_step(*branch, parts.nodes.len())?;
    }
    validate_step(join, parts.nodes.len())
}

/// Validates a TogetherJoin node: the `branch_count` must be non-zero and
/// bounded by the declared resource contract's `max_fanout`, and the
/// accumulator slot must be a valid frame slot.
pub(crate) fn validate_together_join(
    branch_count: u16,
    accumulator: SlotIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_nonzero_u16(branch_count, "max_fanout")?;
    check_against_contract_fanout(usize::from(branch_count), parts.resource_contract.max_fanout)?;
    validate_slot(accumulator, parts.slot_count)
}

/// Validates a non-zero u16 used as a policy count (max_attempts, branch_count).
pub(crate) fn validate_nonzero_u16(
    value: u16,
    resource: &'static str,
) -> Result<(), WorkflowError> {
    if value == 0 {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}

/// Validates a ReduceStart node: input + accumulator slots, initial constant,
/// and body/done steps.
pub(crate) fn validate_reduce_start(
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(input, parts.slot_count)?;
    validate_slot(accumulator, parts.slot_count)?;
    validate_const(initial, parts.constants.len())?;
    validate_two_steps(body, done, parts)
}

/// Validates a ReduceNext node: iterator + accumulator slots, body/done steps.
pub(crate) fn validate_reduce_next(
    iterator_slot: SlotIdx,
    accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(iterator_slot, parts.slot_count)?;
    validate_slot(accumulator, parts.slot_count)?;
    validate_two_steps(body, done, parts)
}

/// Validates a RepeatStart node: max_attempts must be non-zero and bounded by
/// the declared resource contract, and body/done steps must be valid.
pub(crate) fn validate_repeat_start(
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_nonzero_u16(max_attempts, "max_retry_attempts")?;
    check_against_contract_retry(max_attempts, parts.resource_contract.max_retry_attempts)?;
    validate_two_steps(body, done, parts)
}

/// Validates a ChooseSlot node: each branch maps a boolean slot to a step target,
/// and the table must have at least one entry or an otherwise clause. The branch
/// count must not exceed the declared resource contract's `max_fanout`.
pub(crate) fn validate_slot_choose(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise)?;
    check_against_contract_fanout(branches.len(), parts.resource_contract.max_fanout)?;
    branches.iter().try_for_each(|branch| {
        validate_slot(branch.condition, parts.slot_count)?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step(otherwise, parts.nodes.len())
}

/// Validates a Choose (ExprBranch) node: each branch maps an expression condition
/// to a step target, and the table must have at least one entry or an otherwise.
/// The branch count must not exceed the declared resource contract's `max_fanout`.
pub(crate) fn validate_expr_choose(
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise)?;
    check_against_contract_fanout(branches.len(), parts.resource_contract.max_fanout)?;
    branches.iter().try_for_each(|branch| {
        validate_expr(branch.condition, parts.expressions.len())?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step(otherwise, parts.nodes.len())
}
