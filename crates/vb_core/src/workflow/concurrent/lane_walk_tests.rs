use super::*;
use crate::workflow::concurrent_lane_walk::{check_start, walk_lane};

#[test]
fn own_end_is_recorded() -> Result<(), Box<dyn std::error::Error>> {
    let parts = build_parts();
    let (ends, children, _suspension) = walk_lane(&parts, StepIdx::new(0), StepIdx::new(1))?;
    assert_eq!(ends.as_slice(), &[StepIdx::new(1)]);
    assert!(children.is_empty());
    check_start(&parts, StepIdx::new(0))?;
    Ok(())
}

#[test]
fn missing_end_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let node1 = parts.nodes.get_mut(1).ok_or("fixture")?;
    node1.kind = K::Nop;
    node1.routing.next = None.into();
    let walk_result = walk_lane(&parts, StepIdx::new(0), StepIdx::new(1));
    assert_eq!(
        walk_result.map(|(ends, children, susp)| (ends, children, susp)),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(0)
        })
    );
    let compiled_result =
        crate::workflow::CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ());
    assert_eq!(
        compiled_result,
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(0)
        })
    );
    Ok(())
}

#[test]
fn branch_escape_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let node1 = parts.nodes.get_mut(1).ok_or("fixture")?;
    node1.kind = K::Nop;
    node1.routing.next = Some(StepIdx::new(3)).into();
    let walk_result = walk_lane(&parts, StepIdx::new(0), StepIdx::new(1));
    assert_eq!(
        walk_result.map(|(ends, children, susp)| (ends, children, susp)),
        Err(WorkflowError::LoopRegionCrossing {
            control: StepIdx::new(0),
            from: StepIdx::new(1),
            to: StepIdx::new(3)
        })
    );
    let compiled_result =
        crate::workflow::CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ());
    assert_eq!(
        compiled_result,
        Err(WorkflowError::LoopRegionCrossing {
            control: StepIdx::new(0),
            from: StepIdx::new(1),
            to: StepIdx::new(3)
        })
    );
    Ok(())
}

#[test]
fn arbitrary_cycle_is_rejected_by_production_forward_gate() -> Result<(), Box<dyn std::error::Error>>
{
    let mut parts = build_parts();
    let node1 = parts.nodes.get_mut(1).ok_or("fixture")?;
    node1.kind = K::Nop;
    node1.routing.next = Some(StepIdx::new(1)).into();
    let walk_result = walk_lane(&parts, StepIdx::new(0), StepIdx::new(1));
    assert_eq!(
        walk_result.map(|(ends, children, susp)| (ends, children, susp)),
        Err(WorkflowError::BackwardEdge {
            from: StepIdx::new(1),
            to: StepIdx::new(1)
        })
    );
    Ok(())
}
