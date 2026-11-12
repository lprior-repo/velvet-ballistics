#![forbid(unsafe_code)]

use super::shapes::{LoopShape, control_shape, start_shape};
use super::{CompiledLoopRegion, WorkflowError, WorkflowParts, crossing};
use crate::ids::StepIdx;
use crate::workflow::CompiledNode;

pub(super) fn cache_control(
    parts: &WorkflowParts,
    node: &CompiledNode,
    regions: &mut [Option<CompiledLoopRegion>],
) -> Result<(), WorkflowError> {
    let Some(shape) = control_shape(node) else {
        return Ok(());
    };
    if shape.body > node.id {
        return Ok(());
    }
    let start = unique_owner(parts, node.id, shape)?;
    let region = CompiledLoopRegion {
        start,
        body: shape.body,
        control: node.id,
        done: shape.done,
    };
    let entry = regions
        .get_mut(node.id.as_usize())
        .ok_or(WorkflowError::StepOutOfBounds { step: node.id })?;
    *entry = Some(region);
    Ok(())
}

fn unique_owner(
    parts: &WorkflowParts,
    control: StepIdx,
    shape: LoopShape,
) -> Result<StepIdx, WorkflowError> {
    let mut owner = None;
    for node in &parts.nodes {
        if start_shape(node) != Some(shape) {
            continue;
        }
        let ordered = node.id < shape.body && shape.body < control && control < shape.done;
        if owner.is_some() || !ordered {
            return Err(WorkflowError::LoopRegionUnpaired { control });
        }
        owner = Some(node.id);
    }
    owner.ok_or(WorkflowError::LoopRegionUnpaired { control })
}

pub(super) fn validate_region_relations(
    regions: &[Option<CompiledLoopRegion>],
) -> Result<(), WorkflowError> {
    for first in regions.iter().flatten().copied() {
        for second in regions.iter().flatten().copied() {
            if first.control >= second.control {
                continue;
            }
            if first.start == second.start {
                return Err(WorkflowError::LoopRegionUnpaired {
                    control: second.control,
                });
            }
            if !disjoint_or_nested(first, second) {
                return Err(crossing(second, second.body, second.control));
            }
        }
    }
    Ok(())
}
fn disjoint_or_nested(first: CompiledLoopRegion, second: CompiledLoopRegion) -> bool {
    first.control < second.body
        || second.control < first.body
        || (first.body < second.body && second.control < first.control)
        || (second.body < first.body && first.control < second.control)
}
