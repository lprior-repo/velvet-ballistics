use super::{WorkflowError, group};
use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind, CompiledWorkflow,
    InputSlotKind, NodeOutput, ResourceContract, WorkflowIrProfile, WorkflowParts,
};

fn constructor_nodes() -> Box<[CompiledNode]> {
    let mut nodes = group(None, 1).into_boxed_slice();
    for node in nodes.iter_mut() {
        if matches!(node.kind, CompiledNodeKind::ConcurrentGroupJoin { .. }) {
            node.output = NodeOutput::from(Some(SlotIdx::new(2)));
        }
    }
    nodes
}

fn input_slots() -> Box<[CompiledInputSlot]> {
    Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::I64,
        constraint: CompiledInputConstraint::required(),
    }])
}

fn parts() -> WorkflowParts {
    WorkflowParts {
        name: "budget-control".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: constructor_nodes(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 3,
        symbols_count: 1,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: input_slots(),
    }
}

// Historical case identity retained; C1/C4 require checked acceptance.
#[test]
fn constructor_stays_closed_pending_ir_validation() -> Result<(), WorkflowError> {
    let original = parts();
    let workflow = CompiledWorkflow::try_from_concurrent_parts(original.clone())?;
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(
        workflow
            .concurrent_requirements()
            .map(|r| (r.frames(), r.groups())),
        Some((2, 1))
    );
    assert_eq!(workflow.to_parts(), original);
    Ok(())
}

#[test]
fn checked_budget_constructor_rejects_missing_join_output() -> Result<(), WorkflowError> {
    let mut malformed = parts();
    malformed
        .nodes
        .get_mut(3)
        .ok_or(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(3),
        })?
        .output = NodeOutput::Discard;
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(malformed).map(|_| ()),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(3)
        })
    );
    Ok(())
}

#[test]
fn budget_fixture_remains_closed_under_legacy_constructor() {
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts()).map(|_| ()),
        Err(WorkflowError::ConcurrentProfileRequired {
            step: StepIdx::new(0)
        })
    );
}
