use super::*;

pub(super) fn validate_concurrent_foreach(
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: FanoutLimit,
    at_once: BranchCount,
    body: StepIdx,
    join: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    crate::workflow::resource_limits::validate_reduce_limit(limit.get())?;
    validate_slot(input, parts.slot_count)?;
    validate_slot(item_slot, parts.slot_count)?;
    validate_nonzero_u16(at_once.get(), ContractResource::BranchCount)?;
    if u32::from(at_once.get()) > u32::from(parts.resource_contract.max_fanout) {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Fanout,
        });
    }
    validate_two_steps(body, join, parts)
}
