#![forbid(unsafe_code)]

use super::{CompiledNode, CompiledNodeKind, WorkflowError, WorkflowParts};
use crate::ids::{SlotIdx, StepIdx};

pub(super) fn validate(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let counter_owners = identify_counter_owners(parts)?;
    for (slot, owner) in counter_owners {
        validate_slot_ownership(parts, slot, owner)?;
    }
    Ok(())
}

fn identify_counter_owners(
    parts: &WorkflowParts,
) -> Result<Vec<(SlotIdx, StepIdx)>, WorkflowError> {
    let mut owners = Vec::new();
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::RepeatStart { .. } = node.kind
            && let Some(slot) = node.output
        {
            for (existing_slot, existing_owner) in &owners {
                if *existing_slot == slot {
                    return Err(WorkflowError::RepeatCounterForeignWrite {
                        owner: *existing_owner,
                        writer: node.id,
                        slot,
                    });
                }
            }
            owners.push((slot, node.id));
        }
    }
    Ok(owners)
}

fn validate_slot_ownership(
    parts: &WorkflowParts,
    slot: SlotIdx,
    owner: StepIdx,
) -> Result<(), WorkflowError> {
    for node in parts.nodes.iter() {
        if node.id == owner {
            if node.error_slot == Some(slot) {
                return Err(WorkflowError::RepeatCounterForeignWrite {
                    owner,
                    writer: node.id,
                    slot,
                });
            }
            continue;
        }
        if is_paired_check(parts, node, slot, owner) {
            if node.error_slot == Some(slot) {
                return Err(WorkflowError::RepeatCounterForeignWrite {
                    owner,
                    writer: node.id,
                    slot,
                });
            }
            continue;
        }
        if writes_slot(node, slot) {
            return Err(WorkflowError::RepeatCounterForeignWrite {
                owner,
                writer: node.id,
                slot,
            });
        }
    }
    Ok(())
}

fn is_paired_check(
    parts: &WorkflowParts,
    node: &CompiledNode,
    slot: SlotIdx,
    owner: StepIdx,
) -> bool {
    let CompiledNodeKind::RepeatCheck {
        attempt_slot,
        done: check_done,
    } = &node.kind
    else {
        return false;
    };
    if *attempt_slot != slot {
        return false;
    }
    let owner_node = parts.nodes.get(owner.as_usize());
    let Some(owner_node) = owner_node else {
        return false;
    };
    let CompiledNodeKind::RepeatStart {
        body,
        done: start_done,
        ..
    } = &owner_node.kind
    else {
        return false;
    };
    if *check_done != *start_done {
        return false;
    }
    let body_u = body.as_usize();
    let check_u = node.id.as_usize();
    let done_u = start_done.as_usize();
    body_u < check_u && check_u < done_u && node.next == Some(*body)
}

fn writes_slot(node: &CompiledNode, slot: SlotIdx) -> bool {
    if node.output == Some(slot) {
        return true;
    }
    if node.error_slot == Some(slot) {
        return true;
    }
    match &node.kind {
        CompiledNodeKind::ForEachStart { item_slot, .. } => *item_slot == slot,
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => *iterator_slot == slot,
        CompiledNodeKind::TogetherBranch { accumulator, .. }
        | CompiledNodeKind::TogetherJoin { accumulator, .. } => *accumulator == slot,
        CompiledNodeKind::CollectStart { source, .. } => node.output.is_none() && *source == slot,
        CompiledNodeKind::CollectNext { collector_slot, .. } => *collector_slot == slot,
        CompiledNodeKind::ReduceStart {
            input, accumulator, ..
        } => *input == slot || *accumulator == slot,
        CompiledNodeKind::ReduceNext { iterator_slot, .. } => *iterator_slot == slot,
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => *attempt_slot == slot,
        CompiledNodeKind::RetryCheck { policy_slot, .. } => *policy_slot == slot,
        CompiledNodeKind::ErrorHandler { error_slot, .. } => *error_slot == Some(slot),
        CompiledNodeKind::AskResume { answer } => *answer == slot,
        _ => false,
    }
}
