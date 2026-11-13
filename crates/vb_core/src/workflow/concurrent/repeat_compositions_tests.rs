use super::legacy_fixtures::{foreach_within_group, group_within_foreach};
use super::*;
use crate::workflow::concurrent_analyze::analyze;

pub(super) fn change(
    p: &mut WorkflowParts,
    id: usize,
    kind: K,
    out: Option<u16>,
    next: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    let node = p.nodes.get_mut(id).ok_or("change node id out of range")?;
    node.kind = kind;
    node.output = out.map(SlotIdx::new).into();
    node.routing.next = next.map(StepIdx::new).into();
    Ok(())
}

fn repeat_lane_start(parts: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        parts,
        1,
        K::RepeatStart {
            max_attempts: 2.into(),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(1),
        None,
    )?;
    change(parts, 2, K::Nop, None, Some(3))?;
    Ok(())
}

fn within() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let mut parts = foreach_within_group();
    repeat_lane_start(&mut parts)?;
    change(
        &mut parts,
        3,
        K::RepeatCheck {
            attempt_slot: SlotIdx::new(1),
            done: StepIdx::new(4),
        },
        None,
        Some(2),
    )?;
    change(&mut parts, 4, K::Nop, None, Some(5))?;
    Ok(parts)
}

fn around() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let mut parts = group_within_foreach();
    change(
        &mut parts,
        0,
        K::RepeatStart {
            max_attempts: 2.into(),
            body: StepIdx::new(1),
            done: StepIdx::new(5),
        },
        Some(1),
        None,
    )?;
    change(
        &mut parts,
        4,
        K::RepeatCheck {
            attempt_slot: SlotIdx::new(1),
            done: StepIdx::new(5),
        },
        None,
        Some(1),
    )?;
    change(&mut parts, 5, K::Nop, None, Some(6))?;
    Ok(parts)
}

#[test]
fn validated_repeat_inside_lane() -> Result<(), Box<dyn std::error::Error>> {
    let parts = within()?;
    let regions = analyze(&parts)?;
    assert_eq!(regions.len(), 1);
    let region = regions.first().ok_or("analyze returned no regions")?;
    assert_eq!(region.owner, StepIdx::new(0));
    let lane = region.lanes.first().ok_or("lane")?;
    assert_eq!(lane.ends.len(), 1);
    let end = lane.ends.first().ok_or("region has no end")?;
    assert_eq!(*end, StepIdx::new(5));
    Ok(())
}

#[test]
fn validated_repeat_around_group() -> Result<(), Box<dyn std::error::Error>> {
    let parts = around()?;
    let regions = analyze(&parts)?;
    assert_eq!(regions.len(), 1);
    let region = regions.first().ok_or("analyze returned no regions")?;
    assert_eq!(region.owner, StepIdx::new(1));
    let lane = region.lanes.first().ok_or("lane")?;
    assert_eq!(lane.ends.len(), 1);
    let end = lane.ends.first().ok_or("region has no end")?;
    assert_eq!(*end, StepIdx::new(2));
    Ok(())
}
