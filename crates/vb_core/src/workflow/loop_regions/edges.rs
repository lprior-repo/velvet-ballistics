#![forbid(unsafe_code)]

use super::{CompiledLoopRegion, WorkflowError, WorkflowParts, crossing, resource_error};
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeOrigin {
    Next,
    Error,
    Kind,
}

pub(super) fn validate_region_edges(
    parts: &WorkflowParts,
    region: CompiledLoopRegion,
    regions: &[Option<CompiledLoopRegion>],
    targets: &mut Vec<StepIdx>,
) -> Result<(), WorkflowError> {
    if region.contains(parts.entry) {
        return Err(crossing(region, parts.entry, parts.entry));
    }
    for node in &parts.nodes {
        for (target, origin) in [
            (node.next, EdgeOrigin::Next),
            (node.on_error, EdgeOrigin::Error),
        ] {
            if let Some(target) = target {
                validate_edge(parts, region, regions, node, target, origin)?;
            }
        }
        collect_kind_targets(node, targets)?;
        for target in targets.iter().copied() {
            validate_edge(parts, region, regions, node, target, EdgeOrigin::Kind)?;
        }
        validate_recovery_support(parts, region, node)?;
    }
    Ok(())
}

fn collect_kind_targets(
    node: &CompiledNode,
    targets: &mut Vec<StepIdx>,
) -> Result<(), WorkflowError> {
    targets.clear();
    let branches = match &node.kind {
        CompiledNodeKind::Choose { branches, .. } => branches.len(),
        CompiledNodeKind::ChooseSlot { branches, .. } => branches.len(),
        CompiledNodeKind::TogetherStart { branches, .. } => branches.len(),
        _ => 1,
    };
    let capacity = branches.checked_add(1).ok_or_else(resource_error)?;
    targets
        .try_reserve_exact(capacity)
        .map_err(|_| resource_error())?;
    // Reuse the constructor's exhaustive successor inventory, including all
    // branch/fallback, loop body/done/exhausted and handler/async routes.
    super::super::collect_node_targets(&node.kind, targets);
    Ok(())
}

fn validate_edge(
    parts: &WorkflowParts,
    region: CompiledLoopRegion,
    regions: &[Option<CompiledLoopRegion>],
    node: &CompiledNode,
    to: StepIdx,
    origin: EdgeOrigin,
) -> Result<(), WorkflowError> {
    let from = node.id;
    let invalid = match (region.contains(from), region.contains(to)) {
        (false, true) => origin != EdgeOrigin::Kind || from != region.start || to != region.body,
        (true, false) => to != region.done && !is_finish(parts, to),
        (true, true) => to <= from && !owned_backedge(region, regions, node, to, origin),
        (false, false) => false,
    };
    if invalid {
        Err(crossing(region, from, to))
    } else {
        Ok(())
    }
}

fn owned_backedge(
    outer: CompiledLoopRegion,
    regions: &[Option<CompiledLoopRegion>],
    node: &CompiledNode,
    to: StepIdx,
    origin: EdgeOrigin,
) -> bool {
    let expected = if matches!(node.kind, CompiledNodeKind::RepeatCheck { .. }) {
        EdgeOrigin::Next
    } else {
        EdgeOrigin::Kind
    };
    if origin != expected {
        return false;
    }
    regions
        .get(node.id.as_usize())
        .copied()
        .flatten()
        .is_some_and(|inner| {
            inner.body == to && outer.body <= inner.body && inner.control <= outer.control
        })
}

fn validate_recovery_support(
    parts: &WorkflowParts,
    region: CompiledLoopRegion,
    node: &CompiledNode,
) -> Result<(), WorkflowError> {
    if !region.contains(node.id) {
        return Ok(());
    }
    let internal_handler = node
        .on_error
        .is_some_and(|target| region.contains(target) && !is_finish(parts, target));
    if internal_handler || matches!(node.kind, CompiledNodeKind::ErrorHandler { .. }) {
        return Err(WorkflowError::LoopRegionUnsupported {
            control: region.control,
            step: node.id,
        });
    }
    Ok(())
}

fn is_finish(parts: &WorkflowParts, step: StepIdx) -> bool {
    parts
        .nodes
        .get(step.as_usize())
        .is_some_and(|node| matches!(node.kind, CompiledNodeKind::Finish { .. }))
}
