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

#[test]
fn concurrent_ctor_rejects_nested_together_at_any_depth() -> Result<(), Box<dyn std::error::Error>>
{
    let mut parts = build_parts();
    parts.slot_count = 2;
    parts.nodes = Box::new([
        node(
            StepIdx::new(0),
            K::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(1)]),
                join: StepIdx::new(7),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), K::Nop, None, Some(StepIdx::new(2))),
        node(
            StepIdx::new(2),
            K::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(3)]),
                join: StepIdx::new(5),
            },
            None,
            None,
        ),
        node(StepIdx::new(3), K::Nop, None, Some(StepIdx::new(4))),
        node(
            StepIdx::new(4),
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(2),
                result: None,
            },
            None,
            Some(StepIdx::new(5)),
        ),
        node(
            StepIdx::new(5),
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(2),
            },
            Some(crate::ids::SlotIdx::new(0)),
            Some(StepIdx::new(6)),
        ),
        node(
            StepIdx::new(6),
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: None,
            },
            None,
            Some(StepIdx::new(7)),
        ),
        node(
            StepIdx::new(7),
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(0),
            },
            Some(crate::ids::SlotIdx::new(1)),
            Some(StepIdx::new(8)),
        ),
        node(
            StepIdx::new(8),
            K::Finish {
                result: crate::ids::SlotIdx::new(1),
            },
            None,
            None,
        ),
    ]);
    assert_eq!(
        crate::workflow::CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(WorkflowError::NestedTogether {
            outer: StepIdx::new(0),
            inner: StepIdx::new(2),
        })
    );
    Ok(())
}

#[test]
fn concurrent_ctor_rejects_suspending_lane_nodes_with_typed_error()
-> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        K::Do {
            action: crate::ids::ActionId::new(0),
            input: crate::ids::SlotIdx::new(0),
        },
        K::WaitUntil {
            deadline_slot: crate::ids::SlotIdx::new(0),
        },
        K::WaitEvent {
            event: crate::ids::SlotIdx::new(0),
            timeout_slot: crate::workflow::Deadline::Unset,
        },
        K::Ask {
            prompt: crate::ids::SlotIdx::new(0),
            timeout_slot: crate::workflow::Deadline::Unset,
        },
    ];
    for kind in cases {
        let mut parts = build_parts();
        parts.slot_count = 2;
        parts.nodes = Box::new([
            node(
                StepIdx::new(0),
                K::TogetherConcurrentStart {
                    branches: Box::new([StepIdx::new(1)]),
                    join: StepIdx::new(3),
                },
                None,
                None,
            ),
            node(StepIdx::new(1), kind, None, Some(StepIdx::new(2))),
            node(
                StepIdx::new(2),
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(0),
                    result: None,
                },
                None,
                Some(StepIdx::new(3)),
            ),
            node(
                StepIdx::new(3),
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(0),
                },
                Some(crate::ids::SlotIdx::new(1)),
                Some(StepIdx::new(4)),
            ),
            node(
                StepIdx::new(4),
                K::Finish {
                    result: crate::ids::SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]);
        assert_eq!(
            crate::workflow::CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
            Err(WorkflowError::ConcurrentLaneSuspension {
                owner: StepIdx::new(0),
                step: StepIdx::new(1),
            })
        );
    }
    Ok(())
}
