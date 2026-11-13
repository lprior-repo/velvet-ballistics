use super::resource_limits::validate_reduce_limit;
use super::validate_refs::{validate_slot, validate_step};
use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNode, CompiledNodeKind as K, ContractResource, WorkflowError, WorkflowParts,
};

pub(super) fn validate(parts: &WorkflowParts, src: &CompiledNode) -> Result<(), WorkflowError> {
    match &src.kind {
        K::ForEachConcurrentStart {
            input,
            item_slot,
            limit,
            at_once,
            body,
            join,
        } => {
            validate_foreach_input(parts, *input, *item_slot, limit.get(), at_once.get())?;
            validate_concurrent_entries(parts, src, std::slice::from_ref(body), *join)
        }
        K::TogetherConcurrentStart { branches, join } | K::FirstSuccessStart { branches, join } => {
            validate_concurrent_entries(parts, src, branches.as_ref(), *join)
        }
        _ => Err(WorkflowError::LoopRegionUnpaired { control: src.id }),
    }
}

fn validate_foreach_input(
    parts: &WorkflowParts,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    at_once: u16,
) -> Result<(), WorkflowError> {
    validate_slot(input, parts.slot_count)?;
    validate_slot(item_slot, parts.slot_count)?;
    validate_reduce_limit(limit)?;
    if at_once == 0 {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::BranchCount,
        });
    }
    if u32::from(at_once) > u32::from(parts.resource_contract.max_fanout) {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Fanout,
        });
    }
    Ok(())
}

fn validate_concurrent_entries(
    parts: &WorkflowParts,
    src: &CompiledNode,
    entries: &[StepIdx],
    join: StepIdx,
) -> Result<(), WorkflowError> {
    if entries.is_empty() {
        return Err(WorkflowError::EmptyBranchTable);
    }
    if entries.len() > usize::from(parts.resource_contract.max_fanout) {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Fanout,
        });
    }
    let node_count = parts.nodes.len();
    for entry in entries {
        validate_step(*entry, node_count)?;
        if !(*entry > src.id && *entry < join) {
            return Err(WorkflowError::LoopRegionCrossing {
                control: src.id,
                from: src.id,
                to: *entry,
            });
        }
    }
    validate_concurrent_entry_uniqueness(src, entries)
}

fn validate_concurrent_entry_uniqueness(
    src: &CompiledNode,
    entries: &[StepIdx],
) -> Result<(), WorkflowError> {
    for (index, entry) in entries.iter().enumerate() {
        let duplicate = entries.iter().take(index).any(|prior| prior == entry);
        if duplicate {
            return Err(WorkflowError::LoopRegionUnpaired { control: src.id });
        }
    }
    Ok(())
}
