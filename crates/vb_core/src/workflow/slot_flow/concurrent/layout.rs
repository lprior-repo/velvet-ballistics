use super::{FlowResult, capacity, invalid, model_error, scratch, slots};
use crate::ids::StepIdx;
use crate::workflow::WorkflowParts;
use crate::workflow::concurrent_ownership::LaneOwner;
use crate::workflow::concurrent_regions::ConcurrentGroupRegion;

pub(super) struct Layout<'a> {
    pub(super) regions: &'a [ConcurrentGroupRegion],
    pub(super) scopes: Vec<LaneOwner>,
    pub(super) node_frames: Vec<usize>,
    pub(super) group_by_owner: Vec<Option<usize>>,
    pub(super) child_frames: Vec<Vec<usize>>,
}

fn count_lanes(regions: &[ConcurrentGroupRegion]) -> FlowResult<usize> {
    regions.iter().try_fold(1usize, |acc, r| {
        acc.checked_add(r.lanes.len()).ok_or_else(capacity)
    })
}

fn initial<'a>(
    parts: &WorkflowParts,
    regions: &'a [ConcurrentGroupRegion],
) -> FlowResult<Layout<'a>> {
    let lane_count = count_lanes(regions)?;
    let mut scopes = scratch(lane_count)?;
    scopes.push(None);
    let node_count = parts.nodes.len();
    Ok(Layout {
        regions,
        scopes,
        node_frames: scratch(node_count)?,
        group_by_owner: slots(node_count, None)?,
        child_frames: scratch(regions.len())?,
    })
}

fn register_entry(
    layout: &mut Layout<'_>,
    entry_frames: &mut [Option<usize>],
    owner: StepIdx,
    entry: StepIdx,
) -> FlowResult<usize> {
    let slot = entry_frames
        .get_mut(entry.as_usize())
        .ok_or_else(|| invalid(entry))?;
    if slot.is_some() {
        return Err(invalid(entry));
    }
    let frame = layout.scopes.len();
    layout.scopes.push(Some((owner, entry)));
    *slot = Some(frame);
    Ok(frame)
}

fn layout_group(
    layout: &mut Layout<'_>,
    entry_frames: &mut [Option<usize>],
    index: usize,
    region: &ConcurrentGroupRegion,
) -> FlowResult<()> {
    let mut child = scratch(region.lanes.len())?;
    for lane in &region.lanes {
        let frame = register_entry(layout, entry_frames, region.owner, lane.entry)?;
        child.push(frame);
    }
    let owner_slot = layout
        .group_by_owner
        .get_mut(region.owner.as_usize())
        .ok_or_else(|| invalid(region.owner))?;
    if owner_slot.is_some() {
        return Err(invalid(region.owner));
    }
    *owner_slot = Some(index);
    layout.child_frames.push(child);
    Ok(())
}

fn map_owner(
    layout: &Layout<'_>,
    entry_frames: &[Option<usize>],
    scope: LaneOwner,
) -> FlowResult<usize> {
    let Some((owner, entry)) = scope else {
        return Ok(0);
    };
    let frame = entry_frames
        .get(entry.as_usize())
        .and_then(|v| *v)
        .ok_or_else(|| invalid(entry))?;
    if layout.scopes.get(frame).and_then(|s| *s) != Some((owner, entry)) {
        return Err(invalid(entry));
    }
    Ok(frame)
}

fn map_nodes(
    parts: &WorkflowParts,
    layout: &Layout<'_>,
    entry_frames: &[Option<usize>],
) -> FlowResult<Vec<usize>> {
    let source_owners = crate::workflow::concurrent_ownership::owners(parts)
        .map_err(|e| model_error(e, parts.entry))?;
    let node_count = parts.nodes.len();
    if source_owners.len() != node_count {
        return Err(capacity());
    }
    let mut result = scratch(node_count)?;
    for owner in source_owners {
        let frame = map_owner(layout, entry_frames, owner)?;
        result.push(frame);
    }
    Ok(result)
}

pub(super) fn build<'a>(
    parts: &WorkflowParts,
    regions: &'a [ConcurrentGroupRegion],
) -> FlowResult<Layout<'a>> {
    let mut layout = initial(parts, regions)?;
    let entry_frames = slots(parts.nodes.len(), None)?;
    let mut entry_frames = entry_frames;
    for (index, region) in regions.iter().enumerate() {
        layout_group(&mut layout, &mut entry_frames, index, region)?;
    }
    layout.node_frames = map_nodes(parts, &layout, &entry_frames)?;
    Ok(layout)
}

impl<'a> Layout<'a> {
    pub(super) fn frame(&self, step: StepIdx) -> FlowResult<usize> {
        self.node_frames
            .get(step.as_usize())
            .copied()
            .ok_or_else(|| invalid(step))
    }

    pub(super) fn group(&self, owner: StepIdx) -> FlowResult<usize> {
        self.group_by_owner
            .get(owner.as_usize())
            .and_then(|v| *v)
            .ok_or_else(|| invalid(owner))
    }

    pub(super) fn scope(&self, frame: usize) -> FlowResult<LaneOwner> {
        self.scopes.get(frame).copied().ok_or_else(capacity)
    }
}
