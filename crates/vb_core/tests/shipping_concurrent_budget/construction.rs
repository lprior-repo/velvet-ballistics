use super::{K, WorkflowError, group, n};
use vb_core::ConstValue;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledWorkflow, InputSlotKind,
    NodeOutput, ResourceContract, WorkflowIrProfile, WorkflowParts, node,
};

fn n_with_output(id: u16, kind: K, out: u16, next: Option<u16>) -> CompiledNode {
    node(
        StepIdx::new(id),
        kind,
        Some(SlotIdx::new(out)),
        next.map(StepIdx::new),
    )
}

fn with_join_output(mut node: CompiledNode) -> CompiledNode {
    if matches!(node.kind, K::ConcurrentGroupJoin { .. }) {
        node.output = NodeOutput::from(Some(SlotIdx::new(2)));
    }
    node
}

fn constructor_nodes() -> Box<[CompiledNode]> {
    // Use non-suspending SetConst for checked constructor admission while
    // preserving the shared budget counting fixture's action-ticket semantics.
    let nodes = vec![
        n(
            0,
            K::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(1)]),
                join: StepIdx::new(3),
            },
            None,
        ),
        n_with_output(
            1,
            K::SetConst {
                value: ConstIdx::new(0),
            },
            1,
            Some(2),
        ),
        n(
            2,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: None,
            },
            Some(3),
        ),
        n(
            3,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(0),
            },
            Some(4),
        ),
        n(
            4,
            K::Finish {
                result: SlotIdx::new(0),
            },
            None,
        ),
    ];
    nodes.into_iter().map(with_join_output).collect()
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
        constants: Box::new([ConstValue::I64(42)]),
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

#[test]
fn checked_constructor_rejects_original_budget_fixture() {
    // Original shared budget fixture uses action (Do) inside lane, which
    // must be rejected by the checked ConcurrentV3 constructor.
    let original = group(None, 1)
        .into_iter()
        .map(with_join_output)
        .collect::<Vec<_>>();
    let original_parts = WorkflowParts {
        name: "budget-original".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: original.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 3,
        symbols_count: 1,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: input_slots(),
    };
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(original_parts).map(|_| ()),
        Err(WorkflowError::ConcurrentLaneSuspension {
            owner: StepIdx::new(0),
            step: StepIdx::new(1),
        })
    );
}
