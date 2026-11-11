use super::super::*;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::workflow::{CompiledInputConstraint, CompiledInputSlot, InputSlotKind, WorkflowIrV2};
use vb_core::{
    CompiledNodeKind, ObjectField, ObjectId, StepIdx, SymbolId, WorkflowId, WorkflowParts,
};
use vb_storage::{AcceptedArtifact, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const RUN: RunId = RunId::new(42_522);

pub(super) struct DurableCase {
    pub(super) directory: tempfile::TempDir,
    pub(super) workflow: CompiledWorkflow,
    pub(super) artifact: AcceptedArtifact,
    pub(super) record: OwnedRunInputsRecordV1,
}

fn parts() -> WorkflowParts {
    WorkflowParts {
        name: "cold-inputs".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        nodes: Box::from([finish()]),
        expressions: Box::default(),
        accessors: Box::default(),
        constants: Box::default(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::from([Box::from("finish")]),
        input_slots: Box::from([input_slot()]),
    }
}

fn finish() -> vb_core::CompiledNode {
    vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    )
}

fn input_slot() -> CompiledInputSlot {
    CompiledInputSlot {
        slot: SlotIdx::ZERO,
        kind: InputSlotKind::Object,
        constraint: CompiledInputConstraint::unconstrained(),
    }
}

pub(super) fn workflow(prefix: Option<&[&str]>, cap: u64) -> TestResult<CompiledWorkflow> {
    let mut parts = parts();
    parts.resource_contract.max_slots = u16::try_from(cap)?;
    let Some(prefix) = prefix else {
        let bytes = postcard::to_allocvec(&parts)?;
        parts.digest = WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes());
        return Ok(CompiledWorkflow::try_from_parts(parts)?);
    };
    parts.symbols_count = u32::try_from(prefix.len())?;
    let mut ir = WorkflowIrV2 {
        wire_version: 2,
        parts,
        symbol_payloads: prefix.iter().map(|s| Box::<str>::from(*s)).collect(),
    };
    let mut bytes = vec![0x01, 0xff, 0x56, 0x42];
    bytes.extend_from_slice(&postcard::to_allocvec(&ir)?);
    ir.parts.digest = WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes());
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        ir.parts,
        ir.symbol_payloads,
    )?)
}

pub(super) fn checkpoint(key: &str, cap: u64) -> ValueStoreCheckpointV1 {
    ValueStoreCheckpointV1 {
        symbols: vec![Box::from("noise"), Box::from(key)],
        lists: vec![],
        list_taints: vec![],
        objects: vec![Box::from([ObjectField::clean(
            SymbolId::new(1),
            SlotValue::Symbol(SymbolId::new(0)),
        )])],
        blobs: vec![],
        max_arena_entries: cap,
    }
}

pub(super) fn roots() -> Box<[(SlotIdx, SlotValue, Taint)]> {
    Box::from([(
        SlotIdx::ZERO,
        SlotValue::Object(ObjectId::new(0)),
        Taint::Secret,
    )])
}

pub(super) fn source() -> vb_storage::WorkflowSourceRecord {
    let bytes = b"cold-owned-input-fixture";
    vb_storage::WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(*blake3::hash(bytes).as_bytes()),
        source: bytes.to_vec(),
    }
}

fn header(workflow: &CompiledWorkflow) -> RunHeaderRecord {
    RunHeaderRecord {
        run: RUN,
        workflow_id: WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    }
}

pub(super) fn durable_case(
    prefix: Option<&[&str]>,
    key: &str,
    cap: u64,
) -> TestResult<DurableCase> {
    durable_checkpoint_case(prefix, checkpoint(key, cap))
}

pub(super) fn durable_checkpoint_case(
    prefix: Option<&[&str]>,
    input: ValueStoreCheckpointV1,
) -> TestResult<DurableCase> {
    let workflow = workflow(prefix, input.max_arena_entries)?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )?;
    let record = owned_record(&workflow, &artifact, &input)?;
    let directory = tempfile::tempdir()?;
    persist(directory.path(), &workflow, &artifact, &input)?;
    Ok(DurableCase {
        directory,
        workflow,
        artifact,
        record,
    })
}

fn owned_record(
    workflow: &CompiledWorkflow,
    artifact: &AcceptedArtifact,
    input: &ValueStoreCheckpointV1,
) -> TestResult<OwnedRunInputsRecordV1> {
    let bundle = OwnedRootBundle::new(roots(), input.clone(), workflow)?;
    Ok(OwnedRunInputsRecordV1::new(
        RUN,
        artifact.digest,
        source().digest,
        &bundle,
        workflow,
    )?)
}

fn persist(
    path: &std::path::Path,
    workflow: &CompiledWorkflow,
    artifact: &AcceptedArtifact,
    input: &ValueStoreCheckpointV1,
) -> TestResult {
    let record = owned_record(workflow, artifact, input)?;
    let submission = RunSubmission::owned(source(), artifact.clone(), record, header(workflow))?;
    let journal = FjallJournal::open(path, None)?;
    journal.commit_run_submission(&submission, vb_core::RuntimePolicy::Strict)?;
    Ok(())
}

pub(super) fn load(
    case: &DurableCase,
    journal: &FjallJournal,
) -> RuntimeResult<RecoveredRetainedInputs> {
    let events = journal
        .events_for_run_full(RUN)
        .map_err(|error| RuntimeError::Recovery {
            error: error.to_string(),
        })?;
    load_retained_inputs_for_recovery(&events, journal, &case.artifact, &case.workflow, RUN)
}

pub(super) fn assert_durable_unchanged(
    case: &DurableCase,
    journal: &FjallJournal,
    events: &[JournalEvent],
) -> TestResult {
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    let record = journal
        .owned_run_inputs(RUN, case.artifact.digest, source().digest, &case.workflow)?
        .ok_or("retained owned record missing after recovery")?;
    assert_eq!(
        postcard::to_allocvec(&record)?,
        postcard::to_allocvec(&case.record)?
    );
    Ok(())
}

pub(super) fn expected(key: &str, key_id: SymbolId, cap: u64) -> ValueStoreCheckpointV1 {
    let mut result = checkpoint(key, cap);
    result.symbols = vec![Box::from("key"), Box::from("noise")];
    if key == "absent" {
        result.symbols.push(Box::from("absent"));
    }
    result.objects = vec![Box::from([ObjectField::clean(
        key_id,
        SlotValue::Symbol(SymbolId::new(1)),
    )])];
    result
}

pub(super) fn rich_input() -> ValueStoreCheckpointV1 {
    let mut input = checkpoint("key", 8);
    input.lists = vec![Box::from([
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::Blob(vb_core::BlobId::new(0)),
    ])];
    input.list_taints = vec![Box::from([Taint::Secret, Taint::Clean])];
    input.blobs = vec![Vec::from([0, 7, 255]).into()];
    input.objects = vec![Box::from([ObjectField::clean(
        SymbolId::new(1),
        SlotValue::List(vb_core::ListId::new(0)),
    )])];
    input
}

pub(super) fn rich_expected() -> ValueStoreCheckpointV1 {
    let mut expected = rich_input();
    expected.symbols = vec![Box::from("key"), Box::from("noise")];
    expected.lists = vec![Box::from([
        SlotValue::Symbol(SymbolId::new(1)),
        SlotValue::Blob(vb_core::BlobId::new(0)),
    ])];
    expected.objects = vec![Box::from([ObjectField::clean(
        SymbolId::new(0),
        SlotValue::List(vb_core::ListId::new(0)),
    )])];
    expected
}

pub(super) fn persist_runtime_write(path: &std::path::Path) -> TestResult {
    let value = postcard::to_allocvec(&SlotValue::Symbol(SymbolId::new(1)))?;
    let extra = vb_storage::encode_slot_written_extra(Taint::DerivedFromSecret, None)
        .map_err(|error| format!("{error:?}"))?;
    let event = JournalEvent::SlotWrittenEvent {
        run: RUN,
        seq: vb_storage::EventSeq::new(3),
        slot: SlotIdx::ZERO,
        value: Some(value),
        extra: Some(extra),
        attempt: 1,
    };
    let journal = FjallJournal::open(path, None)?;
    journal.append_strict_batch(&[event])?;
    Ok(())
}
