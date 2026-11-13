//! Checked `ConcurrentV3` construction: positive geometry, cached requirements,
//! and typed rejection of malformed graphs. Legacy `V2` construction is unchanged.

use vb_core::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind as K, CompiledWorkflow, WorkflowError, WorkflowIrProfile,
    WorkflowParts, node,
};
use vb_core::{ResourceContract, WorkflowDigest};

fn n(id: u16, kind: K, out: Option<u16>, next: Option<u16>) -> CompiledNode {
    node(
        StepIdx::new(id),
        kind,
        out.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

fn parts(nodes: Box<[CompiledNode]>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("concurrent-acceptance"),
        digest: WorkflowDigest::from_bytes([7u8; 32]),
        semantic_source_identity: Some(WorkflowDigest::from_bytes([9u8; 32])),
        source_metadata: None,
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(0), ConstValue::I64(1)]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    }
}

fn together_nodes() -> Box<[CompiledNode]> {
    Box::new([
        n(
            0,
            K::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
                join: StepIdx::new(5),
            },
            None,
            None,
        ),
        n(
            1,
            K::SetConst {
                value: vb_core::ConstIdx::new(0),
            },
            Some(1),
            Some(2),
        ),
        n(
            2,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: Some(SlotIdx::new(1)),
            },
            None,
            Some(5),
        ),
        n(
            3,
            K::SetConst {
                value: vb_core::ConstIdx::new(1),
            },
            Some(1),
            Some(4),
        ),
        n(
            4,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: Some(SlotIdx::new(1)),
            },
            None,
            Some(5),
        ),
        n(
            5,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(0),
            },
            Some(2),
            Some(6),
        ),
        n(
            6,
            K::Finish {
                result: SlotIdx::new(2),
            },
            None,
            None,
        ),
    ])
}

fn zero_work_foreach_nodes() -> Box<[CompiledNode]> {
    Box::new([
        n(
            0,
            K::ForEachConcurrentStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: FanoutLimit::new(0),
                at_once: BranchCount::new(2),
                body: StepIdx::new(1),
                join: StepIdx::new(2),
            },
            None,
            None,
        ),
        n(
            1,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: None,
            },
            None,
            Some(2),
        ),
        n(
            2,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(0),
            },
            Some(2),
            Some(3),
        ),
        n(
            3,
            K::Finish {
                result: SlotIdx::new(2),
            },
            None,
            None,
        ),
    ])
}

fn zero_work_foreach_parts() -> WorkflowParts {
    let mut parts = parts(zero_work_foreach_nodes(), 3);
    parts.input_slots = Box::new([vb_core::workflow::CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: vb_core::workflow::InputSlotKind::List,
        constraint: vb_core::workflow::CompiledInputConstraint::required(),
    }]);
    parts
}

#[test]
fn together_group_constructs_with_cached_frame_and_group_peaks() {
    let workflow =
        CompiledWorkflow::try_from_concurrent_parts(parts(together_nodes(), 3)).expect("valid");

    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    let requirements = workflow.concurrent_requirements().expect("concurrent cache");
    assert_eq!(requirements.frames(), 3);
    assert_eq!(requirements.groups(), 1);
}

#[test]
fn zero_work_foreach_constructs_with_root_frame_and_group() {
    let workflow =
        CompiledWorkflow::try_from_concurrent_parts(zero_work_foreach_parts()).expect("valid");

    let requirements = workflow.concurrent_requirements().expect("concurrent cache");
    assert_eq!(requirements.frames(), 1);
    assert_eq!(requirements.groups(), 1);
}

fn nested_together_in_foreach_nodes() -> Box<[CompiledNode]> {
    Box::new([
        n(
            0,
            K::ForEachConcurrentStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: FanoutLimit::new(4),
                at_once: BranchCount::new(2),
                body: StepIdx::new(1),
                join: StepIdx::new(8),
            },
            None,
            None,
        ),
        n(
            1,
            K::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(2), StepIdx::new(4)]),
                join: StepIdx::new(6),
            },
            None,
            None,
        ),
        n(
            2,
            K::SetConst {
                value: vb_core::ConstIdx::new(0),
            },
            Some(2),
            Some(3),
        ),
        n(
            3,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(1),
                result: Some(SlotIdx::new(2)),
            },
            None,
            Some(6),
        ),
        n(
            4,
            K::SetConst {
                value: vb_core::ConstIdx::new(1),
            },
            Some(2),
            Some(5),
        ),
        n(
            5,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(1),
                result: Some(SlotIdx::new(2)),
            },
            None,
            Some(6),
        ),
        n(
            6,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(1),
            },
            Some(3),
            Some(7),
        ),
        n(
            7,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: Some(SlotIdx::new(3)),
            },
            None,
            Some(8),
        ),
        n(
            8,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(0),
            },
            Some(4),
            Some(9),
        ),
        n(
            9,
            K::Finish {
                result: SlotIdx::new(4),
            },
            None,
            None,
        ),
    ])
}

fn nested_together_in_foreach_parts() -> WorkflowParts {
    let mut parts = parts(nested_together_in_foreach_nodes(), 5);
    parts.input_slots = Box::new([vb_core::workflow::CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: vb_core::workflow::InputSlotKind::List,
        constraint: vb_core::workflow::CompiledInputConstraint::required(),
    }]);
    parts
}

fn legacy_foreach_in_lane_nodes() -> Box<[CompiledNode]> {
    Box::new([
        n(
            0,
            K::ForEachConcurrentStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: FanoutLimit::new(4),
                at_once: BranchCount::new(2),
                body: StepIdx::new(1),
                join: StepIdx::new(6),
            },
            None,
            None,
        ),
        n(
            1,
            K::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(2),
                limit: FanoutLimit::new(4),
                body: StepIdx::new(2),
                done: StepIdx::new(4),
            },
            Some(3),
            None,
        ),
        n(
            2,
            K::Copy {
                source: SlotIdx::new(2),
            },
            Some(5),
            Some(3),
        ),
        n(
            3,
            K::ForEachNext {
                iterator_slot: SlotIdx::new(3),
                body: StepIdx::new(2),
                done: StepIdx::new(4),
            },
            Some(2),
            None,
        ),
        n(
            4,
            K::ForEachJoin {
                output: SlotIdx::new(3),
            },
            Some(4),
            Some(5),
        ),
        n(
            5,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: Some(SlotIdx::new(4)),
            },
            None,
            Some(6),
        ),
        n(
            6,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(0),
            },
            Some(5),
            Some(7),
        ),
        n(
            7,
            K::Finish {
                result: SlotIdx::new(5),
            },
            None,
            None,
        ),
    ])
}

fn legacy_foreach_in_lane_parts() -> WorkflowParts {
    let mut parts = parts(legacy_foreach_in_lane_nodes(), 6);
    parts.input_slots = Box::new([vb_core::workflow::CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: vb_core::workflow::InputSlotKind::List,
        constraint: vb_core::workflow::CompiledInputConstraint::required(),
    }]);
    parts
}

#[test]
fn nested_concurrent_group_multiplies_frame_and_group_peaks() {
    let workflow = CompiledWorkflow::try_from_concurrent_parts(nested_together_in_foreach_parts())
        .expect("nested group valid");

    let requirements = workflow.concurrent_requirements().expect("concurrent cache");
    assert_eq!(requirements.frames(), 7);
    assert_eq!(requirements.groups(), 3);
}

#[test]
fn legacy_loop_inside_lane_keeps_one_lane_frame() {
    let workflow = CompiledWorkflow::try_from_concurrent_parts(legacy_foreach_in_lane_parts())
        .expect("legacy loop in lane valid");

    let requirements = workflow.concurrent_requirements().expect("concurrent cache");
    assert_eq!(requirements.frames(), 3);
    assert_eq!(requirements.groups(), 1);
}

#[test]
fn legacy_constructor_rejects_concurrent_graph_at_first_node() {
    let result = CompiledWorkflow::try_from_parts(parts(together_nodes(), 3));

    assert_eq!(
        result,
        Err(WorkflowError::ConcurrentProfileRequired {
            step: StepIdx::new(0)
        })
    );
}

#[test]
fn serial_workflow_stays_legacy_and_has_no_concurrent_cache() {
    let nodes = Box::new([
        n(
            0,
            K::SetConst {
                value: vb_core::ConstIdx::new(0),
            },
            Some(1),
            Some(1),
        ),
        n(
            1,
            K::Finish {
                result: SlotIdx::new(1),
            },
            None,
            None,
        ),
    ]);
    let workflow = CompiledWorkflow::try_from_parts(parts(nodes, 2)).expect("valid serial");

    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::LegacyV2);
    assert_eq!(workflow.concurrent_requirements(), None);
}

#[test]
fn lane_end_with_foreign_owner_is_rejected() {
    let mut nodes = together_nodes();
    nodes
        .get_mut(2)
        .expect("lane end")
        .kind = K::ConcurrentLaneEnd {
        owner: StepIdx::new(5),
        result: None,
    };

    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts(nodes, 3)),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(5)
        })
    );
}

#[test]
fn join_owner_mismatch_is_rejected() {
    let mut nodes = together_nodes();
    nodes
        .get_mut(5)
        .expect("join")
        .kind = K::ConcurrentGroupJoin {
        owner: StepIdx::new(1),
    };

    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts(nodes, 3)),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(5)
        })
    );
}

#[test]
fn branch_entry_at_or_after_join_is_rejected() {
    let mut nodes = together_nodes();
    nodes
        .get_mut(0)
        .expect("start")
        .kind = K::TogetherConcurrentStart {
        branches: Box::new([StepIdx::new(1), StepIdx::new(5)]),
        join: StepIdx::new(5),
    };

    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts(nodes, 3)),
        Err(WorkflowError::LoopRegionCrossing {
            control: StepIdx::new(0),
            from: StepIdx::new(0),
            to: StepIdx::new(5),
        })
    );
}

#[test]
fn zero_at_once_is_rejected() {
    let mut nodes = zero_work_foreach_nodes();
    nodes
        .get_mut(0)
        .expect("start")
        .kind = K::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: FanoutLimit::new(0),
        at_once: BranchCount::new(0),
        body: StepIdx::new(1),
        join: StepIdx::new(2),
    };

    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts(nodes, 3)),
        Err(WorkflowError::ResourceContractExceeded {
            resource: vb_core::workflow::ContractResource::BranchCount
        })
    );
}
