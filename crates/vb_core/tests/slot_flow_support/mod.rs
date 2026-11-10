use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind as K, InputSlotKind,
    ResourceContract, SlotBranch, WorkflowParts,
};

pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(crate) fn slot(raw: u16) -> SlotIdx {
    SlotIdx::new(raw)
}
pub(crate) fn step(raw: u16) -> StepIdx {
    StepIdx::new(raw)
}
pub(crate) fn node(id: u16, kind: K, out: Option<u16>, next: Option<u16>) -> CompiledNode {
    vb_core::workflow::node(step(id), kind, out.map(slot), next.map(step))
}
pub(crate) fn set(id: u16, value: u16, out: u16, next: u16) -> CompiledNode {
    node(
        id,
        K::SetConst {
            value: ConstIdx::new(value),
        },
        Some(out),
        Some(next),
    )
}
pub(crate) fn finish(id: u16, result: u16) -> CompiledNode {
    node(
        id,
        K::Finish {
            result: slot(result),
        },
        None,
        None,
    )
}
pub(crate) fn choose(id: u16, yes: u16, no: u16) -> CompiledNode {
    node(
        id,
        K::ChooseSlot {
            branches: vec![SlotBranch {
                condition: slot(0),
                target: step(yes),
            }]
            .into_boxed_slice(),
            otherwise: Some(step(no)),
        },
        None,
        None,
    )
}
pub(crate) fn parts(nodes: Vec<CompiledNode>, slots: u16) -> WorkflowParts {
    WorkflowParts {
        name: "slot-flow".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: slots,
        symbols_count: 0,
        entry: step(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    }
}
pub(crate) fn declare(p: &mut WorkflowParts, raw: u16, kind: InputSlotKind) {
    let mut inputs = p.input_slots.to_vec();
    inputs.push(CompiledInputSlot {
        slot: slot(raw),
        kind,
        constraint: CompiledInputConstraint::required(),
    });
    p.input_slots = inputs.into_boxed_slice();
}

pub(crate) fn rejected(p: WorkflowParts, reason: vb_core::workflow::SlotFlowFailure) -> TestResult {
    match vb_core::workflow::CompiledWorkflow::try_from_parts(p) {
        Err(vb_core::workflow::WorkflowError::SlotFlow(error)) => assert_eq!(error.reason, reason),
        other => return Err(format!("expected slot-flow failure, got {other:?}").into()),
    }
    Ok(())
}
