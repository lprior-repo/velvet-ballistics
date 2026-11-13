use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts, node};

fn value_node() -> CompiledNode {
    node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    )
}

fn finish_node() -> CompiledNode {
    node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )
}

pub(super) fn parts(symbols_count: u32) -> WorkflowParts {
    WorkflowParts {
        name: "profile-fixture".into(),
        digest: WorkflowDigest::from_bytes([7; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::new([value_node(), finish_node()]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(7)]),
        slot_count: 1,
        symbols_count,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new(["set".into(), "finish".into()]),
        input_slots: Box::new([]),
    }
}

pub(super) fn malformed_concurrent(literal: &[u8]) -> Result<WorkflowParts, postcard::Error> {
    let kind = postcard::from_bytes::<CompiledNodeKind>(literal)?;
    let mut p = parts(0);
    p.nodes = Box::new([node(StepIdx::ZERO, kind, None, None)]);
    p.step_names = Box::new([]);
    p.entry = StepIdx::new(99);
    Ok(p)
}

pub(super) fn payloads(values: &[&str]) -> Box<[Box<str>]> {
    values
        .iter()
        .map(|value| Box::<str>::from(*value))
        .collect()
}

pub(super) fn source_parts()
-> Result<WorkflowParts, vb_core::workflow::source_metadata::SourceMetadataError> {
    let mut original = parts(3);
    original.source_metadata = Some(Box::new(
        vb_core::workflow::source_metadata::WorkflowSourceMetadata::try_new(
            payloads(&["z", "é", ""]),
            Box::new([]),
        )?,
    ));
    Ok(original)
}
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, InputDefaultValue, InputSlotKind,
};

fn secret_input() -> CompiledInputSlot {
    CompiledInputSlot {
        slot: SlotIdx::new(1),
        kind: InputSlotKind::Symbol,
        constraint: CompiledInputConstraint {
            secretness: true.into(),
            default_cold: Some(InputDefaultValue::Text("REQ_A".into())),
            ..CompiledInputConstraint::required()
        },
    }
}

pub(super) fn secret_parts()
-> Result<WorkflowParts, vb_core::workflow::source_metadata::SourceMetadataError> {
    use vb_core::ids::SymbolId;
    use vb_core::workflow::source_metadata::{CompiledSecretRequirement, WorkflowSourceMetadata};
    let mut original = parts(2);
    original.slot_count = 2;
    original.input_slots = Box::new([secret_input()]);
    let binding =
        CompiledSecretRequirement::with_slot(SymbolId::new(0), SymbolId::new(1), SlotIdx::new(1));
    original.source_metadata = Some(Box::new(WorkflowSourceMetadata::try_new(
        payloads(&["alias", "REQ_A"]),
        Box::new([binding]),
    )?));
    Ok(original)
}
