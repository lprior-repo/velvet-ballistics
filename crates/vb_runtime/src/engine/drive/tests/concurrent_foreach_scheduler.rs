use crate::engine::drive::transitions::concurrent_foreach_body_owner;
use vb_core::ids::{BranchCount, ConstIdx, FanoutLimit, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts, node,
};

fn unchecked_plan(join_owner: StepIdx) -> CompiledWorkflow {
    let nodes: Box<[CompiledNode]> = Box::new([
        node(
            StepIdx::ZERO,
            CompiledNodeKind::ForEachConcurrentStart {
                input: SlotIdx::ZERO,
                item_slot: SlotIdx::new(1),
                limit: FanoutLimit::new(3),
                at_once: BranchCount::new(2),
                body: StepIdx::new(1),
                join: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(2)),
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::ConcurrentGroupJoin { owner: join_owner },
            Some(SlotIdx::new(2)),
            None,
        ),
    ]);
    CompiledWorkflow::from_parts_unchecked(WorkflowParts {
        name: "scheduler_rearm_test".into(),
        digest: WorkflowDigest::from_bytes([1; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(1)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    })
}

#[test]
fn succeeded_pc_requires_exact_concurrent_foreach_body_owner() {
    let valid = unchecked_plan(StepIdx::ZERO);
    assert_eq!(
        concurrent_foreach_body_owner(&valid, StepIdx::new(1)),
        Some(StepIdx::ZERO)
    );
    assert_eq!(concurrent_foreach_body_owner(&valid, StepIdx::new(2)), None);

    let wrong_owner = unchecked_plan(StepIdx::new(1));
    assert_eq!(
        concurrent_foreach_body_owner(&wrong_owner, StepIdx::new(1)),
        None
    );
}
