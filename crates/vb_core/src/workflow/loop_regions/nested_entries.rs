#![forbid(unsafe_code)]

//! Cold entry flags for the bounded unsupported-nested-activation gate.
//! Derive only after complete loop-region validation. Cached inner owners
//! contribute their Start and exact inclusive body/control interval. Uncached
//! loop Starts and TogetherStart contribute only their Start: their restored
//! in-body ownership is not established here. These are union flags, not a
//! cleanup proof or permission to activate nested state.

use super::{CompiledLoopRegion, WorkflowError, WorkflowParts, resource_error};
use crate::workflow::CompiledNodeKind;

/// Allocates one node-count table on the cold constructor path, never at drive.
pub(in crate::workflow) fn derive(
    parts: &WorkflowParts,
    regions: &[Option<CompiledLoopRegion>],
) -> Result<Box<[bool]>, WorkflowError> {
    let mut flags = empty_flags(parts.nodes.len())?;
    mark_nested_starts(parts, regions, &mut flags)?;
    mark_cached_bodies(regions, &mut flags)?;
    Ok(flags.into_boxed_slice())
}

fn empty_flags(count: usize) -> Result<Vec<bool>, WorkflowError> {
    let mut flags = Vec::new();
    flags
        .try_reserve_exact(count)
        .map_err(|_| resource_error())?;
    flags.resize(count, false);
    Ok(flags)
}

fn mark_nested_starts(
    parts: &WorkflowParts,
    regions: &[Option<CompiledLoopRegion>],
    flags: &mut [bool],
) -> Result<(), WorkflowError> {
    for node in &parts.nodes {
        if is_start(&node.kind)
            && regions
                .iter()
                .flatten()
                .any(|outer| outer.body() <= node.id && node.id <= outer.control())
        {
            let flag = flags
                .get_mut(node.id.as_usize())
                .ok_or(WorkflowError::StepOutOfBounds { step: node.id })?;
            *flag = true;
        }
    }
    Ok(())
}

fn is_start(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ForEachStart { .. }
            | CompiledNodeKind::CollectStart { .. }
            | CompiledNodeKind::ReduceStart { .. }
            | CompiledNodeKind::RepeatStart { .. }
            | CompiledNodeKind::TogetherStart { .. }
    )
}

fn mark_cached_bodies(
    regions: &[Option<CompiledLoopRegion>],
    flags: &mut [bool],
) -> Result<(), WorkflowError> {
    for inner in regions.iter().flatten() {
        let nested =
            flags
                .get(inner.start().as_usize())
                .copied()
                .ok_or(WorkflowError::StepOutOfBounds {
                    step: inner.start(),
                })?;
        if nested {
            let body = flags
                .get_mut(inner.body().as_usize()..=inner.control().as_usize())
                .ok_or(WorkflowError::StepOutOfBounds {
                    step: inner.control(),
                })?;
            body.fill(true);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "nested_entries/tests.rs"]
mod tests;
