use std::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{
    ActionContract, CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, SlotValue, Taint,
    WorkflowDigest,
};
use vb_runtime::RuntimeError;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    AcceptedArtifact, FjallJournal, JournalEvent, OwnedRunInputsRecordV1, RunHeaderRecord,
    RunSubmission, WorkflowSourceRecord,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) type JournalSnapshot = (Vec<JournalEvent>, Vec<u8>);

pub(super) struct Case {
    pub(super) directory: tempfile::TempDir,
    pub(super) workflow: CompiledWorkflow,
    pub(super) source: WorkflowSourceRecord,
    pub(super) artifact: AcceptedArtifact,
    pub(super) original: OwnedRootBundle,
    pub(super) input_digest: WorkflowDigest,
    pub(super) run: RunId,
}

impl Case {
    pub(super) fn persist(
        workflow: CompiledWorkflow,
        source: &str,
        original: OwnedRootBundle,
        run: RunId,
        contracts: &[ActionContract],
    ) -> TestResult<Self> {
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            contracts,
        )?;
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
            source: source.as_bytes().to_vec(),
        };
        make_case(workflow, source, artifact, original, run)
    }

    pub(super) fn open(&self) -> TestResult<Arc<FjallJournal>> {
        Ok(Arc::new(FjallJournal::open(self.directory.path(), None)?))
    }

    pub(super) fn retained_bytes(&self, journal: &FjallJournal) -> TestResult<Vec<u8>> {
        let record = journal
            .owned_run_inputs(
                self.run,
                self.artifact.digest,
                self.source.digest,
                &self.workflow,
            )?
            .ok_or("missing original owned inputs")?;
        assert_eq!(record.checkpoint(), self.original.checkpoint());
        assert_eq!(record.roots(), self.original.roots());
        assert_eq!(record.input_digest(), self.input_digest);
        Ok(postcard::to_allocvec(&record)?)
    }
}

fn make_case(
    workflow: CompiledWorkflow,
    source: WorkflowSourceRecord,
    artifact: AcceptedArtifact,
    original: OwnedRootBundle,
    run: RunId,
) -> TestResult<Case> {
    let directory = tempfile::tempdir()?;
    let record =
        OwnedRunInputsRecordV1::new(run, artifact.digest, source.digest, &original, &workflow)?;
    let input_digest = record.input_digest();
    let case = Case {
        directory,
        workflow,
        source,
        artifact,
        original,
        input_digest,
        run,
    };
    commit_owned(&case, record)?;
    Ok(case)
}

fn commit_owned(case: &Case, record: OwnedRunInputsRecordV1) -> TestResult<()> {
    let submission = RunSubmission::owned(
        case.source.clone(),
        case.artifact.clone(),
        record,
        header(case.run, &case.workflow),
    )?;
    let journal = FjallJournal::open(case.directory.path(), None)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    Ok(())
}

fn header(run: RunId, workflow: &CompiledWorkflow) -> RunHeaderRecord {
    RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    }
}

pub(super) fn compile(source: &str) -> TestResult<CompiledWorkflow> {
    vb_compile::compile_workflow(source.as_bytes()).map_err(|error| format!("{error:?}").into())
}

pub(super) fn config(step_budget: u64) -> ShardConfig {
    ShardConfig {
        step_budget_per_tick: step_budget,
        ..ShardConfig::default()
    }
}

pub(super) fn runtime(journal: Arc<FjallJournal>, config: ShardConfig) -> TestResult<Runtime> {
    Ok(Runtime::new(
        NonZeroUsize::new(1).ok_or("zero shard count")?,
        config,
        StorageRuntimeJournal::shared_strict(journal),
    ))
}

pub(super) fn journal_snapshot(journal: &FjallJournal, run: RunId) -> TestResult<JournalSnapshot> {
    let events = journal.events_for_run_full(run)?;
    let bytes = postcard::to_allocvec(&events)?;
    Ok((events, bytes))
}

pub(super) fn finished_has(
    case: &Case,
    actor: &mut Runtime,
    journal: &FjallJournal,
    expected: bool,
) -> TestResult {
    assert_eq!(
        actor.inspect_run_state(case.run).map(|_| ()),
        Err(RuntimeError::RunNotFound)
    );
    assert_terminal(
        journal,
        case.run,
        SlotIdx::new(1),
        SlotValue::Bool(expected),
    )?;
    let before = journal_snapshot(journal, case.run)?;
    assert_eq!(actor.tick_all(), Ok(true));
    assert_eq!(actor.tick_all(), Ok(true));
    assert_eq!(journal_snapshot(journal, case.run)?, before);
    Ok(())
}

pub(super) fn assert_terminal(
    journal: &FjallJournal,
    run: RunId,
    output: SlotIdx,
    expected: SlotValue,
) -> TestResult {
    let events = journal.events_for_run_full(run)?;
    let terminals: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::RunFinished {
                run,
                result,
                attempt,
                ..
            } => Some((*run, *result, *attempt)),
            _ => None,
        })
        .collect();
    assert_eq!(terminals, [(run, output, 1)]);
    assert_write(&events, output, expected, Taint::Clean)
}

pub(super) fn assert_write(
    events: &[JournalEvent],
    slot: SlotIdx,
    expected: SlotValue,
    taint: Taint,
) -> TestResult {
    let writes: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                slot: actual,
                value,
                extra,
                ..
            } if *actual == slot => Some((value, extra)),
            _ => None,
        })
        .collect();
    assert_eq!(writes.len(), 1);
    let (value, extra) = writes.first().ok_or("missing slot write")?;
    assert_eq!(
        postcard::from_bytes::<SlotValue>(value.as_ref().ok_or("missing value")?)?,
        expected
    );
    assert_extra(extra.as_ref().ok_or("missing taint")?, taint)
}

fn assert_extra(extra: &[u8], taint: Taint) -> TestResult {
    let decoded =
        vb_storage::decode_slot_written_extra(extra).map_err(|error| format!("{error:?}"))?;
    let vb_storage::DecodedSlotWrittenExtra::Envelope(envelope) = decoded else {
        return Err("unexpected legacy slot-write extra".into());
    };
    assert_eq!(envelope.taint, taint);
    assert_eq!(envelope.frame_extra, None);
    Ok(())
}

pub(super) fn nested_store(prepared: bool) -> vb_core::value_store::ValueStoreCheckpointV1 {
    let symbols = if prepared {
        ["k", "p", "a", "z"]
    } else {
        ["a", "k", "a", "z"]
    };
    vb_core::value_store::ValueStoreCheckpointV1 {
        symbols: symbols.into_iter().map(Box::<str>::from).collect(),
        lists: vec![Box::from([
            SlotValue::Object(vb_core::ObjectId::new(0)),
            SlotValue::Symbol(vb_core::SymbolId::new(2)),
            SlotValue::Symbol(vb_core::SymbolId::new(3)),
            SlotValue::Blob(vb_core::BlobId::new(0)),
        ])],
        list_taints: vec![Box::from([
            Taint::Secret,
            Taint::DerivedFromSecret,
            Taint::Clean,
            Taint::Secret,
        ])],
        objects: vec![nested_fields(prepared)],
        blobs: vec![bytes::Bytes::from_static(b"\x00\xffblob")],
        max_arena_entries: 7,
    }
}

fn nested_fields(prepared: bool) -> Box<[vb_core::value_store::ObjectField]> {
    use vb_core::SymbolId;
    use vb_core::value_store::ObjectField;
    let (key, first, second) = if prepared { (2, 0, 2) } else { (0, 1, 0) };
    Box::from([
        ObjectField::with_taint(
            SymbolId::new(key),
            SlotValue::Symbol(SymbolId::new(first)),
            Taint::Secret,
        ),
        ObjectField::with_taint(
            SymbolId::new(2),
            SlotValue::Symbol(SymbolId::new(second)),
            Taint::DerivedFromSecret,
        ),
        ObjectField::with_taint(
            SymbolId::new(3),
            SlotValue::Blob(vb_core::BlobId::new(0)),
            Taint::Secret,
        ),
    ])
}
