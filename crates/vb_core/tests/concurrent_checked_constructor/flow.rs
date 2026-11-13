use super::{CompiledNodeKind as K, CompiledWorkflow, WorkflowError, n, parts};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, InputSlotKind, SlotFlowError, SlotFlowFailure,
};

fn capture_parts(second_source: u16, finish_slot: u16) -> vb_core::workflow::WorkflowParts {
    let start = n(
        0,
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
            join: StepIdx::new(5),
        },
        None,
        None,
    );
    let left = n(
        1,
        K::Copy {
            source: SlotIdx::new(0),
        },
        Some(1),
        Some(2),
    );
    let left_end = n(
        2,
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(1)),
        },
        None,
        Some(5),
    );
    let right = n(
        3,
        K::Copy {
            source: SlotIdx::new(second_source),
        },
        Some(2),
        Some(4),
    );
    let right_end = n(
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
            result: SlotIdx::new(finish_slot),
        },
        None,
        None,
    );
    let mut submitted = parts(
        "lane_flow",
        Box::new([start, left, left_end, right, right_end, join, finish]),
        4,
    );
    submitted.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::I64,
        constraint: CompiledInputConstraint::required(),
    }]);
    submitted
}

#[test]
fn sibling_lane_read_other_lane_local_rejected() {
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(capture_parts(1, 3)),
        Err(WorkflowError::SlotFlow(SlotFlowError {
            step: StepIdx::new(3),
            slot: Some(SlotIdx::new(1)),
            reason: SlotFlowFailure::UninitializedRead,
        }))
    );
}

#[test]
fn parent_read_child_local_rejected() {
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(capture_parts(0, 1)),
        Err(WorkflowError::SlotFlow(SlotFlowError {
            step: StepIdx::new(6),
            slot: Some(SlotIdx::new(1)),
            reason: SlotFlowFailure::UninitializedRead,
        }))
    );
}

fn lane_end_parts(result: u16) -> vb_core::workflow::WorkflowParts {
    let mut submitted = parts(
        "lane_end_result",
        Box::new([
            n(
                0,
                K::ForEachConcurrentStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: vb_core::ids::FanoutLimit::new(2),
                    at_once: vb_core::ids::BranchCount::new(2),
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
                    result: Some(SlotIdx::new(result)),
                },
                None,
                Some(3),
            ),
            n(
                3,
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(0),
                },
                Some(3),
                Some(4),
            ),
            n(
                4,
                K::Finish {
                    result: SlotIdx::new(3),
                },
                None,
                None,
            ),
        ]),
        4,
    );
    submitted.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::List,
        constraint: CompiledInputConstraint::required(),
    }]);
    submitted
}

#[test]
fn lane_end_result_slot_uninitialized_rejected() {
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(lane_end_parts(2)),
        Err(WorkflowError::SlotFlow(SlotFlowError {
            step: StepIdx::new(2),
            slot: Some(SlotIdx::new(2)),
            reason: SlotFlowFailure::UninitializedRead,
        }))
    );
}

#[path = "flow_valid.rs"]
mod flow_valid;
