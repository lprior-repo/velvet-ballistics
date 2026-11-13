use crate::ids::{SlotIdx, StepIdx, WorkflowDigest};
use crate::workflow::{
    CompiledNodeKind as K, CompiledWorkflow, ResourceContract, WorkflowError, WorkflowParts, node,
};

fn n(id: u16, kind: K, out: Option<u16>, next: Option<u16>) -> crate::workflow::CompiledNode {
    node(
        StepIdx::new(id),
        kind,
        out.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

fn valid_together_parts() -> WorkflowParts {
    let start = n(
        0,
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(3),
        },
        None,
        None,
    );
    let lane = n(1, K::Nop, None, Some(2));
    let lane_end = n(
        2,
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(3),
    );
    let join = n(
        3,
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(0),
        Some(4),
    );
    let finish = n(
        4,
        K::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    WorkflowParts {
        name: "test".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::new([start, lane, lane_end, join, finish]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([
            "start".into(),
            "lane".into(),
            "end".into(),
            "join".into(),
            "finish".into(),
        ]),
        input_slots: Box::new([]),
    }
}

#[test]
fn checked_ctor_accepts_valid_together_concurrent() -> Result<(), Box<dyn std::error::Error>> {
    let parts = valid_together_parts();
    let wf = CompiledWorkflow::try_from_concurrent_parts(parts)?;
    assert_eq!(
        wf.ir_profile(),
        crate::workflow::WorkflowIrProfile::ConcurrentV3
    );
    let reqs = wf.concurrent_requirements().ok_or("missing reqs")?;
    assert!(reqs.frames() > 0);
    assert!(reqs.groups() > 0);
    Ok(())
}

#[test]
fn checked_ctor_rejects_join_at_len() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = valid_together_parts();
    parts.nodes.get_mut(0).ok_or("fixture")?.kind = K::TogetherConcurrentStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(99),
    };
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99)
        })
    );
    Ok(())
}

#[test]
fn checked_ctor_rejects_empty_branches() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = valid_together_parts();
    parts.nodes.get_mut(0).ok_or("fixture")?.kind = K::TogetherConcurrentStart {
        branches: Box::new([]),
        join: StepIdx::new(3),
    };
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(WorkflowError::EmptyBranchTable)
    );
    Ok(())
}

#[test]
fn checked_ctor_rejects_missing_owner() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = valid_together_parts();
    parts.nodes.get_mut(2).ok_or("fixture")?.kind = K::ConcurrentLaneEnd {
        owner: StepIdx::new(99),
        result: None,
    };
    let res = CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ());
    assert_eq!(
        res,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99)
        })
    );
    Ok(())
}

#[test]
fn checked_ctor_rejects_group_join_wrong_owner() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = valid_together_parts();
    // Isolate numeric owner bounds before any start's backlink check.
    parts.nodes = Box::new([
        n(
            0,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(99),
            },
            Some(0),
            Some(1),
        ),
        n(
            1,
            K::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ]);
    parts.step_names = ["join", "finish"].into_iter().map(Into::into).collect();
    let res = CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ());
    assert_eq!(
        res,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99)
        })
    );
    Ok(())
}

#[test]
fn legacy_ctor_still_rejects_concurrent_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let parts = valid_together_parts();
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts).map(|_| ()),
        Err(WorkflowError::ConcurrentProfileRequired {
            step: StepIdx::new(0)
        })
    );
    Ok(())
}

#[test]
fn checked_ctor_accepts_first_success() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = valid_together_parts();
    parts.nodes.get_mut(0).ok_or("fixture")?.kind = K::FirstSuccessStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(3),
    };
    let wf = CompiledWorkflow::try_from_concurrent_parts(parts)?;
    assert_eq!(
        wf.ir_profile(),
        crate::workflow::WorkflowIrProfile::ConcurrentV3
    );
    Ok(())
}

// ORIGINAL registered cases restored using original 4-node super::build_parts() geometry

#[test]
fn concurrent_ctor_rejects_join_at_len() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = super::build_parts();
    parts.nodes.get_mut(0).ok_or("fixture")?.kind = K::TogetherConcurrentStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(4),
    };
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(4)
        })
    );
    Ok(())
}

#[test]
fn concurrent_ctor_rejects_backward_join() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = super::build_parts();
    parts.nodes.get_mut(0).ok_or("fixture")?.kind = K::TogetherConcurrentStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(0),
    };
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(WorkflowError::BackwardEdge {
            from: StepIdx::new(0),
            to: StepIdx::new(0)
        })
    );
    Ok(())
}

#[test]
fn concurrent_ctor_rejects_missing_owner() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = super::build_parts();
    parts.nodes.get_mut(2).ok_or("fixture")?.kind = K::ConcurrentGroupJoin {
        owner: StepIdx::new(1),
    };
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2)
        })
    );
    Ok(())
}

#[test]
fn legacy_ctor_rejects_concurrent_profile() -> Result<(), Box<dyn std::error::Error>> {
    let parts = super::build_parts();
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts).map(|_| ()),
        Err(WorkflowError::ConcurrentProfileRequired {
            step: StepIdx::new(0)
        })
    );
    Ok(())
}

#[test]
fn concurrent_ctor_keeps_bootstrap_barrier() -> Result<(), Box<dyn std::error::Error>> {
    // Paired Together geometry with 4 nodes: start(0)->lane_end(1)->join(2)->finish(3).
    // Valid paired loop region passes the checked constructor.
    let parts = super::build_parts();
    let parts_clone = parts.clone();
    let wf = CompiledWorkflow::try_from_concurrent_parts(parts)?;
    assert_eq!(
        wf.ir_profile(),
        crate::workflow::WorkflowIrProfile::ConcurrentV3
    );
    let reqs = wf.concurrent_requirements().ok_or("missing reqs")?;
    assert_eq!(reqs.frames(), 2);
    assert_eq!(reqs.groups(), 1);
    assert_eq!(wf.name(), "pair");
    let reconstructed = wf.to_parts();
    assert_eq!(reconstructed, parts_clone);
    Ok(())
}
