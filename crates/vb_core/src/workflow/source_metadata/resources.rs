//! Cold capacity checking for the immutable source symbol prefix.
use crate::workflow::{ContractResource, WorkflowError, WorkflowParts};

pub(crate) fn validate(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let count = parts
        .source_metadata
        .as_ref()
        .map_or(0, |metadata| metadata.symbols_count());
    if count > u32::from(parts.resource_contract.max_slots) {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Slots,
        });
    }
    Ok(())
}
