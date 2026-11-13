use super::*;
use crate::workflow::concurrent_ownership::owners;

#[test]
fn own_scopes_are_source_bound() -> Result<(), Box<dyn std::error::Error>> {
    let parts = build_parts();
    let map = owners(&parts)?;
    assert_eq!(
        map.as_slice(),
        &[None, Some((StepIdx::new(0), StepIdx::new(1))), None, None]
    );
    Ok(())
}

#[test]
fn root_cannot_enter_lane_body() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    parts.entry = StepIdx::new(1);
    let r = owners(&parts);
    let err = r.err();
    assert_eq!(
        err,
        Some(WorkflowError::LoopRegionCrossing {
            control: StepIdx::new(0),
            from: StepIdx::new(1),
            to: StepIdx::new(1)
        })
    );
    let r2 = crate::workflow::CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ());
    let err2 = r2.err();
    assert_eq!(
        err2,
        Some(WorkflowError::LoopRegionCrossing {
            control: StepIdx::new(0),
            from: StepIdx::new(1),
            to: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn root_cannot_enter_join() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    parts.entry = StepIdx::new(2);
    let r = owners(&parts);
    let err = r.err();
    assert_eq!(
        err,
        Some(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2)
        })
    );
    Ok(())
}

#[test]
fn detached_root_node_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let mut nodes = parts.nodes.into_vec();
    nodes.push(node(StepIdx::new(4), K::Nop, None, None));
    parts.nodes = nodes.into_boxed_slice();
    let r = owners(&parts);
    let err = r.err();
    assert_eq!(
        err,
        Some(WorkflowError::UnreachableNode {
            step: StepIdx::new(4)
        })
    );
    Ok(())
}
