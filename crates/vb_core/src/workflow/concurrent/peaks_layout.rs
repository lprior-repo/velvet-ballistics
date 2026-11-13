use super::*;
use crate::workflow::CompiledNodeKind as K;

pub(super) fn registry<'a>(
    parts: &WorkflowParts,
    regions: &'a [ConcurrentGroupRegion],
) -> Result<Vec<Option<&'a ConcurrentGroupRegion>>, PeakError> {
    let mut table = slots(parts.nodes.len(), None)?;
    let mut previous: Option<StepIdx> = None;
    for g in regions {
        let current = g.owner;
        if previous.is_some_and(|prev| prev >= current) {
            return Err(PeakError::InvalidLayout);
        }
        previous = Some(current);
        validate_region(parts, g)?;
        let owner_idx = current.as_usize();
        let slot = table.get_mut(owner_idx).ok_or(PeakError::InvalidLayout)?;
        if slot.is_some() {
            return Err(PeakError::InvalidLayout);
        }
        *slot = Some(g);
    }
    census(parts, &table)?;
    Ok(table)
}

fn validate_region(parts: &WorkflowParts, g: &ConcurrentGroupRegion) -> Result<(), PeakError> {
    let owner_node = parts
        .nodes
        .get(g.owner.as_usize())
        .ok_or(PeakError::InvalidLayout)?;
    if owner_node.id != g.owner {
        return Err(PeakError::InvalidLayout);
    }
    let expected_join = validate_kind(&owner_node.kind, g)?;
    if expected_join != g.join {
        return Err(PeakError::InvalidLayout);
    }
    validate_join(parts, g.owner, g.join)?;
    for lane in g.lanes.iter() {
        validate_lane_entry(parts, lane)?;
        validate_lane_ends(parts, g, lane)?;
    }
    Ok(())
}

fn validate_join(parts: &WorkflowParts, owner: StepIdx, join: StepIdx) -> Result<(), PeakError> {
    let join_node = parts
        .nodes
        .get(join.as_usize())
        .ok_or(PeakError::InvalidLayout)?;
    if join_node.id != join {
        return Err(PeakError::InvalidLayout);
    }
    match &join_node.kind {
        K::ConcurrentGroupJoin { owner: o, .. } => {
            if *o != owner {
                return Err(PeakError::InvalidLayout);
            }
        }
        _ => return Err(PeakError::InvalidLayout),
    }
    Ok(())
}

fn validate_kind(
    kind: &crate::workflow::CompiledNodeKind,
    g: &ConcurrentGroupRegion,
) -> Result<StepIdx, PeakError> {
    match kind {
        K::ForEachConcurrentStart {
            at_once,
            limit,
            body,
            join,
            ..
        } => {
            validate_foreach(g, *limit, *at_once, *body)?;
            Ok(*join)
        }
        K::TogetherConcurrentStart { branches, join } => {
            if g.mode != GroupMode::TogetherOrdered {
                return Err(PeakError::InvalidLayout);
            }
            validate_fixed(g, branches)?;
            Ok(*join)
        }
        K::FirstSuccessStart { branches, join } => {
            if g.mode != GroupMode::FirstSuccess {
                return Err(PeakError::InvalidLayout);
            }
            validate_fixed(g, branches)?;
            Ok(*join)
        }
        _ => Err(PeakError::InvalidLayout),
    }
}

fn validate_foreach(
    g: &ConcurrentGroupRegion,
    limit: crate::ids::FanoutLimit,
    at_once: crate::ids::BranchCount,
    body: StepIdx,
) -> Result<(), PeakError> {
    if g.mode != GroupMode::ForEachOrdered {
        return Err(PeakError::InvalidLayout);
    }
    if at_once.get() == 0 {
        return Err(PeakError::InvalidLayout);
    }
    if g.lanes.len() != 1 {
        return Err(PeakError::InvalidLayout);
    }
    let first = g.lanes.first().ok_or(PeakError::InvalidLayout)?;
    if first.entry != body {
        return Err(PeakError::InvalidLayout);
    }
    let cap = u32::from(at_once.get()).min(limit.get());
    let capacity = u16::try_from(cap).map_err(|_| PeakError::InvalidLayout)?;
    if g.capacity != capacity {
        return Err(PeakError::InvalidLayout);
    }
    Ok(())
}

fn validate_fixed(g: &ConcurrentGroupRegion, branches: &[StepIdx]) -> Result<(), PeakError> {
    if branches.is_empty() {
        return Err(PeakError::InvalidLayout);
    }
    let capacity = u16::try_from(branches.len()).map_err(|_| PeakError::InvalidLayout)?;
    if g.capacity != capacity {
        return Err(PeakError::InvalidLayout);
    }
    if g.lanes.len() != branches.len() {
        return Err(PeakError::InvalidLayout);
    }
    for (lane, branch) in g.lanes.iter().zip(branches.iter()) {
        if lane.entry != *branch {
            return Err(PeakError::InvalidLayout);
        }
    }
    Ok(())
}

fn validate_lane_entry(
    parts: &WorkflowParts,
    lane: &ConcurrentLaneRegion,
) -> Result<(), PeakError> {
    let entry_node = parts
        .nodes
        .get(lane.entry.as_usize())
        .ok_or(PeakError::InvalidLayout)?;
    if entry_node.id != lane.entry {
        return Err(PeakError::InvalidLayout);
    }
    if lane.entry.as_usize() >= lane.exclusive_end.as_usize() {
        return Err(PeakError::InvalidLayout);
    }
    if lane.exclusive_end.as_usize() > parts.nodes.len() {
        return Err(PeakError::InvalidLayout);
    }
    Ok(())
}

fn validate_lane_ends(
    parts: &WorkflowParts,
    g: &ConcurrentGroupRegion,
    lane: &ConcurrentLaneRegion,
) -> Result<(), PeakError> {
    for end in lane.ends.iter() {
        let end_node = parts
            .nodes
            .get(end.as_usize())
            .ok_or(PeakError::InvalidLayout)?;
        if end_node.id != *end {
            return Err(PeakError::InvalidLayout);
        }
        match &end_node.kind {
            K::ConcurrentLaneEnd { owner, .. } => {
                if *owner != g.owner {
                    return Err(PeakError::InvalidLayout);
                }
            }
            _ => return Err(PeakError::InvalidLayout),
        }
        if *end < lane.entry || *end >= lane.exclusive_end {
            return Err(PeakError::InvalidLayout);
        }
    }
    Ok(())
}

fn census(
    parts: &WorkflowParts,
    table: &[Option<&ConcurrentGroupRegion>],
) -> Result<(), PeakError> {
    for (index, node) in parts.nodes.iter().enumerate() {
        if is_concurrent_start(node) {
            if node.id.as_usize() != index {
                return Err(PeakError::InvalidLayout);
            }
            let registered = table.get(index).and_then(|v| *v).is_some();
            if !registered {
                return Err(PeakError::InvalidLayout);
            }
        }
    }
    Ok(())
}

fn is_concurrent_start(node: &crate::workflow::CompiledNode) -> bool {
    matches!(
        node.kind,
        K::ForEachConcurrentStart { .. }
            | K::TogetherConcurrentStart { .. }
            | K::FirstSuccessStart { .. }
    )
}
