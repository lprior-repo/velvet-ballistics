use super::envelope::whole;
use super::local::base;
use crate::ids::StepIdx;
use crate::workflow::{CompiledNodeKind as K, WorkflowError, WorkflowParts};

fn plain(parts: &mut WorkflowParts, step: StepIdx) -> Result<(), Box<dyn std::error::Error>> {
    let n = parts.nodes.get_mut(step.as_usize()).ok_or("fixture")?;
    n.kind = K::Nop;
    n.routing.next = Some(StepIdx::new(3)).into();
    Ok(())
}

#[test]
fn orphan_end_without_any_start() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = base();
    plain(&mut parts, StepIdx::new(0))?;
    plain(&mut parts, StepIdx::new(2))?;
    assert_eq!(
        whole(&parts),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(0)
        })
    );
    Ok(())
}

#[test]
fn orphan_join_without_any_start() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = base();
    plain(&mut parts, StepIdx::new(0))?;
    plain(&mut parts, StepIdx::new(1))?;
    assert_eq!(
        whole(&parts),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(0)
        })
    );
    Ok(())
}

#[test]
fn wrong_lane_owner_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = base();
    let n = parts.nodes.get_mut(1).ok_or("fixture")?;
    n.kind = K::ConcurrentLaneEnd {
        owner: StepIdx::new(2),
        result: None,
    };
    assert_eq!(
        whole(&parts),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2)
        })
    );
    Ok(())
}

#[test]
fn wrong_join_owner_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = base();
    let n = parts.nodes.get_mut(2).ok_or("fixture")?;
    n.kind = K::ConcurrentGroupJoin {
        owner: StepIdx::new(1),
    };
    assert_eq!(
        whole(&parts),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2)
        })
    );
    Ok(())
}
