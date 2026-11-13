use super::*;
use crate::workflow::ErrorRouting;
use crate::workflow::concurrent_targets::targets;

#[test]
fn start_targets_skip_child_body_and_owner_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let parts = build_parts();
    let node0 = parts
        .nodes
        .get(StepIdx::new(0).as_usize())
        .ok_or("fixture: node0")?;
    let result = targets(&parts, node0)?;
    assert_eq!(result.as_slice(), &[StepIdx::new(3)]);
    Ok(())
}

#[test]
fn start_error_handler_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let node0 = parts
        .nodes
        .get_mut(StepIdx::new(0).as_usize())
        .ok_or("fixture: node0")?;
    node0.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(1),
        error_slot: None,
    };
    let result = targets(&parts, parts.nodes.get(0).ok_or("fixture")?)?;
    assert_eq!(result.as_slice(), &[StepIdx::new(3), StepIdx::new(1)]);
    Ok(())
}

#[test]
fn end_targets_exclude_normal_join() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let result = targets(&parts, parts.nodes.get(1).ok_or("fixture")?)?;
    assert_eq!(result.as_slice(), &[]);

    let node1 = parts
        .nodes
        .get_mut(StepIdx::new(1).as_usize())
        .ok_or("fixture: node1")?;
    node1.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(3),
        error_slot: None,
    };
    let result = targets(&parts, parts.nodes.get(1).ok_or("fixture")?)?;
    assert_eq!(result.as_slice(), &[StepIdx::new(3)]);

    use crate::workflow::concurrent_lane_end::validate;
    let err = validate(&parts, parts.nodes.get(1).ok_or("fixture")?).err();
    assert_eq!(
        err,
        Some(crate::workflow::WorkflowError::LoopRegionCrossing {
            control: StepIdx::new(0),
            from: StepIdx::new(1),
            to: StepIdx::new(3),
        })
    );
    Ok(())
}

#[test]
fn direct_join_target_walk_is_unpaired() -> Result<(), Box<dyn std::error::Error>> {
    let parts = build_parts();
    let err = targets(&parts, parts.nodes.get(2).ok_or("fixture")?).err();
    assert_eq!(
        err,
        Some(crate::workflow::WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2),
        })
    );
    Ok(())
}

#[test]
fn join_error_handler_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    {
        let node = parts.nodes.get_mut(2).ok_or("fixture")?;
        node.routing.error = ErrorRouting::Handler {
            handler: StepIdx::new(3),
            error_slot: None,
        };
    }
    let node = parts.nodes.get(0).ok_or("fixture")?;
    let result = targets(&parts, node)?;
    assert_eq!(result.as_slice(), &[StepIdx::new(3), StepIdx::new(3)]);
    Ok(())
}
