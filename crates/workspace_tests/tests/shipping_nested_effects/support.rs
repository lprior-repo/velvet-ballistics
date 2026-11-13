#![forbid(unsafe_code)]
//! Shared owned-admission fixture for three-level nested effect qualification.
//!
//! Bead: vb-eirmk.4.7.3. Every case starts from real source bytes, compiles to
//! checked IR, is admitted as an owned submission on a strict Fjall journal,
//! and drives the public `Runtime` ports.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::join_taint;
use vb_core::{
    ActionId, CompiledWorkflow, RunId, RuntimePolicy, SlotValue, StepIdx, Taint, ValueStore,
    WorkflowDigest,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    FjallJournal, JournalEvent, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord,
};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// One owned submission whose Fjall directory survives runtime drops.
pub(crate) struct Case {
    pub(crate) directory: tempfile::TempDir,
    pub(crate) workflow: CompiledWorkflow,
    pub(crate) source_digest: WorkflowDigest,
    pub(crate) run: RunId,
}

impl Case {
    pub(crate) fn new(
        source: &str,
        run: RunId,
        values: &[SlotValue],
        taints: &[Taint],
        contracts: &[ActionContract],
    ) -> TestResult<Self> {
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        vb_validate::shared::validate(&workflow.to_parts())?;
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let list = store.insert_list_with_taint(
            values.to_vec().into_boxed_slice(),
            taints.to_vec().into_boxed_slice(),
        )?;
        let root_taint = taints
            .iter()
            .fold(Taint::Clean, |joined, taint| join_taint(joined, *taint));
        let input = workflow
            .input_slots()
            .first()
            .ok_or("missing list input")?
            .slot;
        let bundle = OwnedRootBundle::new(
            Box::from([(input, SlotValue::List(list), root_taint)]),
            store.checkpoint_v1(),
            &workflow,
        )?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            contracts,
        )?;
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into());
        let record =
            OwnedRunInputsRecordV1::new(run, artifact.digest, source_digest, &bundle, &workflow)?;
        let submission = RunSubmission::owned(
            WorkflowSourceRecord {
                digest: source_digest,
                source: source.as_bytes().to_vec(),
            },
            artifact,
            record,
            RunHeaderRecord {
                run,
                workflow_id: vb_core::WorkflowId::new(0),
                compiled_digest: workflow.digest(),
                status: 0,
                accepted_at_ms: 123,
            },
        )?;
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        Ok(Self {
            directory,
            workflow,
            source_digest,
            run,
        })
    }

    pub(crate) fn open(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        Ok(self.open_with_quantum(1)?)
    }

    pub(crate) fn open_with_quantum(
        &self,
        quantum: u64,
    ) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let journal = Arc::new(FjallJournal::open(self.directory.path(), None)?);
        let runtime = Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                policy: RuntimePolicy::Strict,
                step_budget_per_tick: quantum,
                ..ShardConfig::default()
            },
            StorageRuntimeJournal::shared_strict(journal.clone()),
        );
        Ok((journal, runtime))
    }

    pub(crate) fn activate_live(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let (journal, runtime) = self.open()?;
        runtime.activate_owned_submission(self.run, self.source_digest)?;
        Ok((journal, runtime))
    }

    pub(crate) fn recover(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let (journal, runtime) = self.open()?;
        match runtime.recover_product(self.run)? {
            vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(_) => {}
            other => return Err(format!("owned recovery is not resumable: {other:?}").into()),
        }
        runtime.recover_and_resume(self.run)?;
        Ok((journal, runtime))
    }

    pub(crate) fn events(&self, journal: &FjallJournal) -> TestResult<Vec<JournalEvent>> {
        Ok(journal.events_for_run_full(self.run)?)
    }

    pub(crate) fn step(
        &self,
        predicate: impl Fn(&vb_core::CompiledNodeKind) -> bool,
    ) -> TestResult<StepIdx> {
        self.workflow
            .to_parts()
            .nodes
            .iter()
            .find(|node| predicate(&node.kind))
            .map(|node| node.id)
            .ok_or_else(|| "step not found".into())
    }
}

pub(crate) fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("nested-effects")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

pub(crate) fn count<F>(events: &[JournalEvent], mut predicate: F) -> usize
where
    F: FnMut(&JournalEvent) -> bool,
{
    events.iter().filter(|event| predicate(event)).count()
}

pub(crate) fn run_finished(events: &[JournalEvent]) -> TestResult<(SlotValue, Taint)> {
    let mut finished = events.iter().filter_map(|event| match event {
        JournalEvent::RunFinished { result, .. } => Some(*result),
        _ => None,
    });
    let slot = finished.next().ok_or("missing RunFinished")?;
    if finished.next().is_some() {
        return Err("multiple RunFinished events".into());
    }
    let (value, extra) = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                slot: written,
                value,
                extra,
                ..
            } if *written == slot => Some((value, extra)),
            _ => None,
        })
        .ok_or("missing terminal result write")?;
    let Some(bytes) = value.as_deref() else {
        return Err("terminal result has no payload".into());
    };
    let taint = match extra.as_deref().map(vb_storage::decode_slot_written_extra) {
        Some(Ok(vb_storage::DecodedSlotWrittenExtra::Envelope(envelope))) => envelope.taint,
        Some(Ok(vb_storage::DecodedSlotWrittenExtra::LegacyFrameExtra(_))) => Taint::Clean,
        Some(Err(error)) => return Err(format!("slot-write extra decode: {error:?}").into()),
        None => Taint::Clean,
    };
    Ok((postcard::from_bytes(bytes)?, taint))
}

/// Asserts the run reached exactly one clean terminal completion.
pub(crate) fn assert_completed(runtime: &Runtime, events: &[JournalEvent]) {
    assert_eq!(runtime.counters_snapshot().runs_completed, 1, "{events:?}");
    assert_eq!(runtime.counters_snapshot().runs_failed, 0, "{events:?}");
    assert_eq!(
        count(events, |event| matches!(
            event,
            JournalEvent::RunFinished { .. }
        )),
        1
    );
}

/// Journal length used to prove a refused authority appended nothing.
pub(crate) fn journal_len(case: &Case, journal: &FjallJournal) -> TestResult<usize> {
    Ok(case.events(journal)?.len())
}
