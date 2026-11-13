use crate::ids::StepIdx;
use crate::workflow::{
    CompiledLoopRegion, CompiledNode, CompiledNodeKind as K, WorkflowError, WorkflowParts,
};

pub(super) fn validated(
    parts: &WorkflowParts,
) -> Result<Box<[Option<CompiledLoopRegion>]>, WorkflowError> {
    references(parts)?;
    super::forward_edges::validate_forward_edges(parts)?;
    super::loop_regions::validate(parts)
}

pub(super) fn known(
    table: &[Option<CompiledLoopRegion>],
    seen: &[bool],
    from: StepIdx,
    target: StepIdx,
) -> bool {
    let Some(region) = table.get(from.as_usize()).and_then(Option::as_ref) else {
        return false;
    };
    if region.control() != from {
        return false;
    }
    if region.body() != target {
        return false;
    }
    seen.get(region.start().as_usize()).copied() == Some(true)
}

fn check_join(
    parts: &WorkflowParts,
    node: &CompiledNode,
    owner: StepIdx,
) -> Result<(), WorkflowError> {
    super::concurrent_pair::validate_concurrent_pair(parts, owner)?;
    let source = super::concurrent_pair::at(parts, owner)?;
    let (_, join) = super::concurrent_lane_walk::entries(source)?;
    if join != node.id {
        return Err(WorkflowError::LoopRegionUnpaired { control: node.id });
    }
    Ok(())
}
fn reference(parts: &WorkflowParts, node: &CompiledNode) -> Result<(), WorkflowError> {
    match &node.kind {
        K::ForEachConcurrentStart { .. }
        | K::TogetherConcurrentStart { .. }
        | K::FirstSuccessStart { .. } => {
            super::concurrent_pair::validate_concurrent_pair(parts, node.id)?;
            super::concurrent_start_entries::validate(parts, node)
        }
        K::ConcurrentLaneEnd { .. } => super::concurrent_lane_end::validate(parts, node),
        K::ConcurrentGroupJoin { owner } => check_join(parts, node, *owner),
        _ => super::validate_node_kind::validate_node_kind(&node.kind, parts),
    }
}
fn references(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    for (index, node) in parts.nodes.iter().enumerate() {
        super::validate::validate_node_id(node, index)?;
        super::validate::validate_node_common(node, parts)?;
        reference(parts, node)?;
    }
    Ok(())
}
