use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind as K, InputSlotKind,
    ResourceContract, WorkflowParts, node,
};

fn lane() -> CompiledNode {
    node(
        StepIdx::new(1),
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(StepIdx::new(2)),
    )
}

fn join() -> CompiledNode {
    node(
        StepIdx::new(2),
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(3)),
    )
}

fn finish() -> CompiledNode {
    node(
        StepIdx::new(3),
        K::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    )
}

fn nodes(kind: K) -> Box<[CompiledNode]> {
    Box::new([
        node(StepIdx::new(0), kind, None, None),
        lane(),
        join(),
        finish(),
    ])
}

fn inputs() -> Box<[CompiledInputSlot]> {
    Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::List,
        constraint: CompiledInputConstraint::required(),
    }])
}

pub(super) fn parts(kind: K) -> WorkflowParts {
    let n = nodes(kind);
    WorkflowParts {
        name: "kind-refs".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: n,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        step_names: Box::new([]),
        input_slots: inputs(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    }
}
