use super::validate_refs::validate_optional_slot;
use crate::workflow::{
    CompiledNodeKind as K, ConcurrentAnalysisError, WorkflowError, WorkflowParts,
};

pub(super) fn validate(kind: &K, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    match kind {
        K::ForEachConcurrentStart { .. }
        | K::TogetherConcurrentStart { .. }
        | K::FirstSuccessStart { .. } => {
            super::concurrent_start_entries::validate_kind(kind, parts)
        }
        K::ConcurrentLaneEnd { owner, result } => {
            super::concurrent_pair::validate_concurrent_pair(parts, *owner)?;
            validate_optional_slot(*result, parts.slot_count)
        }
        K::ConcurrentGroupJoin { owner } => {
            super::concurrent_pair::validate_concurrent_pair(parts, *owner)
        }
        _ => Err(ConcurrentAnalysisError::InvalidLayout.into()),
    }
}

#[cfg(test)]
#[path = "kind_refs_tests.rs"]
mod tests;
