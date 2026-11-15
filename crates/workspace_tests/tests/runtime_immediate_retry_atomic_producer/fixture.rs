use super::{RUN, TestResult, cut, runtime};
use std::{
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use vb_core::{
    ActionContract, ActionFailure, ActionFailureCode, ActionOutputReady, ActionTicket,
    CapabilitySet, CompiledWorkflow, RuntimePolicy, SlotValue, Taint, ValueStore, WorkflowDigest,
};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime};
use vb_storage::{
    FjallJournal, JournalEvent, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord,
};

pub(super) struct Case {
    pub directory: tempfile::TempDir,
    pub ticket: ActionTicket,
    workflow: CompiledWorkflow,
    artifact: WorkflowDigest,
}

fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: vb_core::ActionId::new(0),
        name: vb_core::action::ActionName::new("atomic-retry-producer")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: vb_core::Idempotency::DeterministicPure,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

impl Case {
    pub(super) fn new(owned: bool, handler: bool) -> TestResult<Self> {
        Self::configured(owned, handler, None)
    }

    pub(super) fn with_journal_batch_limit(limit: u32) -> TestResult<Self> {
        Self::configured(false, false, Some(limit))
    }

    pub(super) fn batch_limit(&self) -> u32 {
        self.workflow.resource_contract().max_journal_batch_bytes
    }

    fn configured(owned: bool, handler: bool, batch_limit: Option<u32>) -> TestResult<Self> {
        let route = if handler {
            "    on_error: {handler: observe}\n"
        } else {
            ""
        };
        let source = format!(
            "version: velvet-ballistics/v1\nname: atomic_retry_producer\nwhen: {{manual: {{}}}}\ninputs: {{payload: any}}\nsteps:\n  - id: work\n{route}    try_again: {{max_attempts: 3, delay: '0ms'}}\n    do: {{action: '0', input: '$input.payload'}}\n  - id: observe\n    ask: {{prompt: '$input.payload'}}\n  - id: done\n    finish: {{result: '$input.payload'}}\n"
        );
        let workflow = configured_workflow(
            vb_compile::compile_workflow(source.as_bytes())?,
            batch_limit,
        )?;
        let input = workflow.input_slots().first().ok_or("input absent")?.slot;
        let directory = tempfile::tempdir()?;
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        let contract = contract()?;
        if owned {
            let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
            let list = store.insert_list_with_taint(
                Box::from([SlotValue::I64(42)]),
                Box::from([Taint::Clean]),
            )?;
            let roots = vb_core::root_bundle::OwnedRootBundle::new(
                Box::from([(input, SlotValue::List(list), Taint::Clean)]),
                store.checkpoint_v1(),
                &workflow,
            )?;
            let prepared = vb_storage::admission::prepare_compiled_artifact(
                &workflow,
                RuntimePolicy::Strict,
                &[contract],
            )?;
            let source = WorkflowSourceRecord {
                digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
                source: source.into_bytes(),
            };
            let record = OwnedRunInputsRecordV1::new(
                RUN,
                prepared.digest,
                source.digest,
                &roots,
                &workflow,
            )?;
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
                        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
                    )?,
                },
            )?;
            journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
            let mut live = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
            live.activate_owned_submission(RUN, source.digest)?;
            let ticket = pending(&mut live)?;
            let artifact = artifact(&journal)?;
            Ok(Self {
                directory,
                ticket,
                workflow,
                artifact,
            })
        } else {
            vb_storage::submit_artifact_with_contracts(
                &journal,
                &workflow,
                RuntimePolicy::Strict,
                &[contract.clone()],
            )?;
            let mut live = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
            live.submit_direct_with_inputs_grants_and_contracts(
                RUN,
                workflow.clone(),
                Box::from([(input, SlotValue::I64(42))]),
                CapabilitySet::empty(),
                Box::from([contract]),
            )?;
            let ticket = pending(&mut live)?;
            let artifact = artifact(&journal)?;
            Ok(Self {
                directory,
                ticket,
                workflow,
                artifact,
            })
        }
    }

    pub(super) fn open(
        &self,
        mode: cut::Mode,
    ) -> TestResult<(Arc<FjallJournal>, Arc<cut::CutJournal>, Runtime)> {
        self.open_with_tick_budget(mode, 16)
    }

    pub(super) fn open_with_tick_budget(
        &self,
        mode: cut::Mode,
        tick_budget: u64,
    ) -> TestResult<(Arc<FjallJournal>, Arc<cut::CutJournal>, Runtime)> {
        let baseline = rows(self.directory.path())?;
        let journal = Arc::new(FjallJournal::open(self.directory.path(), None)?);
        let fault = Arc::new(cut::CutJournal::new(journal.clone(), mode, baseline));
        let mut live = super::runtime_with_budget(fault.clone(), tick_budget)?;
        live.recover_and_resume(RUN)?;
        assert_eq!(pending(&mut live)?, self.ticket);
        Ok((journal, fault, live))
    }

    pub(super) fn receipt(&self, journal: &FjallJournal) -> TestResult<Vec<u8>> {
        match journal.retained_run_inputs_for_recovery(RUN, self.artifact, &self.workflow)? {
            Some(vb_storage::RetainedRunInputs::Owned(record)) => {
                Ok(postcard::to_allocvec(&record)?)
            }
            Some(vb_storage::RetainedRunInputs::Scalar(record)) => {
                Ok(postcard::to_allocvec(&record)?)
            }
            None => Err("actual retained receipt absent".into()),
        }
    }

    pub(super) fn output(&self) -> TestResult<ActionOutputReady> {
        let output_slot = self
            .workflow
            .node(self.ticket.step)
            .and_then(|node| node.output.slot())
            .ok_or("output absent")?;
        let value = SlotValue::I64(73);
        Ok(ActionOutputReady {
            output_slot,
            value,
            taint: Taint::Clean,
            encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
        })
    }
}

fn configured_workflow(
    workflow: CompiledWorkflow,
    limit: Option<u32>,
) -> TestResult<CompiledWorkflow> {
    let Some(limit) = limit else {
        return Ok(workflow);
    };
    let mut parts = workflow.to_parts();
    parts.resource_contract.max_journal_batch_bytes = limit;
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn artifact(journal: &FjallJournal) -> TestResult<WorkflowDigest> {
    journal
        .events_for_run_full(RUN)?
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAdmissionBudget {
                artifact_digest, ..
            } => Some(*artifact_digest),
            _ => None,
        })
        .ok_or_else(|| "issued artifact absent".into())
}

pub(super) fn pending(live: &mut Runtime) -> TestResult<ActionTicket> {
    for _ in 0..12 {
        live.tick_all()?;
        if let Some(ticket) = live.lookup_pending_action_ticket(RUN, 0) {
            return Ok(ticket);
        }
    }
    Err("actual pending action absent".into())
}

pub(super) fn failure() -> ActionFailure {
    ActionFailureCode::Timeout.into()
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
