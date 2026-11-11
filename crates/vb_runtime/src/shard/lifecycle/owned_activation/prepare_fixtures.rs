use super::*;
use vb_core::{
    CompiledInputSlot, CompiledNodeKind, InputSlotKind, StepIdx, WorkflowDigest, WorkflowParts,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn parts(bound: u16, slots: u16) -> WorkflowParts {
    let mut resource_contract = vb_core::ResourceContract::DEFAULT;
    resource_contract.max_slots = bound;
    WorkflowParts {
        name: "owned prepared".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        slot_count: slots,
        symbols_count: 0,
        nodes: Box::from([vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
            None,
            None,
        )]),
        expressions: Box::default(),
        accessors: Box::default(),
        constants: Box::default(),
        entry: StepIdx::ZERO,
        resource_contract,
        step_names: Box::default(),
        input_slots: input_slots(slots),
    }
}

fn input_slots(slots: u16) -> Box<[CompiledInputSlot]> {
    (0..slots)
        .map(|slot| CompiledInputSlot {
            slot: SlotIdx::new(slot),
            kind: InputSlotKind::Any,
            constraint: Default::default(),
        })
        .collect()
}

pub(super) fn workflow(
    prefix: Option<&[&str]>,
    bound: u16,
    slots: u16,
) -> TestResult<CompiledWorkflow> {
    let mut parts = parts(bound, slots);
    parts.symbols_count = u32::try_from(prefix.map_or(0, |prefix| prefix.len()))?;
    Ok(match prefix {
        Some(prefix) => CompiledWorkflow::try_from_parts_with_symbols(
            parts,
            prefix.iter().map(|text| Box::<str>::from(*text)).collect(),
        ),
        None => CompiledWorkflow::try_from_parts(parts),
    }?)
}

pub(super) fn original_store() -> ValueStoreCheckpointV1 {
    let mut checkpoint = ValueStore::with_max_slots(8).checkpoint_v1();
    checkpoint.symbols = vec!["payload".into(), "key".into(), "payload".into()];
    checkpoint.lists = vec![Box::from([
        SlotValue::Symbol(SymbolId::new(2)),
        SlotValue::Blob(vb_core::BlobId::new(0)),
    ])];
    checkpoint.list_taints = vec![Box::from([Taint::Secret, Taint::Clean])];
    checkpoint.objects = vec![Box::from([vb_core::ObjectField {
        key: SymbolId::new(1),
        value: SlotValue::Symbol(SymbolId::new(0)),
        taint: Taint::DerivedFromSecret,
    }])];
    checkpoint.blobs = vec![Vec::from(*b"preserved").into()];
    checkpoint
}

pub(super) fn expected_store() -> ValueStoreCheckpointV1 {
    let mut checkpoint = ValueStore::with_max_slots(8).checkpoint_v1();
    checkpoint.symbols = vec!["key".into(), "compiler".into(), "payload".into()];
    checkpoint.lists = vec![Box::from([
        SlotValue::Symbol(SymbolId::new(2)),
        SlotValue::Blob(vb_core::BlobId::new(0)),
    ])];
    checkpoint.list_taints = vec![Box::from([Taint::Secret, Taint::Clean])];
    checkpoint.objects = vec![Box::from([vb_core::ObjectField {
        key: SymbolId::new(0),
        value: SlotValue::Symbol(SymbolId::new(2)),
        taint: Taint::DerivedFromSecret,
    }])];
    checkpoint.blobs = vec![Vec::from(*b"preserved").into()];
    checkpoint
}

pub(super) fn original_roots() -> Box<[(SlotIdx, SlotValue, Taint)]> {
    Box::from([
        (
            SlotIdx::new(0),
            SlotValue::Symbol(SymbolId::new(0)),
            Taint::DerivedFromSecret,
        ),
        (
            SlotIdx::new(1),
            SlotValue::List(ListId::new(0)),
            Taint::Secret,
        ),
        (
            SlotIdx::new(2),
            SlotValue::Object(ObjectId::new(0)),
            Taint::Secret,
        ),
    ])
}

pub(super) fn expected_frame() -> TestResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(501), StepIdx::ZERO, 1, 3)?;
    frame.write_slot_with_taint(
        SlotIdx::ZERO,
        SlotValue::Symbol(SymbolId::new(2)),
        Taint::DerivedFromSecret,
    )?;
    frame.write_slot_with_taint(
        SlotIdx::new(1),
        SlotValue::List(ListId::new(0)),
        Taint::Secret,
    )?;
    frame.write_slot_with_taint(
        SlotIdx::new(2),
        SlotValue::Object(ObjectId::new(0)),
        Taint::Secret,
    )?;
    Ok(frame)
}

pub(super) fn shard() -> Shard {
    let mut config = crate::shard::ShardConfig::default();
    config.max_active_runs = 2;
    Shard::new(config)
}

pub(super) fn admission(digest: WorkflowDigest) -> crate::admission::RunAdmission {
    crate::admission::RunAdmission::new(
        digest,
        RunId::new(501),
        vb_core::CapabilitySet::from_grants(Box::default()),
        vb_core::RuntimePolicy::Strict,
    )
}

pub(super) fn accepted(
    workflow: CompiledWorkflow,
    bundle: OwnedRootBundle,
) -> PreparedOwnedActivation {
    PreparedOwnedActivation {
        admission: admission(workflow.digest()),
        workflow,
        bundle,
        contracts: Box::default(),
        next_seq: vb_storage::EventSeq::new(3),
    }
}

pub(super) fn capacity_fixture() -> TestResult<(PreparedOwnedActivation, ValueStoreCheckpointV1)> {
    let workflow = workflow(Some(&["key", "compiler"]), 3, 2)?;
    let mut checkpoint = ValueStore::with_max_slots(3).checkpoint_v1();
    checkpoint.symbols = vec!["payload".into(), "owned".into()];
    let roots = Box::from([
        (
            SlotIdx::ZERO,
            SlotValue::Symbol(SymbolId::new(0)),
            Taint::Secret,
        ),
        (
            SlotIdx::new(1),
            SlotValue::Symbol(SymbolId::new(1)),
            Taint::Secret,
        ),
    ]);
    let bundle = OwnedRootBundle::new(roots, checkpoint.clone(), &workflow)?;
    Ok((accepted(workflow, bundle), checkpoint))
}
