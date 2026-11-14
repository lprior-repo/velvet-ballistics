use super::ConcurrentAnalysisError;
use super::concurrent_group_frame_bound::FrameBoundError;
use super::concurrent_ownership::LaneOwner;
use super::concurrent_peaks::{PeakError, PeakRequirements};
use super::concurrent_regions::{ConcurrentGroupRegion, ConcurrentLaneRegion};
use crate::execution_lanes::GroupMode;
use crate::ids::StepIdx;
use crate::workflow::{
    CompiledNode, CompiledNodeKind as K, ContractResource, WorkflowError, WorkflowParts,
};

type RegionAnalysis = (Box<[ConcurrentGroupRegion]>, Option<(StepIdx, StepIdx)>);

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

fn mode_cap(kind: &K, owner: StepIdx) -> Result<(GroupMode, u16), WorkflowError> {
    match kind {
        K::ForEachConcurrentStart { limit, at_once, .. } => Ok((
            GroupMode::ForEachOrdered,
            foreach_capacity(limit.get(), at_once.get())?,
        )),
        K::TogetherConcurrentStart { branches, .. } => {
            Ok((GroupMode::TogetherOrdered, fixed_capacity(branches.len())?))
        }
        K::FirstSuccessStart { branches, .. } => {
            Ok((GroupMode::FirstSuccess, fixed_capacity(branches.len())?))
        }
        _ => Err(WorkflowError::LoopRegionUnpaired { control: owner }),
    }
}

fn lane(
    parts: &WorkflowParts,
    owner: StepIdx,
    entry: StepIdx,
) -> Result<(ConcurrentLaneRegion, Option<StepIdx>), WorkflowError> {
    let exclusive_end = super::concurrent_lane_walk::boundary(parts, owner, entry)?;
    let (ends, children, suspension) = super::concurrent_lane_walk::walk_lane(parts, owner, entry)?;
    Ok((
        ConcurrentLaneRegion {
            entry,
            exclusive_end,
            ends: ends.into_boxed_slice(),
            children: children.into_boxed_slice(),
        },
        suspension,
    ))
}

fn group(
    parts: &WorkflowParts,
    map: &[LaneOwner],
    node: &CompiledNode,
) -> Result<(ConcurrentGroupRegion, Option<StepIdx>), WorkflowError> {
    let (starts, join) = super::concurrent_lane_walk::entries(node)?;
    let mut lanes = super::concurrent_lane_walk::scratch(starts.len())?;
    let mut first_suspension = None;
    for entry in starts {
        let (lane_region, suspension) = lane(parts, node.id, *entry)?;
        if first_suspension.is_none() {
            first_suspension = suspension;
        }
        lanes.push(lane_region);
    }
    let (mode, capacity) = mode_cap(&node.kind, node.id)?;
    let parent = map
        .get(node.id.as_usize())
        .copied()
        .ok_or(WorkflowError::StepOutOfBounds { step: node.id })?
        .map(|(parent, _)| parent);
    Ok((
        ConcurrentGroupRegion {
            owner: node.id,
            join,
            parent,
            mode,
            capacity,
            lanes: lanes.into_boxed_slice(),
        },
        first_suspension,
    ))
}

fn build_regions(parts: &WorkflowParts) -> Result<RegionAnalysis, WorkflowError> {
    let map = super::concurrent_ownership::owners(parts)?;
    let mut groups = super::concurrent_lane_walk::scratch(parts.nodes.len())?;
    let mut first_suspension = None;
    for node in parts.nodes.iter() {
        if matches!(
            node.kind,
            K::ForEachConcurrentStart { .. }
                | K::TogetherConcurrentStart { .. }
                | K::FirstSuccessStart { .. }
        ) {
            let (group_region, suspension) = group(parts, &map, node)?;
            if first_suspension.is_none()
                && let Some(step) = suspension
            {
                first_suspension = Some((group_region.owner, step));
            }
            groups.push(group_region);
        }
    }
    Ok((groups.into_boxed_slice(), first_suspension))
}

pub(super) fn analyze(
    parts: &WorkflowParts,
) -> Result<Box<[ConcurrentGroupRegion]>, WorkflowError> {
    let (groups, first_suspension) = build_regions(parts)?;
    if let Some((owner, step)) = first_suspension {
        return Err(WorkflowError::ConcurrentLaneSuspension { owner, step });
    }
    Ok(groups)
}

pub(super) fn analyze_raw(
    parts: &WorkflowParts,
) -> Result<Box<[ConcurrentGroupRegion]>, WorkflowError> {
    let (groups, _suspensions) = build_regions(parts)?;
    Ok(groups)
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
