//! Test fixtures for kind_refs numeric reference validation.

use crate::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx, WorkflowDigest};
use crate::workflow::{CompiledNodeKind, ResourceContract, WorkflowParts, node};

pub(super) fn foreach_start() -> CompiledNodeKind {
    CompiledNodeKind::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: FanoutLimit::new(2),
        at_once: BranchCount::new(2),
        body: StepIdx::new(1),
        join: StepIdx::new(3),
    }
}

pub(super) fn together_start() -> CompiledNodeKind {
    CompiledNodeKind::TogetherConcurrentStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(3),
    }
}

pub(super) fn first_success_start() -> CompiledNodeKind {
    CompiledNodeKind::FirstSuccessStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(3),
    }
}

pub(super) fn starts() -> [CompiledNodeKind; 3] {
    [together_start(), first_success_start(), foreach_start()]
}

pub(super) fn parts(kind: CompiledNodeKind) -> WorkflowParts {
    let start = node(StepIdx::new(0), kind, None, None);
    let lane = node(
        StepIdx::new(1),
        CompiledNodeKind::Nop,
        None,
        Some(StepIdx::new(2)),
    );
    let lane_end = node(
        StepIdx::new(2),
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(StepIdx::new(3)),
    );
    let join = node(
        StepIdx::new(3),
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(4)),
    );
    let finish = node(
        StepIdx::new(4),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );

    WorkflowParts {
        name: "kind_refs".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::new([start, lane, lane_end, join, finish]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
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
