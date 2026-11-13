use super::legacy_fixtures::{foreach_within_group, group_within_foreach};
use super::repeat_compositions::change;
use super::*;

fn within_start(parts: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    let k1 = K::CollectStart {
        source: SlotIdx::new(0),
        limit: 4.into(),
        page_size: 2,
        body: StepIdx::new(2),
        done: StepIdx::new(5),
    };
    change(parts, 1, k1, Some(1), None)?;
    Ok(())
}

fn within_page(parts: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    let k2 = K::CollectPage {
        collector_slot: SlotIdx::new(1),
        body: StepIdx::new(3),
        done: StepIdx::new(5),
    };
    change(parts, 2, k2, None, None)?;
    Ok(())
}

fn within_copy(parts: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        parts,
        3,
        K::Copy {
            source: SlotIdx::new(1),
        },
        Some(3),
        Some(4),
    )?;
    Ok(())
}

fn within_next(parts: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    let k4 = K::CollectNext {
        collector_slot: SlotIdx::new(1),
        body: StepIdx::new(2),
        done: StepIdx::new(5),
    };
    change(parts, 4, k4, None, None)?;
    Ok(())
}

fn within() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let mut parts = foreach_within_group();
    within_start(&mut parts)?;
    within_page(&mut parts)?;
    within_copy(&mut parts)?;
    within_next(&mut parts)?;
    Ok(parts)
}

fn around_start(parts: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    let k0 = K::CollectStart {
        source: SlotIdx::new(0),
        limit: 4.into(),
        page_size: 2,
        body: StepIdx::new(1),
        done: StepIdx::new(5),
    };
    change(parts, 0, k0, Some(1), None)?;
    Ok(())
}

fn around_next(parts: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    let k4 = K::CollectNext {
        collector_slot: SlotIdx::new(1),
        body: StepIdx::new(1),
        done: StepIdx::new(5),
    };
    change(parts, 4, k4, None, None)?;
    Ok(())
}

fn around_finish(parts: &mut WorkflowParts) -> Result<(), Box<dyn std::error::Error>> {
    change(
        parts,
        5,
        K::CollectFinish {
            collector_slot: SlotIdx::new(1),
        },
        Some(4),
        Some(6),
    )?;
    Ok(())
}

fn around() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let mut parts = group_within_foreach();
    around_start(&mut parts)?;
    around_next(&mut parts)?;
    around_finish(&mut parts)?;
    Ok(parts)
}

#[test]
fn validated_collect_inside_lane() -> Result<(), Box<dyn std::error::Error>> {
    let parts = within()?;
    let regions = crate::workflow::concurrent_analyze::analyze(&parts)?;
    assert_eq!(regions.len(), 1);
    let first = regions.first().ok_or("region")?;
    assert_eq!(first.owner, StepIdx::new(0));
    let lane = first.lanes.first().ok_or("lane")?;
    assert_eq!(lane.ends.as_ref(), &[StepIdx::new(5)]);
    Ok(())
}

#[test]
fn validated_collect_around_group() -> Result<(), Box<dyn std::error::Error>> {
    let parts = around()?;
    let regions = crate::workflow::concurrent_analyze::analyze(&parts)?;
    assert_eq!(regions.len(), 1);
    let first = regions.first().ok_or("region")?;
    assert_eq!(first.owner, StepIdx::new(1));
    let lane = first.lanes.first().ok_or("lane")?;
    assert_eq!(lane.ends.as_ref(), &[StepIdx::new(2)]);
    Ok(())
}
