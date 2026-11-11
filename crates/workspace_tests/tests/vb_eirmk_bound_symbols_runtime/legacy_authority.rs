use super::{goldens, support};
use support::{Case, TestResult};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::workflow::{CompiledInputSlot, CompiledNodeKind, WorkflowIrV2, WorkflowParts};
use vb_core::{CompiledWorkflow, ConstIdx, ConstValue, InputSlotKind, RunId, SlotIdx};
use vb_core::{SlotValue, StepIdx, SymbolId, Taint, ValueStore, WorkflowDigest};

#[path = "raw_observation.rs"]
mod raw_observation;

pub(super) fn plan(prefix: &[&str], bound: u16) -> TestResult<CompiledWorkflow> {
    let mut parts = parts(bound)?;
    parts.symbols_count = u32::try_from(prefix.len())?;
    let mut ir = WorkflowIrV2 {
        wire_version: 2,
        parts,
        symbol_payloads: prefix.iter().map(|text| Box::<str>::from(*text)).collect(),
    };
    let mut bytes = vec![0x01, 0xff, b'V', b'B'];
    bytes.extend_from_slice(&postcard::to_allocvec(&ir)?);
    ir.parts.digest = WorkflowDigest::from_bytes(blake3::hash(&bytes).into());
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        ir.parts,
        ir.symbol_payloads,
    )?)
}

fn parts(bound: u16) -> TestResult<WorkflowParts> {
    let mut parts = support::compile(goldens::SOURCE)?.to_parts();
    parts.name = "public-bound-input-control".into();
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.semantic_source_identity = None;
    parts.nodes = nodes();
    parts.expressions = Box::default();
    parts.constants = Box::from([ConstValue::I64(7)]);
    parts.slot_count = 2;
    parts.symbols_count = 0;
    parts.resource_contract.max_slots = bound;
    parts.step_names = Box::from(["before".into(), "finish".into()]);
    parts.input_slots = Box::from([CompiledInputSlot {
        slot: SlotIdx::new(1),
        kind: InputSlotKind::Any,
        constraint: Default::default(),
    }]);
    Ok(parts)
}

fn nodes() -> Box<[vb_core::CompiledNode]> {
    Box::from([
        vb_core::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(1)),
        ),
        vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ])
}

pub(super) fn owned_case(prefix: &[&str], bound: u16) -> TestResult<Case> {
    let workflow = plan(prefix, bound)?;
    let mut store = ValueStore::with_max_slots(bound);
    assert_eq!(store.insert_symbol("noise")?, SymbolId::new(0));
    assert_eq!(store.insert_symbol("key")?, SymbolId::new(1));
    let object = store.insert_object(Box::from([vb_core::value_store::ObjectField::clean(
        SymbolId::new(1),
        SlotValue::Symbol(SymbolId::new(0)),
    )]))?;
    let roots = Box::from([(SlotIdx::new(1), SlotValue::Object(object), Taint::Clean)]);
    let bundle = OwnedRootBundle::new(roots, store.checkpoint_v1(), &workflow)?;
    Case::persist(
        workflow,
        "public runtime-only input control",
        bundle,
        RunId::new(18),
        &[],
    )
}

#[test]
fn bound_symbol_runtime_v1_legacy_inputs_and_recovery_unchanged() -> TestResult {
    super::cold::legacy_roundtrip(legacy_case(false)?, SlotValue::I64(7))?;
    super::cold::legacy_roundtrip(legacy_case(true)?, SlotValue::Symbol(SymbolId::new(1)))
}

fn legacy_case(symbolic: bool) -> TestResult<Case> {
    let mut parts = legacy_parts(symbolic)?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(
        vb_storage::encode_compiled_ir(&workflow)?.as_ref(),
        postcard::to_allocvec(&workflow.to_parts())?.as_slice()
    );
    let (checkpoint, value) = legacy_inputs(symbolic);
    let roots = Box::from([(SlotIdx::new(2), value, Taint::Clean)]);
    let bundle = OwnedRootBundle::new(roots, checkpoint, &workflow)?;
    Case::persist(
        workflow,
        "historical V1 Ask IR control",
        bundle,
        RunId::new(21),
        &[],
    )
}

fn legacy_parts(symbolic: bool) -> TestResult<WorkflowParts> {
    let mut parts = support::compile(goldens::SOURCE)?.to_parts();
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.semantic_source_identity = None;
    parts.expressions = Box::default();
    parts.symbols_count = if symbolic { 2 } else { 0 };
    parts.resource_contract.max_slots = 8;
    parts.constants = Box::from([if symbolic {
        ConstValue::Symbol(SymbolId::new(1))
    } else {
        ConstValue::I64(7)
    }]);
    parts
        .input_slots
        .first_mut()
        .ok_or("missing legacy input")?
        .kind = InputSlotKind::Any;
    parts.nodes = legacy_nodes(parts.nodes.first().ok_or("missing prompt step")?.clone());
    parts.step_names = Box::from([
        "before".into(),
        "ask".into(),
        "answer".into(),
        "finish".into(),
    ]);
    Ok(parts)
}

fn legacy_nodes(before: vb_core::CompiledNode) -> Box<[vb_core::CompiledNode]> {
    Box::from([
        before,
        ask_node(),
        vb_core::workflow::node(
            StepIdx::new(2),
            CompiledNodeKind::AskResume {
                answer: SlotIdx::new(1),
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(3)),
        ),
        vb_core::workflow::node(
            StepIdx::new(3),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
            None,
            None,
        ),
    ])
}

fn ask_node() -> vb_core::CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: vb_core::workflow::Deadline::Unset,
        },
        None,
        Some(StepIdx::new(2)),
    )
}

fn legacy_inputs(symbolic: bool) -> (vb_core::value_store::ValueStoreCheckpointV1, SlotValue) {
    let mut checkpoint = ValueStore::with_max_slots(8).checkpoint_v1();
    if symbolic {
        checkpoint.symbols = vec!["same".into(), "same".into()];
        checkpoint.lists = vec![Box::from([
            SlotValue::Symbol(SymbolId::new(0)),
            SlotValue::Symbol(SymbolId::new(1)),
        ])];
        checkpoint.list_taints = vec![Box::from([Taint::Clean, Taint::Clean])];
        (checkpoint, SlotValue::List(vb_core::ListId::new(0)))
    } else {
        (checkpoint, SlotValue::I64(7))
    }
}

#[test]
fn bound_symbol_runtime_raw_recovery_never_promotes_authority() -> TestResult {
    let boundary = super::cold::raw_boundary()?;
    super::failures::raw_scalar_authority(boundary)?;
    let case = goldens::fresh_case("key")?;
    let journal = case.open()?;
    let backend = vb_runtime::journal::StorageRuntimeJournal::shared_strict(journal.clone());
    let mut shard = vb_runtime::shard::Shard::new_with_journal(support::config(1), backend);
    let before = raw_observation::RawBefore::capture(&shard, &case, &journal)?;
    let command = raw_handle_command(&case, &journal, boundary)?;
    raw_wrong_identity(&mut shard, &case, command.clone())?;
    before.assert_original(&shard, &case, &journal)?;
    raw_observation::assert_driver_reason(&case, &command)?;
    enqueue_recovery(&shard, command)?;
    raw_observation::observe(&mut shard, &case, &journal, &before)
}

fn raw_wrong_identity(
    shard: &mut vb_runtime::shard::Shard,
    case: &Case,
    mut command: vb_runtime::shard::types::RecoverRunCommand,
) -> TestResult {
    command.workflow_digest = WorkflowDigest::from_bytes([99; 32]);
    let expected = vb_runtime::RuntimeError::Recovery {
        error: format!(
            "workflow digest mismatch: expected {:?}, got {:?}",
            command.workflow_digest,
            case.workflow.digest()
        ),
    };
    enqueue_recovery(shard, command)?;
    assert_eq!(shard.tick(), Err(expected));
    assert_eq!(shard.run_state_get(case.run), None);
    Ok(())
}

fn raw_handle_command(
    case: &Case,
    journal: &vb_storage::FjallJournal,
    boundary: vb_runtime::recovery::RecoveredRunBoundary,
) -> TestResult<vb_runtime::shard::types::RecoverRunCommand> {
    let frame = raw_frame(case.run)?;
    Ok(vb_runtime::shard::types::RecoverRunCommand::new(
        case.run,
        frame,
        case.artifact.digest,
        case.workflow.digest(),
        next_sequence(journal, case.run)?,
        vb_runtime::primitives::collect::CollectStates::new(),
        boundary,
    ))
}

fn raw_frame(run: RunId) -> TestResult<vb_core::RunFrame> {
    let mut frame = vb_core::RunFrame::new(run, StepIdx::new(1), 3, 3)?;
    frame.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;
    frame.mark_running(StepIdx::new(0))?;
    frame.mark_succeeded(StepIdx::new(0))?;
    frame.write_slot(
        SlotIdx::new(2),
        SlotValue::Object(vb_core::ObjectId::new(0)),
    )?;
    Ok(frame)
}

fn next_sequence(
    journal: &vb_storage::FjallJournal,
    run: RunId,
) -> TestResult<vb_storage::EventSeq> {
    let events = journal.events_for_run_full(run)?;
    let next = events
        .last()
        .ok_or("missing prefix")?
        .seq()
        .get()
        .checked_add(1)
        .ok_or("journal sequence overflow")?;
    Ok(vb_storage::EventSeq::new(next))
}

pub(super) fn enqueue_recovery(
    shard: &vb_runtime::shard::Shard,
    command: vb_runtime::shard::types::RecoverRunCommand,
) -> TestResult {
    assert_eq!(
        shard.enqueue(vb_runtime::shard::ShardCommand::Recover {
            command: Box::new(command)
        }),
        Ok(())
    );
    Ok(())
}
