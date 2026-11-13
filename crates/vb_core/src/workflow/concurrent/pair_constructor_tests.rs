use super::*;

#[test]
fn concurrent_ctor_rejects_join_at_len() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    {
        let n = parts.nodes.get_mut(0).ok_or("missing fixture")?;
        n.kind = K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(4),
        };
    }
    assert_eq!(
        crate::workflow::CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(4)
        })
    );
    Ok(())
}

#[test]
fn concurrent_ctor_rejects_backward_join() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    {
        let n = parts.nodes.get_mut(0).ok_or("missing fixture")?;
        n.kind = K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(0),
        };
    }
    assert_eq!(
        crate::workflow::CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(WorkflowError::BackwardEdge {
            from: StepIdx::new(0),
            to: StepIdx::new(0)
        })
    );
    Ok(())
}

#[test]
fn concurrent_ctor_rejects_missing_owner() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    {
        let n = parts.nodes.get_mut(2).ok_or("missing fixture")?;
        n.kind = K::ConcurrentGroupJoin {
            owner: StepIdx::new(1),
        };
    }
    assert_eq!(
        crate::workflow::CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2)
        })
    );
    Ok(())
}

#[test]
fn legacy_ctor_rejects_concurrent_profile() -> Result<(), Box<dyn std::error::Error>> {
    let parts = build_parts();
    assert_eq!(
        crate::workflow::CompiledWorkflow::try_from_parts(parts).map(|_| ()),
        Err(WorkflowError::ConcurrentProfileRequired {
            step: StepIdx::new(0)
        })
    );
    Ok(())
}

#[test]
fn concurrent_ctor_accepts_minimal_pair() -> Result<(), Box<dyn std::error::Error>> {
    let parts = build_parts();
    let workflow = crate::workflow::CompiledWorkflow::try_from_concurrent_parts(parts)?;
    assert_eq!(
        workflow.ir_profile(),
        crate::workflow::WorkflowIrProfile::ConcurrentV3
    );
    let requirements = workflow.concurrent_requirements().ok_or("cache")?;
    assert_eq!(requirements.frames(), 2);
    assert_eq!(requirements.groups(), 1);
    Ok(())
}
