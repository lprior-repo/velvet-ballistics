use std::{num::NonZeroUsize, path::Path, sync::Arc};
use vb_core::{
    ActionContract, ActionOutputReady, ActionTicket, CompiledNodeKind, CompiledWorkflow, RunId,
    RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint, ValueStore, WorkflowDigest,
    root_bundle::OwnedRootBundle,
};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};
use vb_storage::{
    FjallJournal, JournalEvent, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const RUN: RunId = RunId::new(47249);
const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: strict_action_preflight\nwhen: {manual: {}}\ninputs: {payload: {is: number}}\nsteps:\n  - id: work\n    do: {action: '0', input: '$input.payload'}\n  - id: done\n    finish: {result: '$step.work'}\n";

pub(super) struct Case {
    directory: tempfile::TempDir,
    workflow: CompiledWorkflow,
    source: WorkflowDigest,
    artifact: WorkflowDigest,
    pub(super) input: SlotIdx,
    pub(super) output: SlotIdx,
    pub(super) action: StepIdx,
}

fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: vb_core::ActionId::new(0),
        name: vb_core::action::ActionName::new("strict-action-preflight")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1_000,
        idempotency: vb_core::Idempotency::IdempotentExternal,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

impl Case {
    pub(super) fn new() -> TestResult<Self> {
        let workflow = vb_compile::compile_workflow(SOURCE)?;
        let input = workflow
            .input_slots()
            .first()
            .ok_or("compiled input absent")?
            .slot;
        let action = workflow
            .to_parts()
            .nodes
            .iter()
            .find_map(|node| matches!(node.kind, CompiledNodeKind::Do { .. }).then_some(node.id))
            .ok_or("compiled Do absent")?;
        let output = workflow
            .node(action)
            .and_then(|node| node.output.slot())
            .ok_or("Do output absent")?;
        let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let roots = OwnedRootBundle::new(
            Box::from([(input, SlotValue::I64(42), Taint::Random)]),
            store.checkpoint_v1(),
            &workflow,
        )?;
        let prepared = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[contract()?],
        )?;
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE).into()),
            source: SOURCE.to_vec(),
        };
        let record =
            OwnedRunInputsRecordV1::new(RUN, prepared.digest, source.digest, &roots, &workflow)?;
        let submission = RunSubmission::owned(
            source.clone(),
            prepared,
            record,
            RunHeaderRecord {
                run: RUN,
                workflow_id: vb_core::WorkflowId::new(0),
                compiled_digest: workflow.digest(),
                status: 0,
                accepted_at_ms: u64::try_from(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_millis(),
                )?,
            },
        )?;
        let directory = tempfile::tempdir()?;
        let artifact = {
            let journal = FjallJournal::open(directory.path(), None)?;
            journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
            journal
                .events_for_run_full(RUN)?
                .iter()
                .find_map(|event| match event {
                    JournalEvent::RunAdmissionBudget {
                        artifact_digest, ..
                    } => Some(*artifact_digest),
                    _ => None,
                })
                .ok_or("issued artifact binding absent")?
        };
        Ok(Self {
            directory,
            workflow,
            source: source.digest,
            artifact,
            input,
            output,
            action,
        })
    }

    pub(super) fn path(&self) -> &Path {
        self.directory.path()
    }

    pub(super) fn receipt(&self, journal: &FjallJournal) -> TestResult<Vec<u8>> {
        let record = journal
            .owned_run_inputs(RUN, self.artifact, self.source, &self.workflow)?
            .ok_or("actual owned receipt absent")?;
        assert_eq!(
            record.roots(),
            [(self.input, SlotValue::I64(42), Taint::Random)]
        );
        Ok(postcard::to_allocvec(&record)?)
    }

    pub(super) fn output(&self) -> TestResult<ActionOutputReady> {
        let value = SlotValue::I64(73);
        Ok(ActionOutputReady {
            output_slot: self.output,
            value,
            taint: Taint::Random,
            encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
        })
    }

    pub(super) fn issue(&self) -> TestResult<ActionTicket> {
        let (journal, mut live) = open(self.path())?;
        live.activate_owned_submission(RUN, self.source)?;
        let ticket = pending(&mut live, self.action)?;
        assert_eq!(
            live.inspect_run_state(RUN)?.frame.read_slot(self.input)?,
            &SlotValue::I64(42)
        );
        assert_eq!(
            live.inspect_run_state(RUN)?.frame.read_taint(self.input)?,
            Taint::Random
        );
        self.receipt(&journal)?;
        Ok(ticket)
    }
}

pub(super) fn open(path: &Path) -> TestResult<(Arc<FjallJournal>, Runtime)> {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 16,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal.clone()),
    )?;
    Ok((journal, runtime))
}

pub(super) fn pending(runtime: &mut Runtime, action: StepIdx) -> TestResult<ActionTicket> {
    for _ in 0..12 {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_action_ticket(RUN, u64::from(action.get())) {
            assert_eq!(
                runtime.pending_boundary_snapshot(8).command_queue_depth(),
                0
            );
            return Ok(ticket);
        }
    }
    Err(format!(
        "actual pending ticket absent: state={:?}; counters={:?}; trace={:?}",
        runtime.inspect_run_state(RUN),
        runtime.counters_snapshot(),
        runtime.list_events(RUN)?
    )
    .into())
}

pub(super) fn rows(path: &Path) -> TestResult<Vec<(String, Vec<u8>, Vec<u8>)>> {
    let database = fjall::Database::builder(path).open()?;
    let mut rows = Vec::new();
    for name in FjallJournal::declared_keyspaces() {
        let keyspace = database.keyspace(name, fjall::KeyspaceCreateOptions::default)?;
        for entry in keyspace.iter() {
            let (key, value) = entry.into_inner()?;
            rows.push((name.to_owned(), key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}
