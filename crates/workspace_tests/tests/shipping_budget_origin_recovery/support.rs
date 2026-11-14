use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use vb_core::root_bundle::OwnedRootBundle;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};
use vb_core::{ConstIdx, ConstValue, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::constants::{
    KEYSPACE_RUN_EVENT, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
};
use vb_storage::{
    EventSeq, FjallJournal, JournalEvent, RecordKind, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const ORIGIN_SEQ: u64 = 3;

#[derive(Clone, Copy)]
pub(super) enum OriginMutation {
    Foreign,
    OutOfOrder,
}

pub(super) struct Case {
    pub(super) directory: tempfile::TempDir,
    pub(super) run: RunId,
    pub(super) origin_seq: u64,
    pub(super) started_millis: u64,
}

impl OriginMutation {
    pub(super) fn event(
        self,
        run: RunId,
        seq: u64,
        started_millis: u64,
    ) -> TestResult<JournalEvent> {
        let event_run = match self {
            Self::Foreign => RunId::new(run.get().checked_add(1).ok_or("run id overflow")?),
            Self::OutOfOrder => run,
        };
        let event_seq = match self {
            Self::Foreign => seq,
            Self::OutOfOrder => seq.checked_add(1).ok_or("event sequence overflow")?,
        };
        Ok(JournalEvent::RunBudgetOriginEvent {
            run: event_run,
            seq: EventSeq::new(event_seq),
            started_millis,
        })
    }

    pub(super) fn expected_error(self, run: RunId, seq: u64) -> String {
        match self {
            Self::Foreign => vb_storage::JournalError::WrongRun {
                expected: run,
                actual: RunId::new(run.get().saturating_add(1)),
            }
            .to_string(),
            Self::OutOfOrder => vb_storage::JournalError::ReplayKeyMismatch {
                run,
                key_seq: seq,
                payload_seq: seq.saturating_add(1),
            }
            .to_string(),
        }
    }
}

pub(super) fn fixture(run: RunId) -> TestResult<Case> {
    let now = wall_clock_millis()?;
    let deadline = now.checked_add(60_000).ok_or("deadline overflow")?;
    let workflow = waiting_workflow(deadline)?;
    let source_bytes = b"origin recovery source".to_vec();
    let source_digest = WorkflowDigest::from_bytes(*blake3::hash(&source_bytes).as_bytes());
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let checkpoint =
        vb_core::value_store::ValueStore::with_max_slots(workflow.resource_contract().max_slots)
            .checkpoint_v1();
    let bundle = OwnedRootBundle::new(Box::default(), checkpoint, &workflow)?;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )?;
    let submission = RunSubmission::owned(
        WorkflowSourceRecord {
            digest: source_digest,
            source: source_bytes,
        },
        artifact,
        inputs,
        RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: now,
        },
    )?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    let runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    );
    runtime.activate_owned_submission(run, source_digest)?;
    let events = journal.events_for_run_full(run)?;
    assert!(
        matches!(events.get(ORIGIN_SEQ as usize), Some(JournalEvent::RunBudgetOriginEvent { run: found, seq, .. }) if *found == run && seq.get() == ORIGIN_SEQ)
    );
    drop(runtime);
    drop(journal);
    Ok(Case {
        directory,
        run,
        origin_seq: ORIGIN_SEQ,
        started_millis: now,
    })
}

pub(super) fn rewrite_origin(case: &Case, mutation: OriginMutation) -> TestResult<Vec<u8>> {
    let event = mutation.event(case.run, case.origin_seq, case.started_millis)?;
    let envelope_seq = event.seq().get();
    let bytes = vb_storage::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunBudgetOrigin,
        envelope_seq,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(vb_storage::journal::parse_event(&bytes)?, event);

    let database = fjall::Database::builder(case.directory.path())
        .cache_size(1 << 20)
        .open()
        .map_err(|error| format!("open database for origin rewrite: {error:?}"))?;
    let space = database.keyspace(KEYSPACE_RUN_EVENT, fjall::KeyspaceCreateOptions::default)?;
    let key = vb_storage::keys::run_event_key(case.run, EventSeq::new(case.origin_seq))?;
    space.insert(key, bytes.as_slice())?;
    database.persist(fjall::PersistMode::SyncAll)?;
    drop(space);
    drop(database);
    Ok(bytes)
}

pub(super) fn cold_recover(case: &Case) -> TestResult<(Runtime, Arc<FjallJournal>)> {
    let journal = Arc::new(
        FjallJournal::open(case.directory.path(), None)
            .map_err(|error| format!("cold reopen origin journal: {error:?}"))?,
    );
    Ok((
        Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                policy: RuntimePolicy::Strict,
                ..ShardConfig::default()
            },
            StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
        ),
        journal,
    ))
}

impl Case {
    pub(super) fn raw_rows(&self, journal: &FjallJournal) -> TestResult<Vec<Vec<u8>>> {
        (0..=self.origin_seq)
            .map(|seq| {
                journal
                    .get_event_bytes(self.run, EventSeq::new(seq))?
                    .ok_or_else(|| format!("journal event {seq} missing").into())
            })
            .collect()
    }
}

fn waiting_workflow(deadline: u64) -> TestResult<CompiledWorkflow> {
    let value = i64::try_from(deadline)?;
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("origin_recovery"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            vb_core::workflow::node(
                StepIdx::ZERO,
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::ZERO),
                Some(StepIdx::new(1)),
            ),
            vb_core::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::ZERO,
                },
                None,
                None,
            ),
        ]),
        expressions: Box::default(),
        accessors: Box::default(),
        constants: Box::from([ConstValue::I64(value)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::default(),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::default(),
    };
    let encoded = postcard::to_allocvec(&parts)?;
    parts.digest = WorkflowDigest::from_bytes(*blake3::hash(&encoded).as_bytes());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn wall_clock_millis() -> TestResult<u64> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(u64::try_from(millis)?)
}
