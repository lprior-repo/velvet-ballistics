use super::*;
use vb_core::{CompiledNodeKind, ResourceContract, StepIdx, WorkflowDigest, WorkflowParts};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn parts(resource_contract: ResourceContract) -> WorkflowParts {
    WorkflowParts {
        name: "prepared".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        slot_count: 3,
        symbols_count: 0,
        nodes: Box::from([vb_core::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]),
        expressions: Box::default(),
        accessors: Box::default(),
        constants: Box::default(),
        entry: StepIdx::new(0),
        resource_contract,
        step_names: Box::default(),
        input_slots: Box::default(),
    }
}

pub(super) fn workflow(prefix: Option<&[&str]>, bound: u16) -> TestResult<CompiledWorkflow> {
    let mut resource_contract = ResourceContract::DEFAULT;
    resource_contract.max_slots = bound;
    let mut parts = parts(resource_contract);
    parts.symbols_count = u32::try_from(prefix.map_or(0, |items| items.len()))?;
    let workflow = match prefix {
        None => CompiledWorkflow::try_from_parts(parts),
        Some(values) => CompiledWorkflow::try_from_parts_with_symbols(
            parts,
            values
                .iter()
                .map(|value| Box::<str>::from(*value))
                .collect(),
        ),
    }?;
    Ok(workflow)
}

pub(super) fn checkpoint(values: &[&str], bound: u64) -> ValueStoreCheckpointV1 {
    ValueStoreCheckpointV1 {
        symbols: values
            .iter()
            .map(|value| Box::<str>::from(*value))
            .collect(),
        lists: vec![],
        list_taints: vec![],
        objects: vec![],
        blobs: vec![],
        max_arena_entries: bound,
    }
}

pub(super) fn golden_input() -> ValueStoreCheckpointV1 {
    let mut input = checkpoint(&["a", "k", "a"], 0);
    input.lists = vec![Box::from([SlotValue::Symbol(vb_core::SymbolId::new(2))])];
    input.list_taints = vec![Box::from([Taint::Secret])];
    input.objects = vec![Box::from([vb_core::ObjectField::clean(
        vb_core::SymbolId::new(1),
        SlotValue::Symbol(vb_core::SymbolId::new(0)),
    )])];
    input
}

pub(super) fn golden_expected() -> ValueStoreCheckpointV1 {
    let mut expected = checkpoint(&["k", "p", "a"], 0);
    expected.lists = vec![Box::from([SlotValue::Symbol(vb_core::SymbolId::new(2))])];
    expected.list_taints = vec![Box::from([Taint::Secret])];
    expected.objects = vec![Box::from([vb_core::ObjectField::clean(
        vb_core::SymbolId::new(0),
        SlotValue::Symbol(vb_core::SymbolId::new(2)),
    )])];
    expected
}

pub(super) fn golden_roots() -> Vec<InputRoot> {
    vec![
        (
            SlotIdx::new(2),
            SlotValue::List(ListId::new(0)),
            Taint::Secret,
        ),
        (
            SlotIdx::new(1),
            SlotValue::Object(ObjectId::new(0)),
            Taint::DerivedFromSecret,
        ),
        (
            SlotIdx::new(0),
            SlotValue::Symbol(SymbolId::new(0)),
            Taint::Secret,
        ),
    ]
}

pub(super) fn expected_roots() -> Vec<InputRoot> {
    vec![
        (
            SlotIdx::new(2),
            SlotValue::List(ListId::new(0)),
            Taint::Secret,
        ),
        (
            SlotIdx::new(1),
            SlotValue::Object(ObjectId::new(0)),
            Taint::DerivedFromSecret,
        ),
        (
            SlotIdx::new(0),
            SlotValue::Symbol(SymbolId::new(2)),
            Taint::Secret,
        ),
    ]
}
