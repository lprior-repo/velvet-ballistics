use crate::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};
use crate::workflow::validate_node_kind::validate_node_kind;
use crate::workflow::{CompiledNodeKind as K, ContractResource, WorkflowError, WorkflowParts};

pub(super) fn base() -> WorkflowParts {
    let mut parts = super::fixture::parts(K::TogetherConcurrentStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(2),
    });
    parts.nodes = Box::new([
        crate::workflow::node(
            StepIdx::new(0),
            K::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(1)]),
                join: StepIdx::new(2),
            },
            None,
            None,
        ),
        crate::workflow::node(
            StepIdx::new(1),
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: None,
            },
            None,
            Some(StepIdx::new(2)),
        ),
        crate::workflow::node(
            StepIdx::new(2),
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(3)),
        ),
        crate::workflow::node(
            StepIdx::new(3),
            K::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ]);
    parts.step_names = ["start", "end", "join", "finish"]
        .into_iter()
        .map(Into::into)
        .collect();
    parts
}

fn numeric(kind: K, expected: WorkflowError) {
    assert_eq!(validate_node_kind(&kind, &base()), Err(expected));
}

#[test]
fn lane_result_slot_is_bounded() {
    numeric(
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(2)),
        },
        WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(2),
        },
    );
}

#[test]
fn group_owner_is_bounded() {
    numeric(
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(4),
        },
        WorkflowError::StepOutOfBounds {
            step: StepIdx::new(4),
        },
    );
}

#[test]
fn empty_first_success_is_rejected() {
    numeric(
        K::FirstSuccessStart {
            branches: Box::new([]),
            join: StepIdx::new(2),
        },
        WorkflowError::EmptyBranchTable,
    );
}

#[test]
fn zero_foreach_cap_is_rejected() {
    numeric(
        K::ForEachConcurrentStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(0),
            limit: FanoutLimit::new(2),
            at_once: BranchCount::new(0),
            body: StepIdx::new(1),
            join: StepIdx::new(2),
        },
        WorkflowError::ResourceContractExceeded {
            resource: ContractResource::BranchCount,
        },
    );
}
