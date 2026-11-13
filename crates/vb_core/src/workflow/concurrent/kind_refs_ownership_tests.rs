use super::envelope::whole;
use super::local::base;
use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::{CompiledNodeKind as K, WorkflowError, WorkflowParts, node};

fn five_parts(branches: Box<[StepIdx]>, first: K, next: StepIdx) -> WorkflowParts {
    let mut p = base();
    p.nodes = Box::new([
        node(
            StepIdx::new(0),
            K::TogetherConcurrentStart {
                branches,
                join: StepIdx::new(3),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), first, None, Some(next)),
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
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(4)),
        ),
        node(
            StepIdx::new(4),
            K::Finish {
                result: SlotIdx::new(1),
            },
            None,
            None,
        ),
    ]);
    p
}

#[test]
fn duplicate_node_ids_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut p = base();
    let n = p
        .nodes
        .get_mut(StepIdx::new(1).as_usize())
        .ok_or("fixture")?;
    n.id = StepIdx::new(0);
    let r = whole(&p);
    match r {
        Err(WorkflowError::NodeIdMismatch { expected, actual })
            if expected == StepIdx::new(1) && actual == StepIdx::new(0) => {}
        other => return Err(format!("expected NodeIdMismatch; got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn duplicate_branch_entries_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut p = base();
    let n = p
        .nodes
        .get_mut(StepIdx::new(0).as_usize())
        .ok_or("fixture")?;
    n.kind = K::TogetherConcurrentStart {
        branches: Box::new([StepIdx::new(1), StepIdx::new(1)]),
        join: StepIdx::new(2),
    };
    let r = whole(&p);
    match r {
        Err(WorkflowError::LoopRegionUnpaired { control }) if control == StepIdx::new(0) => {}
        other => return Err(format!("expected LoopRegionUnpaired(Step0); got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn duplicate_group_join_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut p = base();
    let mut nodes = p.nodes.into_vec();
    nodes.try_reserve_exact(2)?;
    nodes.push(node(
        StepIdx::new(4),
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(5)),
    ));
    nodes.push(node(
        StepIdx::new(5),
        K::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    ));
    p.nodes = nodes.into_boxed_slice();
    let r = whole(&p);
    match r {
        Err(WorkflowError::LoopRegionUnpaired { control }) if control == StepIdx::new(4) => {}
        other => return Err(format!("expected LoopRegionUnpaired(Step4); got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn cross_lane_escape_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let p = five_parts(
        Box::new([StepIdx::new(1), StepIdx::new(2)]),
        K::Nop,
        StepIdx::new(2),
    );
    let r = whole(&p);
    match r {
        Err(WorkflowError::LoopRegionCrossing { control, from, to })
            if control == StepIdx::new(0) && from == StepIdx::new(1) && to == StepIdx::new(2) => {}
        other => return Err(format!("expected LoopRegionCrossing; got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn unreachable_new_lane_end_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let p = five_parts(
        Box::new([StepIdx::new(1)]),
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        StepIdx::new(3),
    );
    let r = whole(&p);
    match r {
        Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(2) => {}
        other => return Err(format!("expected UnreachableNode(Step2); got {:?}", other).into()),
    }
    Ok(())
}
