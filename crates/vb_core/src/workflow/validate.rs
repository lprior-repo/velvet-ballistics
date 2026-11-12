#![forbid(unsafe_code)]
//! Workflow parts validation: entry, nodes, inputs and budget.

use super::forward_edges::validate_forward_edges;
use super::repeat_counter_ownership;
use super::resource_limits::validate_resource_contract;
use super::traversal::validate_reachability;
use super::validate_node_kind::validate_node_kind;
use super::validate_refs::{
    validate_accessor_paths, validate_accessors, validate_build_object_symbols,
    validate_constants_symbols, validate_expressions, validate_optional_slot,
    validate_optional_step, validate_step,
};
use super::{
    CompiledInputConstraint, CompiledNode, CompiledNodeKind, ContractResource,
    InputConstraintIssue, WorkflowError, WorkflowParts,
};
use crate::ids::{SlotIdx, StepIdx};

pub(super) fn validate_parts(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    if parts.nodes.is_empty() {
        return Err(WorkflowError::EmptyNodes);
    }
    validate_resource_contract(parts)?;
    validate_entry(parts.entry, parts.nodes.len())?;
    validate_expressions(&parts.expressions, parts.accessors.len())?;
    validate_accessors(&parts.accessors, parts.slot_count)?;
    for (index, node) in parts.nodes.iter().enumerate() {
        validate_node_id(node, index)?;
        validate_node(node, parts)?;
    }
    validate_accessor_paths(&parts.accessors, parts.symbols_count)?;
    validate_constants_symbols(&parts.constants, parts.symbols_count)?;
    validate_build_object_symbols(&parts.nodes, parts.symbols_count)?;
    validate_reachability(parts)?;
    validate_forward_edges(parts)?;
    repeat_counter_ownership::validate(parts)?;
    // RE-001: reject nested `TogetherStart`. Walked inline here (the
    // helper lives in `crate::engine::validate` to avoid an
    // engine->workflow dependency cycle) so `try_from_parts` rejects
    // the shape at the source. The helper returns
    // `WorkflowError::NestedTogether { outer, inner }`.
    crate::engine::validate::validate_no_nested_together(parts)?;
    Ok(())
}

pub(super) fn validate_input_slots(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let slot_count = parts.slot_count;
    let mut prev_slot: Option<SlotIdx> = None;
    for input_slot in parts.input_slots.as_ref() {
        let current = input_slot.slot;
        let current_value = current.get();
        if current_value >= slot_count {
            return Err(WorkflowError::InputSlotOutOfBounds {
                slot: current,
                kind: input_slot.kind,
            });
        }
        if let Some(prev) = prev_slot {
            let prev_value = prev.get();
            if current_value == prev_value {
                return Err(WorkflowError::DuplicateInputSlot {
                    slot: current,
                    kind: input_slot.kind,
                });
            }
            if current_value < prev_value {
                return Err(WorkflowError::NonCanonicalInputSlotOrder {
                    prev_slot: prev,
                    curr_slot: current,
                });
            }
        }
        validate_input_constraint(current, &input_slot.constraint)?;
        prev_slot = Some(current);
    }
    Ok(())
}

fn validate_input_constraint(
    slot: SlotIdx,
    constraint: &CompiledInputConstraint,
) -> Result<(), WorkflowError> {
    let inconsistent =
        |issue: InputConstraintIssue| WorkflowError::InconsistentInputConstraint { slot, issue };
    if constraint.value_bound().inconsistency() {
        return Err(inconsistent(InputConstraintIssue::MinExceedsMax));
    }
    if constraint.length_bound().inconsistency() {
        return Err(inconsistent(
            InputConstraintIssue::MinLengthExceedsMaxLength,
        ));
    }
    if constraint
        .length_bound()
        .lower()
        .is_some_and(|bound| bound < 0)
        || constraint
            .length_bound()
            .upper()
            .is_some_and(|bound| bound < 0)
    {
        return Err(inconsistent(InputConstraintIssue::NegativeLengthBound));
    }
    Ok(())
}

pub(super) fn validate_budget(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    use crate::budget::{BoundednessPolicy, WholeWorkflowBudget};

    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;

    validate_budget_result(BoundednessPolicy::DEFAULT.validate(&budget))
}

pub(crate) fn validate_budget_result(
    result: Result<(), crate::budget::BudgetError>,
) -> Result<(), WorkflowError> {
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: error.dimension(),
        }),
    }
}

fn validate_node_id(node: &CompiledNode, index: usize) -> Result<(), WorkflowError> {
    if node.id.as_usize() == index {
        Ok(())
    } else {
        Err(WorkflowError::NodeIdMismatch {
            expected: StepIdx::new(u16::try_from(index).map_err(|_| {
                WorkflowError::ResourceContractExceeded {
                    resource: ContractResource::Steps,
                }
            })?),
            actual: node.id,
        })
    }
}

fn validate_entry(entry: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    validate_step(entry, node_count).map_err(|_| WorkflowError::EntryOutOfBounds { entry })
}

fn validate_node(node: &CompiledNode, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    validate_node_common(node, parts)?;
    validate_node_kind(&node.kind, parts)?;
    validate_reduce_branch_targets(node)?;
    validate_for_each_next_targets(node)
}

fn validate_reduce_branch_targets(node: &CompiledNode) -> Result<(), WorkflowError> {
    match &node.kind {
        CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
            if body == done =>
        {
            Err(WorkflowError::ReduceBranchTargetAlias {
                step: node.id,
                target: *body,
            })
        }
        _ => Ok(()),
    }
}

fn validate_for_each_next_targets(node: &CompiledNode) -> Result<(), WorkflowError> {
    match &node.kind {
        CompiledNodeKind::ForEachNext { body, done, .. } if body == done => {
            Err(WorkflowError::ForEachBranchTargetAlias {
                step: node.id,
                target: *body,
            })
        }
        _ => Ok(()),
    }
}

fn validate_node_common(node: &CompiledNode, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    validate_optional_slot(node.output.slot(), parts.slot_count)?;
    validate_optional_step(node.routing.next.step(), parts.nodes.len())?;
    validate_optional_step(node.routing.error.handler(), parts.nodes.len())?;
    validate_optional_slot(node.routing.error.error_slot(), parts.slot_count)
}
