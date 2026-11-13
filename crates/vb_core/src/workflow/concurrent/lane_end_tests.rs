use super::*;
use crate::workflow::concurrent_lane_end::validate;

#[test]
fn null_end_pair_validates() -> Result<(), Box<dyn std::error::Error>> {
    let parts = build_parts();
    let compiled = parts.nodes.get(1).ok_or("fixture")?;
    assert_eq!(validate(&parts, compiled), Ok(()));
    Ok(())
}

#[test]
fn end_wrong_owner_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let node = parts.nodes.get_mut(1).ok_or("fixture")?;
    node.kind = K::ConcurrentLaneEnd {
        owner: StepIdx::new(1),
        result: None,
    };
    let compiled = parts.nodes.get(1).ok_or("fixture")?;
    assert_eq!(
        validate(&parts, compiled),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn end_wrong_join_route_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let node = parts.nodes.get_mut(1).ok_or("fixture")?;
    node.routing.next = Some(StepIdx::new(3)).into();
    let compiled = parts.nodes.get(1).ok_or("fixture")?;
    assert_eq!(
        validate(&parts, compiled),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn end_result_slot_out_of_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let node = parts.nodes.get_mut(1).ok_or("fixture")?;
    node.kind = K::ConcurrentLaneEnd {
        owner: StepIdx::new(0),
        result: Some(SlotIdx::new(1)),
    };
    let compiled = parts.nodes.get(1).ok_or("fixture")?;
    assert_eq!(
        validate(&parts, compiled),
        Err(WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(1)
        })
    );
    Ok(())
}
