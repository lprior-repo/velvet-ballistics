use super::{
    CompiledNodeKind, CompiledWorkflow, WorkflowError, assert_accepted, declare_input, n, parts,
};
use vb_core::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};

fn foreach_start(limit: u32, at_once: u16) -> CompiledNodeKind {
    CompiledNodeKind::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: FanoutLimit::new(limit),
        at_once: BranchCount::new(at_once),
        body: StepIdx::new(1),
        join: StepIdx::new(3),
    }
}

#[test]
fn simple_foreach_caches_three_frames() -> Result<(), WorkflowError> {
    let start = n(0, foreach_start(2, 2), None, None);
    let lane = n(1, CompiledNodeKind::Nop, None, Some(2));
    let lane_end = n(
        2,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(3),
    );
    let join = n(
        3,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(1),
        Some(4),
    );
    let finish = n(
        4,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    );

    let mut p = parts(
        "foreach",
        Box::new([start, lane, lane_end, join, finish]),
        2,
    );
    declare_input(&mut p);
    let submitted = p.clone();
    let wf = CompiledWorkflow::try_from_concurrent_parts(p)?;
    assert_accepted(&wf, &submitted, 3, 1);
    Ok(())
}

#[test]
fn zero_limit_foreach_caches_one_frame_one_group() -> Result<(), WorkflowError> {
    let start = n(0, foreach_start(0, 2), None, None);
    let lane = n(1, CompiledNodeKind::Nop, None, Some(2));
    let lane_end = n(
        2,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(3),
    );
    let join = n(
        3,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(1),
        Some(4),
    );
    let finish = n(
        4,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    );

    let mut p = parts(
        "foreach0",
        Box::new([start, lane, lane_end, join, finish]),
        2,
    );
    declare_input(&mut p);
    let submitted = p.clone();
    let wf = CompiledWorkflow::try_from_concurrent_parts(p)?;
    assert_accepted(&wf, &submitted, 1, 1);
    Ok(())
}

#[test]
fn foreach_limit_less_than_at_once_caches_effective_frames() -> Result<(), WorkflowError> {
    let start = n(0, foreach_start(2, 4), None, None);
    let lane = n(1, CompiledNodeKind::Nop, None, Some(2));
    let lane_end = n(
        2,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(3),
    );
    let join = n(
        3,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(1),
        Some(4),
    );
    let finish = n(
        4,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    );

    let mut p = parts(
        "foreach_low_limit",
        Box::new([start, lane, lane_end, join, finish]),
        2,
    );
    declare_input(&mut p);
    let submitted = p.clone();
    let wf = CompiledWorkflow::try_from_concurrent_parts(p)?;
    assert_accepted(&wf, &submitted, 3, 1);
    Ok(())
}

#[path = "cache_nested.rs"]
mod cache_nested;
