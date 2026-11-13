/// Cold aggregation over validated concurrent-group region cache.
/// Live whole-IR integration remains a prerequisite for actual execution.
use super::concurrent_group_frame_bound::{FrameBoundError, group_frame_peak};
use super::concurrent_regions::{ConcurrentGroupRegion, ConcurrentLaneRegion, ConcurrentMode};
use crate::ids::StepIdx;
use crate::workflow::WorkflowParts;

#[path = "peaks_layout.rs"]
mod layout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PeakRequirements {
    pub(super) frames: u16,
    pub(super) groups: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum PeakError {
    Allocate,
    InvalidLayout,
    Frame(FrameBoundError),
    GroupOverflow { owner: StepIdx, actual: u64 },
}

pub(super) fn slots<T: Clone>(len: usize, value: T) -> Result<Vec<T>, PeakError> {
    let mut v = Vec::new();
    v.try_reserve_exact(len).map_err(|_| PeakError::Allocate)?;
    v.resize(len, value);
    Ok(v)
}

pub(super) fn analyze(
    parts: &WorkflowParts,
    regions: &[ConcurrentGroupRegion],
) -> Result<PeakRequirements, PeakError> {
    let registry = layout::registry(parts, regions)?;
    let n = parts.nodes.len();
    let mut peaks = slots(n, None::<PeakRequirements>)?;
    let mut seen = slots(n, false)?;

    for region in regions.iter().rev() {
        compute(parts, region, &registry, &mut peaks, &mut seen)?;
    }

    roots(regions, &peaks, &seen)
}

fn compute(
    parts: &WorkflowParts,
    region: &ConcurrentGroupRegion,
    registry: &[Option<&ConcurrentGroupRegion>],
    peaks: &mut [Option<PeakRequirements>],
    seen: &mut [bool],
) -> Result<(), PeakError> {
    let lane_peaks = lane_peaks_vec(region, registry, peaks, seen)?;
    let frame_peaks = frame_peaks_from_lane(&lane_peaks)?;
    let owner = region.owner;
    let node = parts
        .nodes
        .get(owner.as_usize())
        .ok_or(PeakError::InvalidLayout)?;
    let frames = group_frame_peak(
        owner,
        &node.kind,
        &frame_peaks,
        parts.resource_contract.max_fanout,
    )
    .map_err(PeakError::Frame)?;
    let groups = group_count(region, &lane_peaks)?;
    let idx = owner.as_usize();
    *peaks.get_mut(idx).ok_or(PeakError::InvalidLayout)? =
        Some(PeakRequirements { frames, groups });
    Ok(())
}

fn lane_peaks_vec(
    region: &ConcurrentGroupRegion,
    registry: &[Option<&ConcurrentGroupRegion>],
    peaks: &mut [Option<PeakRequirements>],
    seen: &mut [bool],
) -> Result<Vec<PeakRequirements>, PeakError> {
    let count = region.lanes.len();
    let mut out = slots(
        count,
        PeakRequirements {
            frames: 1,
            groups: 0,
        },
    )?;
    for (lane, cell) in region.lanes.iter().zip(out.iter_mut()) {
        *cell = lane_peak(region.owner, lane, registry, peaks, seen)?;
    }
    Ok(out)
}

fn frame_peaks_from_lane(lane_peaks: &[PeakRequirements]) -> Result<Vec<u16>, PeakError> {
    let count = lane_peaks.len();
    let mut out = slots(count, 0u16)?;
    for (p, cell) in lane_peaks.iter().zip(out.iter_mut()) {
        *cell = p.frames;
    }
    Ok(out)
}

fn lane_peak(
    current_owner: StepIdx,
    lane: &ConcurrentLaneRegion,
    registry: &[Option<&ConcurrentGroupRegion>],
    peaks: &mut [Option<PeakRequirements>],
    seen: &mut [bool],
) -> Result<PeakRequirements, PeakError> {
    let mut result = PeakRequirements {
        frames: 1,
        groups: 0,
    };
    for child_owner in lane.children.iter() {
        let child_req = child_peak(current_owner, *child_owner, registry, peaks, seen)?;
        if child_req.frames > result.frames {
            result.frames = child_req.frames;
        }
        if child_req.groups > result.groups {
            result.groups = child_req.groups;
        }
    }
    Ok(result)
}

fn child_peak(
    current_owner: StepIdx,
    child_owner: StepIdx,
    registry: &[Option<&ConcurrentGroupRegion>],
    peaks: &mut [Option<PeakRequirements>],
    seen: &mut [bool],
) -> Result<PeakRequirements, PeakError> {
    if child_owner <= current_owner {
        return Err(PeakError::InvalidLayout);
    }
    let child_idx = child_owner.as_usize();
    let child_region = registry
        .get(child_idx)
        .and_then(|r| *r)
        .ok_or(PeakError::InvalidLayout)?;
    if child_region.parent != Some(current_owner) {
        return Err(PeakError::InvalidLayout);
    }
    let s = seen.get_mut(child_idx).ok_or(PeakError::InvalidLayout)?;
    if *s {
        return Err(PeakError::InvalidLayout);
    }
    *s = true;
    let peak = peaks
        .get(child_idx)
        .and_then(|v| *v)
        .ok_or(PeakError::InvalidLayout)?;
    Ok(peak)
}

fn group_count(
    region: &ConcurrentGroupRegion,
    lane_peaks: &[PeakRequirements],
) -> Result<u16, PeakError> {
    let owner = region.owner;
    let payload = match region.mode {
        ConcurrentMode::ForEachOrdered => {
            let first = lane_peaks.first().ok_or(PeakError::InvalidLayout)?;
            let cap = u64::from(region.capacity);
            let g = u64::from(first.groups);
            cap.checked_mul(g).ok_or(overflow(owner))
        }
        ConcurrentMode::TogetherOrdered | ConcurrentMode::FirstSuccess => {
            lane_peaks.iter().try_fold(0u64, |acc, p| {
                acc.checked_add(u64::from(p.groups)).ok_or(overflow(owner))
            })
        }
    }?;
    let total = payload.checked_add(1).ok_or(overflow(owner))?;
    u16::try_from(total).map_err(|_| PeakError::GroupOverflow {
        owner,
        actual: total,
    })
}

fn overflow(owner: StepIdx) -> PeakError {
    PeakError::GroupOverflow {
        owner,
        actual: u64::MAX,
    }
}

fn roots(
    regions: &[ConcurrentGroupRegion],
    peaks: &[Option<PeakRequirements>],
    seen: &[bool],
) -> Result<PeakRequirements, PeakError> {
    let mut result = PeakRequirements {
        frames: 1,
        groups: 0,
    };
    for region in regions {
        if let Some(req) = root_peak(region, peaks, seen)? {
            if req.frames > result.frames {
                result.frames = req.frames;
            }
            if req.groups > result.groups {
                result.groups = req.groups;
            }
        }
    }
    Ok(result)
}

fn root_peak(
    region: &ConcurrentGroupRegion,
    peaks: &[Option<PeakRequirements>],
    seen: &[bool],
) -> Result<Option<PeakRequirements>, PeakError> {
    let owner = region.owner;
    let idx = owner.as_usize();
    let s = seen.get(idx).ok_or(PeakError::InvalidLayout)?;
    let expected_seen = region.parent.is_some();
    if *s != expected_seen {
        return Err(PeakError::InvalidLayout);
    }
    let peak = peaks
        .get(idx)
        .and_then(|v| *v)
        .ok_or(PeakError::InvalidLayout)?;
    if region.parent.is_none() {
        Ok(Some(peak))
    } else {
        Ok(None)
    }
}
