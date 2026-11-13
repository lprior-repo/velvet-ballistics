use super::super::assert_accepted;
use super::super::{CompiledNodeKind, CompiledWorkflow, WorkflowError, n, parts};
use vb_core::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};

fn declare_list_input(p: &mut vb_core::workflow::WorkflowParts) {
    p.input_slots = Box::new([vb_core::workflow::CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: vb_core::workflow::InputSlotKind::List,
        constraint: vb_core::workflow::CompiledInputConstraint::required(),
    }]);
}

#[test]
fn valid_foreach_with_item_use_caches_three_frames() -> Result<(), WorkflowError> {
    let start = n(
        0,
        CompiledNodeKind::ForEachConcurrentStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: FanoutLimit::new(2),
            at_once: BranchCount::new(2),
            body: StepIdx::new(1),
            join: StepIdx::new(3),
        },
        None,
        None,
    );
    let lane = n(
        1,
        CompiledNodeKind::Copy {
            source: SlotIdx::new(1),
        },
        Some(1),
        Some(2),
    );
    let lane_end = n(
        2,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(1)),
        },
        None,
        Some(3),
    );
    let join = n(
        3,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(2),
        Some(4),
    );
    let finish = n(
        4,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
        None,
        None,
    );

    let mut p = parts(
        "valid_foreach",
        Box::new([start, lane, lane_end, join, finish]),
        3,
    );
    declare_list_input(&mut p);
    let submitted = p.clone();
    let wf = CompiledWorkflow::try_from_concurrent_parts(p)?;
    assert_accepted(&wf, &submitted, 3, 1);
    Ok(())
}

#[test]
fn valid_two_lane_capture_caches_three_frames() -> Result<(), WorkflowError> {
    let start = n(
        0,
        CompiledNodeKind::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
            join: StepIdx::new(5),
        },
        None,
        None,
    );
    let lane1 = n(
        1,
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
        Some(1),
        Some(2),
    );
    let end1 = n(
        2,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(1)),
        },
        None,
        Some(5),
    );
    let lane2 = n(
        3,
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
        Some(2),
        Some(4),
    );
    let end2 = n(
        4,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(2)),
        },
        None,
        Some(5),
    );
    let join = n(
        5,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(3),
        Some(6),
    );
    let finish = n(
        6,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(3),
        },
        None,
        None,
    );

    let mut p = parts(
        "valid_capture",
        Box::new([start, lane1, end1, lane2, end2, join, finish]),
        4,
    );
    p.input_slots = Box::new([vb_core::workflow::CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: vb_core::workflow::InputSlotKind::I64,
        constraint: vb_core::workflow::CompiledInputConstraint::required(),
    }]);
    let submitted = p.clone();
    let wf = CompiledWorkflow::try_from_concurrent_parts(p)?;
    assert_accepted(&wf, &submitted, 3, 1);
    Ok(())
}

#[test]
fn legacy_loop_inside_concurrent_lane_caches_correctly() -> Result<(), WorkflowError> {
    let together = n(
        0,
        CompiledNodeKind::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(6),
        },
        None,
        None,
    );
    let foreach = n(
        1,
        CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(2),
            limit: FanoutLimit::new(4),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(1),
        None,
    );
    let body = n(
        2,
        CompiledNodeKind::Copy {
            source: SlotIdx::new(2),
        },
        Some(3),
        Some(3),
    );
    let foreach_next = n(
        3,
        CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::new(1),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(2),
        None,
    );
    let foreach_join = n(
        4,
        CompiledNodeKind::ForEachJoin {
            output: SlotIdx::new(1),
        },
        Some(3),
        Some(5),
    );
    let lane_end = n(
        5,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(6),
    );
    let together_join = n(
        6,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(4),
        Some(7),
    );
    let finish = n(
        7,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(4),
        },
        None,
        None,
    );

    let mut p = parts(
        "legacy_in_lane",
        Box::new([
            together,
            foreach,
            body,
            foreach_next,
            foreach_join,
            lane_end,
            together_join,
            finish,
        ]),
        5,
    );
    declare_list_input(&mut p);
    let submitted = p.clone();
    let wf = CompiledWorkflow::try_from_concurrent_parts(p)?;
    assert_accepted(&wf, &submitted, 2, 1);
    Ok(())
}

#[test]
fn valid_lane_end_result_caches_three_frames() -> Result<(), WorkflowError> {
    let submitted = super::lane_end_parts(1);
    let wf = CompiledWorkflow::try_from_concurrent_parts(submitted.clone())?;
    assert_accepted(&wf, &submitted, 3, 1);
    Ok(())
}
