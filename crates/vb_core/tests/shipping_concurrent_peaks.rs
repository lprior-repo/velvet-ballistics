pub use vb_core::{execution_lanes, ids, workflow};

#[path = "../src/workflow/concurrent/group_frame_bound.rs"]
mod concurrent_group_frame_bound;
#[path = "../src/workflow/concurrent/peaks.rs"]
mod concurrent_peaks;
#[path = "../src/workflow/concurrent/regions.rs"]
mod concurrent_regions;
#[path = "shipping_concurrent_peaks/fixtures.rs"]
mod fixtures;

use concurrent_group_frame_bound::FrameBoundError;
use concurrent_peaks::{PeakError, analyze};
use fixtures::*;
use ids::StepIdx;

fn require_equal<T: PartialEq>(actual: T, expected: T) -> Result<(), PeakError> {
    if actual != expected {
        return Err(PeakError::InvalidLayout);
    }
    Ok(())
}

fn outer_lane(
    regions: &mut [concurrent_regions::ConcurrentGroupRegion],
) -> Result<&mut concurrent_regions::ConcurrentLaneRegion, PeakError> {
    let outer = regions.first_mut().ok_or(PeakError::InvalidLayout)?;
    let lane = outer.lanes.first_mut().ok_or(PeakError::InvalidLayout)?;
    Ok(lane)
}

#[test]
fn nested_8x4_exact_peak() -> Result<(), PeakError> {
    let (parts, regions) = nested(8, 4);
    let req = analyze(&parts, &regions)?;
    require_equal(req.frames, 41)?;
    require_equal(req.groups, 9)?;
    require_equal(req.frames.checked_sub(1), Some(40))?;
    Ok(())
}

#[test]
fn sequential_exact() -> Result<(), PeakError> {
    let (parts, regions) = sequential();
    let req = analyze(&parts, &regions)?;
    require_equal(req.frames, 4)?;
    require_equal(req.groups, 1)?;
    Ok(())
}

#[test]
fn fixed_together_exact() -> Result<(), PeakError> {
    let (parts, regions) = fixed_nested(false);
    let req = analyze(&parts, &regions)?;
    require_equal(req.frames, 11)?;
    require_equal(req.groups, 3)?;
    Ok(())
}

#[test]
fn first_success_fixed_exact() -> Result<(), PeakError> {
    let (parts, regions) = fixed_nested(true);
    let req = analyze(&parts, &regions)?;
    require_equal(req.frames, 11)?;
    require_equal(req.groups, 3)?;
    Ok(())
}

#[test]
fn zero_capacity_single_exact() -> Result<(), PeakError> {
    let (parts, regions) = single(0, 4, 0);
    let req = analyze(&parts, &regions)?;
    require_equal(req.frames, 1)?;
    require_equal(req.groups, 1)?;
    Ok(())
}

#[test]
fn nested_256x256_frame_overflow() -> Result<(), PeakError> {
    let (parts, regions) = nested(256, 256);
    require_equal(
        analyze(&parts, &regions),
        Err(PeakError::Frame(FrameBoundError::CountOverflow {
            owner: StepIdx::new(0),
            actual: 65793,
        })),
    )?;
    Ok(())
}

#[test]
fn invalid_unknown_child() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    let lane = outer_lane(&mut regions)?;
    lane.children = Box::new([StepIdx::new(99)]);
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_wrong_parent() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    let inner = regions.get_mut(1).ok_or(PeakError::InvalidLayout)?;
    inner.parent = None;
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_duplicate_owner() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    let inner = regions.get_mut(1).ok_or(PeakError::InvalidLayout)?;
    inner.owner = StepIdx::new(0);
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_out_of_order() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    regions.reverse();
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_missing_region() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    regions.truncate(1);
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_orphan_child() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    let lane = outer_lane(&mut regions)?;
    lane.children = Box::new([]);
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_duplicate_child() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    let lane = outer_lane(&mut regions)?;
    lane.children = Box::new([StepIdx::new(1), StepIdx::new(1)]);
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_backward_child() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    let inner = regions.get_mut(1).ok_or(PeakError::InvalidLayout)?;
    let lane = inner.lanes.first_mut().ok_or(PeakError::InvalidLayout)?;
    lane.children = Box::new([StepIdx::new(0)]);
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_mode_mismatch() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    let outer = regions.first_mut().ok_or(PeakError::InvalidLayout)?;
    outer.mode = execution_lanes::GroupMode::FirstSuccess;
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_capacity_mismatch() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    let outer = regions.first_mut().ok_or(PeakError::InvalidLayout)?;
    outer.capacity = 7;
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_join_mismatch() -> Result<(), PeakError> {
    let (parts, mut regions) = nested(8, 4);
    let outer = regions.first_mut().ok_or(PeakError::InvalidLayout)?;
    outer.join = StepIdx::new(4);
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_node_identity_mismatch() -> Result<(), PeakError> {
    let (mut parts, regions) = nested(8, 4);
    let node = parts.nodes.get_mut(1).ok_or(PeakError::InvalidLayout)?;
    node.id = StepIdx::new(0);
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}

#[test]
fn invalid_branch_entry_mismatch() -> Result<(), PeakError> {
    let (parts, mut regions) = fixed_nested(false);
    let lane = outer_lane(&mut regions)?;
    lane.entry = StepIdx::new(2);
    require_equal(analyze(&parts, &regions), Err(PeakError::InvalidLayout))?;
    Ok(())
}
