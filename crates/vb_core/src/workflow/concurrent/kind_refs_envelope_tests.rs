use super::local::base;
use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::{WorkflowError, WorkflowParts, concurrent_analyze};

pub(super) fn whole(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    concurrent_analyze::analyze(parts).map(|_| ())
}

#[test]
fn starts_and_ends_forbid_output() -> Result<(), Box<dyn std::error::Error>> {
    for step in [StepIdx::new(0), StepIdx::new(1)] {
        let mut parts = base();
        let node = parts.nodes.get_mut(step.as_usize()).ok_or("fixture")?;
        node.output = Some(SlotIdx::new(1)).into();
        assert_eq!(
            whole(&parts),
            Err(WorkflowError::LoopRegionUnpaired { control: step })
        );
    }
    Ok(())
}

#[test]
fn join_requires_output() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = base();
    let node = parts.nodes.get_mut(2).ok_or("fixture")?;
    node.output = None.into();
    assert_eq!(
        whole(&parts),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2)
        })
    );
    Ok(())
}

#[test]
fn join_output_is_bounded() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = base();
    let node = parts.nodes.get_mut(2).ok_or("fixture")?;
    node.output = Some(SlotIdx::new(2)).into();
    assert_eq!(
        whole(&parts),
        Err(WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(2)
        })
    );
    Ok(())
}
