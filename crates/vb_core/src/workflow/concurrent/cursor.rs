use super::concurrent_ownership::{LaneOwner, owners};
use crate::ids::StepIdx;
use crate::workflow::{CompiledNodeKind as K, WorkflowError, WorkflowParts};

pub(super) fn contexts(parts: &WorkflowParts) -> Result<Option<Vec<LaneOwner>>, WorkflowError> {
    let needs = parts.nodes.iter().any(|n| {
        matches!(
            n.kind,
            K::ForEachConcurrentStart { .. }
                | K::TogetherConcurrentStart { .. }
                | K::FirstSuccessStart { .. }
        )
    });
    if !needs {
        return Ok(None);
    }
    owners(parts).map(Some)
}

pub(super) fn same_context(
    map: Option<&[LaneOwner]>,
    owner: StepIdx,
    writer: StepIdx,
) -> Result<bool, WorkflowError> {
    match map {
        None => Ok(true),
        Some(m) => {
            let a = m
                .get(owner.as_usize())
                .ok_or(WorkflowError::StepOutOfBounds { step: owner })?;
            let b = m
                .get(writer.as_usize())
                .ok_or(WorkflowError::StepOutOfBounds { step: writer })?;
            match (a, b) {
                (None, None) => Ok(true),
                (Some(x), Some(y)) => Ok(x == y),
                _ => Ok(false),
            }
        }
    }
}
