use super::ConcurrentAnalysisError;
use super::concurrent_group_frame_bound::FrameBoundError;
use super::concurrent_ownership::LaneOwner;
use super::concurrent_peaks::{PeakError, PeakRequirements};
use super::concurrent_regions::{ConcurrentGroupRegion, ConcurrentLaneRegion, ConcurrentMode};
use crate::ids::StepIdx;
use crate::workflow::{
    CompiledNode, CompiledNodeKind as K, ContractResource, WorkflowError, WorkflowParts,
};

fn foreach_capacity(limit: u32, cap: u16) -> Result<u16, WorkflowError> {
    let value = limit.min(u32::from(cap));
    u16::try_from(value).map_err(|_| WorkflowError::ResourceContractExceeded {
        resource: ContractResource::Fanout,
    })
}

fn fixed_capacity(count: usize) -> Result<u16, WorkflowError> {
    u16::try_from(count).map_err(|_| WorkflowError::ResourceContractExceeded {
        resource: ContractResource::Fanout,
    })
}

fn mode_cap(kind: &K, owner: StepIdx) -> Result<(ConcurrentMode, u16), WorkflowError> {
    match kind {
        K::ForEachConcurrentStart { limit, at_once, .. } => Ok((
            ConcurrentMode::ForEachOrdered,
            foreach_capacity(limit.get(), at_once.get())?,
        )),
        K::TogetherConcurrentStart { branches, .. } => Ok((
            ConcurrentMode::TogetherOrdered,
            fixed_capacity(branches.len())?,
        )),
        K::FirstSuccessStart { branches, .. } => Ok((
            ConcurrentMode::FirstSuccess,
            fixed_capacity(branches.len())?,
        )),
        _ => Err(WorkflowError::LoopRegionUnpaired { control: owner }),
    }
}

fn lane(
    parts: &WorkflowParts,
    owner: StepIdx,
    entry: StepIdx,
) -> Result<ConcurrentLaneRegion, WorkflowError> {
    let exclusive_end = super::concurrent_lane_walk::boundary(parts, owner, entry)?;
    let (ends, children) = super::concurrent_lane_walk::walk_lane(parts, owner, entry)?;
    Ok(ConcurrentLaneRegion {
        entry,
        exclusive_end,
        ends: ends.into_boxed_slice(),
        children: children.into_boxed_slice(),
    })
}

fn group(
    parts: &WorkflowParts,
    map: &[LaneOwner],
    node: &CompiledNode,
) -> Result<ConcurrentGroupRegion, WorkflowError> {
    let (starts, join) = super::concurrent_lane_walk::entries(node)?;
    let mut lanes = super::concurrent_lane_walk::scratch(starts.len())?;
    for entry in starts {
        lanes.push(lane(parts, node.id, *entry)?);
    }
    let (mode, capacity) = mode_cap(&node.kind, node.id)?;
    let parent = map
        .get(node.id.as_usize())
        .copied()
        .ok_or(WorkflowError::StepOutOfBounds { step: node.id })?
        .map(|(parent, _)| parent);
    Ok(ConcurrentGroupRegion {
        owner: node.id,
        join,
        parent,
        mode,
        capacity,
        lanes: lanes.into_boxed_slice(),
    })
}

pub(super) fn analyze(
    parts: &WorkflowParts,
) -> Result<Box<[ConcurrentGroupRegion]>, WorkflowError> {
    let map = super::concurrent_ownership::owners(parts)?;
    let mut groups = super::concurrent_lane_walk::scratch(parts.nodes.len())?;
    for node in parts.nodes.iter() {
        if matches!(
            node.kind,
            K::ForEachConcurrentStart { .. }
                | K::TogetherConcurrentStart { .. }
                | K::FirstSuccessStart { .. }
        ) {
            groups.push(group(parts, &map, node)?);
        }
    }
    Ok(groups.into_boxed_slice())
}

fn from_frame(error: FrameBoundError) -> WorkflowError {
    match error {
        FrameBoundError::InvalidShape { owner } => {
            ConcurrentAnalysisError::InvalidShape { owner }.into()
        }
        FrameBoundError::OverFanout { owner, actual, max } => {
            ConcurrentAnalysisError::OverFanout { owner, actual, max }.into()
        }
        FrameBoundError::CountOverflow { owner, actual } => {
            ConcurrentAnalysisError::FrameOverflow { owner, actual }.into()
        }
    }
}

fn from_peak(error: PeakError) -> WorkflowError {
    match error {
        PeakError::Allocate => WorkflowError::ResourceContractExceeded {
            resource: ContractResource::LoopRegions,
        },
        PeakError::InvalidLayout => ConcurrentAnalysisError::InvalidLayout.into(),
        PeakError::Frame(error) => from_frame(error),
        PeakError::GroupOverflow { owner, actual } => {
            ConcurrentAnalysisError::GroupOverflow { owner, actual }.into()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConcurrentValidation {
    pub(super) regions: Box<[ConcurrentGroupRegion]>,
    pub(super) requirements: PeakRequirements,
}

/// Validate concurrent slotflow requirements after all common validators have run.
/// Caller must complete common validation in established order; this helper does not
/// repeat or bypass them. Regions are validated for concurrency compatibility.
/// Capacity-only path may remain typed as Unsupported until legacy/flow complete.
pub(super) fn validate_after_common(
    parts: &WorkflowParts,
) -> Result<ConcurrentValidation, WorkflowError> {
    let regions = analyze(parts)?;
    let requirements = super::concurrent_peaks::analyze(parts, &regions).map_err(from_peak)?;
    Ok(ConcurrentValidation {
        regions,
        requirements,
    })
}
