use super::{
    CompiledNodeKind as K, CompiledWorkflow, WorkflowError, assert_accepted, declare_input, n,
    parts,
};
use vb_core::execution_lanes::GroupMode;
use vb_core::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};
use vb_core::workflow::{ResourceContract, WorkflowParts};

fn fixed_sum(root: GroupMode) -> WorkflowParts {
    let start = match root {
        GroupMode::TogetherOrdered => K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(6)]),
            join: StepIdx::new(11),
        },
        GroupMode::FirstSuccess => K::FirstSuccessStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(6)]),
            join: StepIdx::new(11),
        },
        GroupMode::ForEachOrdered => K::Nop,
    };
    let foreach = |item, body, join| K::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(item),
        limit: FanoutLimit::new(4),
        at_once: BranchCount::new(4),
        body: StepIdx::new(body),
        join: StepIdx::new(join),
    };
    let mut submitted = parts(
        "fixed_sum",
        Box::new([
            n(0, start, None, None),
            n(1, foreach(1, 2, 4), None, None),
            n(2, K::Nop, None, Some(3)),
            n(
                3,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(1),
                    result: Some(SlotIdx::new(1)),
                },
                None,
                Some(4),
            ),
            n(
                4,
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(1),
                },
                Some(2),
                Some(5),
            ),
            n(
                5,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(0),
                    result: Some(SlotIdx::new(2)),
                },
                None,
                Some(11),
            ),
            n(6, foreach(3, 7, 9), None, None),
            n(7, K::Nop, None, Some(8)),
            n(
                8,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(6),
                    result: Some(SlotIdx::new(3)),
                },
                None,
                Some(9),
            ),
            n(
                9,
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(6),
                },
                Some(4),
                Some(10),
            ),
            n(
                10,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(0),
                    result: Some(SlotIdx::new(4)),
                },
                None,
                Some(11),
            ),
            n(
                11,
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(0),
                },
                Some(5),
                Some(12),
            ),
            n(
                12,
                K::Finish {
                    result: SlotIdx::new(5),
                },
                None,
                None,
            ),
        ]),
        6,
    );
    declare_input(&mut submitted);
    submitted
}

fn assert_fixed_sum(root: GroupMode) -> Result<(), WorkflowError> {
    let submitted = fixed_sum(root);
    let wf = CompiledWorkflow::try_from_concurrent_parts(submitted.clone())?;
    assert_accepted(&wf, &submitted, 11, 3);
    Ok(())
}

#[test]
fn fixed_multibranch_together_caches_peak() -> Result<(), WorkflowError> {
    assert_fixed_sum(GroupMode::TogetherOrdered)
}

#[test]
fn fixed_multibranch_first_success_caches_peak() -> Result<(), WorkflowError> {
    assert_fixed_sum(GroupMode::FirstSuccess)
}

#[test]
fn reachable_fanout_cap_caches_within_bounds() -> Result<(), WorkflowError> {
    let mut submitted = parts(
        "reachable_cap",
        Box::new([
            n(
                0,
                K::ForEachConcurrentStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: FanoutLimit::new(4),
                    at_once: BranchCount::new(4),
                    body: StepIdx::new(1),
                    join: StepIdx::new(3),
                },
                None,
                None,
            ),
            n(1, K::Nop, None, Some(2)),
            n(
                2,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(0),
                    result: None,
                },
                None,
                Some(3),
            ),
            n(
                3,
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(0),
                },
                Some(1),
                Some(4),
            ),
            n(
                4,
                K::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]),
        2,
    );
    declare_input(&mut submitted);
    submitted.resource_contract = ResourceContract {
        max_fanout: 4,
        ..ResourceContract::DEFAULT
    };
    let wf = CompiledWorkflow::try_from_concurrent_parts(submitted.clone())?;
    assert_accepted(&wf, &submitted, 5, 1);
    Ok(())
}

#[test]
fn two_lane_capture_caches_three_frames_one_group() -> Result<(), WorkflowError> {
    let start = n(
        0,
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
            join: StepIdx::new(5),
        },
        None,
        None,
    );
    let lane1 = n(
        1,
        K::Copy {
            source: SlotIdx::new(0),
        },
        Some(1),
        Some(2),
    );
    let end1 = n(
        2,
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(1)),
        },
        None,
        Some(5),
    );
    let lane2 = n(
        3,
        K::Copy {
            source: SlotIdx::new(0),
        },
        Some(2),
        Some(4),
    );
    let end2 = n(
        4,
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(2)),
        },
        None,
        Some(5),
    );
    let join = n(
        5,
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(3),
        Some(6),
    );
    let finish = n(
        6,
        K::Finish {
            result: SlotIdx::new(3),
        },
        None,
        None,
    );
    let mut submitted = parts(
        "two_lane_capture",
        Box::new([start, lane1, end1, lane2, end2, join, finish]),
        4,
    );
    submitted.input_slots = Box::new([vb_core::workflow::CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: vb_core::workflow::InputSlotKind::I64,
        constraint: vb_core::workflow::CompiledInputConstraint::required(),
    }]);
    let wf = CompiledWorkflow::try_from_concurrent_parts(submitted.clone())?;
    assert_accepted(&wf, &submitted, 3, 1);
    Ok(())
}
