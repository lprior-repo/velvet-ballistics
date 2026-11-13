use crate::ids::StepIdx;
use crate::workflow::CompiledLoopRegion;
use crate::workflow::{CompiledNode, CompiledNodeKind as K, WorkflowError, WorkflowParts};

pub(super) type LaneOwner = Option<(StepIdx, StepIdx)>;

fn assign(
    map: &mut [LaneOwner],
    step: StepIdx,
    owner: StepIdx,
    entry: StepIdx,
) -> Result<(), WorkflowError> {
    let cell = map
        .get_mut(step.as_usize())
        .ok_or(WorkflowError::StepOutOfBounds { step })?;
    if cell.is_some_and(|prior| prior != (owner, entry)) {
        return Err(WorkflowError::LoopRegionCrossing {
            control: owner,
            from: entry,
            to: step,
        });
    }
    *cell = Some((owner, entry));
    Ok(())
}

fn mark_members(
    parts: &WorkflowParts,
    map: &mut [LaneOwner],
    owner: StepIdx,
    entry: StepIdx,
) -> Result<(), WorkflowError> {
    for step in super::concurrent_lane_walk::lane_members(parts, owner, entry)? {
        assign(map, step, owner, entry)?;
        let node = super::concurrent_pair::at(parts, step)?;
        match &node.kind {
            K::ForEachConcurrentStart { join, .. }
            | K::TogetherConcurrentStart { join, .. }
            | K::FirstSuccessStart { join, .. } => {
                assign(map, *join, owner, entry)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn build(parts: &WorkflowParts) -> Result<Vec<LaneOwner>, WorkflowError> {
    let mut map = super::concurrent_lane_walk::scratch(parts.nodes.len())?;
    map.resize(parts.nodes.len(), None);
    for node in parts.nodes.iter() {
        if matches!(
            node.kind,
            K::ForEachConcurrentStart { .. }
                | K::TogetherConcurrentStart { .. }
                | K::FirstSuccessStart { .. }
        ) {
            super::concurrent_pair::validate_concurrent_pair(parts, node.id)?;
            super::concurrent_start_entries::validate(parts, node)?;
            let (starts, _) = super::concurrent_lane_walk::entries(node)?;
            for entry in starts {
                mark_members(parts, &mut map, node.id, *entry)?;
            }
        }
    }
    Ok(map)
}

struct RootWalk {
    queue: Vec<StepIdx>,
    seen: Vec<bool>,
    legacy: Box<[Option<CompiledLoopRegion>]>,
}

fn check_root_scope(
    map: &[LaneOwner],
    from: Option<StepIdx>,
    target: StepIdx,
    w: &RootWalk,
) -> Result<(), WorkflowError> {
    let scope = map
        .get(target.as_usize())
        .copied()
        .ok_or(WorkflowError::StepOutOfBounds { step: target })?;
    if let Some((owner, _)) = scope {
        return Err(WorkflowError::LoopRegionCrossing {
            control: owner,
            from: from.map_or(target, |s| s),
            to: target,
        });
    }
    if let Some(origin) = from
        && target <= origin
        && !super::concurrent_backedges::known(&w.legacy, &w.seen, origin, target)
    {
        return Err(WorkflowError::LoopRegionUnsupported {
            control: origin,
            step: origin,
        });
    }
    Ok(())
}

fn root_enqueue(
    map: &[LaneOwner],
    w: &mut RootWalk,
    from: Option<StepIdx>,
    target: StepIdx,
) -> Result<(), WorkflowError> {
    check_root_scope(map, from, target, w)?;
    let flag = w
        .seen
        .get_mut(target.as_usize())
        .ok_or(WorkflowError::StepOutOfBounds { step: target })?;
    if !*flag {
        *flag = true;
        w.queue.push(target);
    }
    Ok(())
}

fn root_walk(parts: &WorkflowParts, map: &[LaneOwner]) -> Result<Vec<bool>, WorkflowError> {
    let legacy = super::concurrent_backedges::validated(parts)?;
    let queue = super::concurrent_lane_walk::scratch(parts.nodes.len())?;
    let mut seen = super::concurrent_lane_walk::scratch(parts.nodes.len())?;
    seen.resize(parts.nodes.len(), false);
    let mut w = RootWalk {
        queue,
        seen,
        legacy,
    };
    root_enqueue(map, &mut w, None, parts.entry)?;
    for index in 0..parts.nodes.len() {
        let Some(step) = w.queue.get(index).copied() else {
            break;
        };
        let node = super::concurrent_pair::at(parts, step)?;
        if matches!(node.kind, K::ConcurrentLaneEnd { .. }) {
            return Err(WorkflowError::LoopRegionUnpaired { control: step });
        }
        for target in super::concurrent_targets::targets(parts, node)? {
            root_enqueue(map, &mut w, Some(step), target)?;
        }
    }
    Ok(w.seen)
}

fn covered_join(
    parts: &WorkflowParts,
    map: &[LaneOwner],
    seen: &[bool],
    node: &CompiledNode,
) -> Result<bool, WorkflowError> {
    let group_step = match &node.kind {
        K::ConcurrentGroupJoin { owner } => *owner,
        _ => return Ok(false),
    };
    super::concurrent_pair::validate_concurrent_pair(parts, group_step)?;
    let src = super::concurrent_pair::at(parts, group_step)?;
    let (_, join) = super::concurrent_lane_walk::entries(src)?;
    Ok(join == node.id
        && map.get(group_step.as_usize()) == Some(&None)
        && seen.get(group_step.as_usize()).copied() == Some(true))
}

fn coverage(parts: &WorkflowParts, map: &[LaneOwner], seen: &[bool]) -> Result<(), WorkflowError> {
    for node in parts.nodes.iter() {
        let scope = map
            .get(node.id.as_usize())
            .ok_or(WorkflowError::StepOutOfBounds { step: node.id })?;
        if scope.is_some() || seen.get(node.id.as_usize()).copied() == Some(true) {
            continue;
        }
        if !covered_join(parts, map, seen, node)? {
            return Err(WorkflowError::UnreachableNode { step: node.id });
        }
    }
    Ok(())
}

pub(super) fn owners(parts: &WorkflowParts) -> Result<Vec<LaneOwner>, WorkflowError> {
    for (index, node) in parts.nodes.iter().enumerate() {
        super::validate::validate_node_id(node, index)?;
    }
    let map = build(parts)?;
    let seen = root_walk(parts, &map)?;
    coverage(parts, &map, &seen)?;
    Ok(map)
}
