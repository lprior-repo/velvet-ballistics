use crate::ids::StepIdx;
use crate::workflow::{
    CompiledLoopRegion, CompiledNode, CompiledNodeKind, ContractResource, WorkflowError,
    WorkflowParts,
};

pub(super) type LaneWalk = (Vec<StepIdx>, Vec<StepIdx>, Option<StepIdx>);

struct Walk {
    owner: StepIdx,
    entry: StepIdx,
    bound: StepIdx,
    queue: Vec<StepIdx>,
    seen: Vec<bool>,
    ends: Vec<StepIdx>,
    children: Vec<StepIdx>,
    suspension: Option<StepIdx>,
    legacy: Box<[Option<CompiledLoopRegion>]>,
}

pub(super) fn scratch<T>(count: usize) -> Result<Vec<T>, WorkflowError> {
    let mut vec: Vec<T> = Vec::new();
    vec.try_reserve_exact(count)
        .map_err(|_| WorkflowError::ResourceContractExceeded {
            resource: ContractResource::LoopRegions,
        })?;
    Ok(vec)
}
fn init(
    owner: StepIdx,
    entry: StepIdx,
    bound: StepIdx,
    count: usize,
    legacy: Box<[Option<CompiledLoopRegion>]>,
) -> Result<Walk, WorkflowError> {
    let mut seen = scratch(count)?;
    seen.resize(count, false);
    let queue = scratch(count)?;
    let ends = scratch(count)?;
    let children = scratch(count)?;
    Ok(Walk {
        owner,
        entry,
        bound,
        queue,
        seen,
        ends,
        children,
        suspension: None,
        legacy,
    })
}

pub(super) fn entries(node: &CompiledNode) -> Result<(&[StepIdx], StepIdx), WorkflowError> {
    match &node.kind {
        CompiledNodeKind::ForEachConcurrentStart { body, join, .. } => {
            Ok((std::slice::from_ref(body), *join))
        }
        CompiledNodeKind::TogetherConcurrentStart { branches, join }
        | CompiledNodeKind::FirstSuccessStart { branches, join } => Ok((branches.as_ref(), *join)),
        _ => Err(WorkflowError::LoopRegionUnpaired { control: node.id }),
    }
}

pub(super) fn boundary(
    parts: &WorkflowParts,
    owner: StepIdx,
    entry: StepIdx,
) -> Result<StepIdx, WorkflowError> {
    let node = super::concurrent_pair::at(parts, owner)?;
    let (starts, join) = entries(node)?;
    if !starts.contains(&entry) {
        return Err(WorkflowError::LoopRegionUnpaired { control: owner });
    }
    Ok(starts
        .iter()
        .copied()
        .filter(|s| *s > entry)
        .fold(join, |bound, step| bound.min(step)))
}

fn enqueue(w: &mut Walk, from: StepIdx, target: StepIdx) -> Result<(), WorkflowError> {
    if target < w.entry || target >= w.bound {
        return Err(WorkflowError::LoopRegionCrossing {
            control: w.owner,
            from,
            to: target,
        });
    }
    if target <= from && !super::concurrent_backedges::known(&w.legacy, &w.seen, from, target) {
        return Err(WorkflowError::LoopRegionUnsupported {
            control: w.owner,
            step: from,
        });
    }
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

fn record_child(
    parts: &WorkflowParts,
    w: &mut Walk,
    node: &CompiledNode,
) -> Result<(), WorkflowError> {
    super::concurrent_start_entries::validate(parts, node)?;
    let (_, join) = entries(node)?;
    if join >= w.bound {
        return Err(WorkflowError::LoopRegionCrossing {
            control: w.owner,
            from: node.id,
            to: join,
        });
    }
    w.children.push(node.id);
    Ok(())
}

fn record(parts: &WorkflowParts, w: &mut Walk, node: &CompiledNode) -> Result<bool, WorkflowError> {
    match &node.kind {
        CompiledNodeKind::ConcurrentLaneEnd { owner, .. } => {
            if *owner != w.owner {
                return Err(WorkflowError::LoopRegionUnpaired { control: node.id });
            }
            super::concurrent_lane_end::validate(parts, node)?;
            w.ends.push(node.id);
            Ok(true)
        }
        CompiledNodeKind::ForEachConcurrentStart { .. }
        | CompiledNodeKind::TogetherConcurrentStart { .. }
        | CompiledNodeKind::FirstSuccessStart { .. } => {
            record_child(parts, w, node)?;
            Ok(false)
        }
        CompiledNodeKind::Do { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. } => {
            if w.suspension.is_none() {
                w.suspension = Some(node.id);
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn visit(parts: &WorkflowParts, w: &mut Walk, node: &CompiledNode) -> Result<(), WorkflowError> {
    let own_end = record(parts, w, node)?;
    let next = super::concurrent_targets::targets(parts, node)?;
    if !own_end && !super::concurrent_targets::has_normal_successor(node)? {
        return Err(WorkflowError::LoopRegionUnpaired { control: w.owner });
    }
    for target in next {
        enqueue(w, node.id, target)?;
    }
    Ok(())
}

fn walk_state(
    parts: &WorkflowParts,
    owner: StepIdx,
    entry: StepIdx,
) -> Result<Walk, WorkflowError> {
    super::concurrent_pair::validate_concurrent_pair(parts, owner)?;
    let src = super::concurrent_pair::at(parts, owner)?;
    super::concurrent_start_entries::validate(parts, src)?;
    let bound = boundary(parts, owner, entry)?;
    let legacy = super::concurrent_backedges::validated(parts)?;
    let mut w = init(owner, entry, bound, parts.nodes.len(), legacy)?;
    enqueue(&mut w, owner, entry)?;
    for index in 0..parts.nodes.len() {
        let Some(step) = w.queue.get(index).copied() else {
            break;
        };
        let node = super::concurrent_pair::at(parts, step)?;
        visit(parts, &mut w, node)?;
    }
    if w.ends.is_empty() {
        return Err(WorkflowError::LoopRegionUnpaired { control: owner });
    }
    Ok(w)
}

pub(super) fn walk_lane(
    parts: &WorkflowParts,
    owner: StepIdx,
    entry: StepIdx,
) -> Result<LaneWalk, WorkflowError> {
    let w = walk_state(parts, owner, entry)?;
    Ok((w.ends, w.children, w.suspension))
}

pub(super) fn lane_members(
    parts: &WorkflowParts,
    owner: StepIdx,
    entry: StepIdx,
) -> Result<Vec<StepIdx>, WorkflowError> {
    let w = walk_state(parts, owner, entry)?;
    Ok(w.queue)
}

pub(super) fn check_start(parts: &WorkflowParts, owner: StepIdx) -> Result<(), WorkflowError> {
    super::concurrent_pair::validate_concurrent_pair(parts, owner)?;
    let src = super::concurrent_pair::at(parts, owner)?;
    super::concurrent_start_entries::validate(parts, src)?;
    let (starts, _) = entries(src)?;
    for entry in starts.iter() {
        let w = walk_state(parts, owner, *entry)?;
        if let Some(step) = w.suspension {
            return Err(WorkflowError::ConcurrentLaneSuspension {
                owner: w.owner,
                step,
            });
        }
    }
    Ok(())
}
