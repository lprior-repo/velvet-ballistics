use super::legacy_fixtures::{foreach_within_group, group_within_foreach};
use super::repeat_compositions::change;
use super::*;
use crate::ids::{ConstIdx, StepIdx};

fn within_start(p: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        p,
        1,
        K::ReduceStart {
            limit: 4,
            input: SlotIdx::new(0),
            accumulator: SlotIdx::new(2),
            initial: ConstIdx::new(0),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(1),
        None,
    )?;
    Ok(())
}

fn within_copy(p: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        p,
        2,
        K::Copy {
            source: SlotIdx::new(2),
        },
        Some(2),
        Some(3),
    )?;
    Ok(())
}

fn within_next(p: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        p,
        3,
        K::ReduceNext {
            iterator_slot: SlotIdx::new(1),
            accumulator: SlotIdx::new(2),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(0),
        None,
    )?;
    Ok(())
}

fn within_finish(p: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        p,
        4,
        K::ReduceFinish {
            accumulator: SlotIdx::new(2),
        },
        Some(3),
        Some(5),
    )?;
    Ok(())
}

fn within() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let mut p = foreach_within_group();
    p.constants = Box::new([crate::ConstValue::I64(1)]);
    within_start(&mut p)?;
    within_copy(&mut p)?;
    within_next(&mut p)?;
    within_finish(&mut p)?;
    Ok(p)
}

fn around_start(p: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        p,
        0,
        K::ReduceStart {
            limit: 4,
            input: SlotIdx::new(0),
            accumulator: SlotIdx::new(2),
            initial: ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(5),
        },
        Some(1),
        None,
    )?;
    Ok(())
}

fn around_next(p: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        p,
        4,
        K::ReduceNext {
            iterator_slot: SlotIdx::new(1),
            accumulator: SlotIdx::new(2),
            body: StepIdx::new(1),
            done: StepIdx::new(5),
        },
        Some(0),
        None,
    )?;
    Ok(())
}

fn around_finish(p: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        p,
        5,
        K::ReduceFinish {
            accumulator: SlotIdx::new(2),
        },
        Some(4),
        Some(6),
    )?;
    Ok(())
}

fn around() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let mut p = group_within_foreach();
    p.constants = Box::new([crate::ConstValue::I64(1)]);
    around_start(&mut p)?;
    around_next(&mut p)?;
    around_finish(&mut p)?;
    Ok(p)
}

#[test]
fn validated_reduce_inside_lane() -> Result<(), Box<dyn std::error::Error>> {
    let parts = within()?;
    let regions = crate::workflow::concurrent_analyze::analyze(&parts)?;
    assert_eq!(regions.len(), 1);
    let region = regions.first().ok_or("region")?;
    assert_eq!(region.owner, StepIdx::new(0));
    assert_eq!(region.lanes.len(), 1);
    let lane = region.lanes.first().ok_or("lane")?;
    assert_eq!(lane.ends.as_ref(), &[StepIdx::new(5)]);
    Ok(())
}

#[test]
fn validated_reduce_around_group() -> Result<(), Box<dyn std::error::Error>> {
    let parts = around()?;
    let regions = crate::workflow::concurrent_analyze::analyze(&parts)?;
    assert_eq!(regions.len(), 1);
    let region = regions.first().ok_or("region")?;
    assert_eq!(region.owner, StepIdx::new(1));
    assert_eq!(region.lanes.len(), 1);
    let lane = region.lanes.first().ok_or("lane")?;
    assert_eq!(lane.ends.as_ref(), &[StepIdx::new(2)]);
    Ok(())
}
