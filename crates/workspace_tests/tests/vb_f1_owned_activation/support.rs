use core::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, SlotValue, Taint, WorkflowDigest};
use vb_runtime::journal::{SharedRuntimeJournal, StorageRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    AcceptedArtifact, FjallJournal, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const SOURCE: &str = r#"version: velvet-ballistics/v1
name: owned_activation
when:
  manual: {}
inputs:
  entries: { is: list, of: any, max: 8 }
steps:
  - id: bounded
    collect:
      variable: batch
      source: "$input.entries"
      pages: 3
      items: 2
      steps:
        - id: first
          set: { output: copied, value: "$batch" }
        - id: second
          set: { output: echoed, value: "$copied" }
  - id: done
    finish: { result: "$step.bounded" }
"#;

pub(super) struct Case {
    pub(super) directory: tempfile::TempDir,
    pub(super) workflow: CompiledWorkflow,
    pub(super) source: WorkflowSourceRecord,
    pub(super) artifact: AcceptedArtifact,
    pub(super) input: SlotIdx,
    pub(super) root: vb_core::ListId,
    pub(super) initial: ValueStoreCheckpointV1,
    pub(super) input_digest: WorkflowDigest,
    pub(super) run: RunId,
}

impl Case {
    pub(super) fn new(empty: bool, policy: RuntimePolicy) -> TestResult<Self> {
        let workflow =
            vb_compile::compile_workflow(SOURCE.as_bytes()).map_err(|e| format!("{e:?}"))?;
        let (store, root) = super::arenas::original(&workflow, empty)?;
        let input = workflow.input_slots().first().ok_or("input missing")?.slot;
        let bundle = OwnedRootBundle::new(
            Box::from([(input, SlotValue::List(root), Taint::Secret)]),
            store.checkpoint_v1(),
            &workflow,
        )?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(&workflow, policy, &[])?;
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE.as_bytes()).into()),
            source: SOURCE.as_bytes().to_vec(),
        };
        let run = RunId::new(17);
        let (directory, input_digest) =
            persist(&workflow, &source, &artifact, &bundle, run, policy)?;
        Ok(Self {
            directory,
            workflow,
            source,
            artifact,
            input,
            root,
            initial: bundle.checkpoint().clone(),
            input_digest,
            run,
        })
    }

    pub(super) fn open(&self) -> TestResult<Arc<FjallJournal>> {
        Ok(Arc::new(FjallJournal::open(self.directory.path(), None)?))
    }

    pub(super) fn runtime(&self, journal: Arc<FjallJournal>, budget: u64) -> TestResult<Runtime> {
        runtime(
            StorageRuntimeJournal::shared_strict(journal),
            config(budget, 8, 8),
        )
    }

    pub(super) fn output(&self, raw: u16) -> TestResult<SlotIdx> {
        self.workflow
            .node(vb_core::StepIdx::new(raw))
            .and_then(|node| node.output)
            .ok_or_else(|| "compiled output missing".into())
    }
}

pub(super) fn header(run: RunId, workflow: &CompiledWorkflow) -> RunHeaderRecord {
    RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    }
}

pub(super) fn config(budget: u64, queue: usize, capacity: usize) -> ShardConfig {
    ShardConfig {
        command_queue_capacity: queue,
        max_active_runs: capacity,
        step_budget_per_tick: budget,
        ..ShardConfig::default()
    }
}

pub(super) fn runtime(journal: SharedRuntimeJournal, config: ShardConfig) -> TestResult<Runtime> {
    Ok(Runtime::new(
        NonZeroUsize::new(1).ok_or("zero shard count")?,
        config,
        journal,
    ))
}

pub(super) fn assert_pristine(case: &Case, journal: &FjallJournal) -> TestResult {
    let events = journal.events_for_run_full(case.run)?;
    assert_eq!(events.len(), 2);
    assert!(
        matches!(events.first(), Some(vb_storage::JournalEvent::RunAccepted { run, seq, workflow })
        if *run == case.run && seq.get() == 0 && *workflow == case.workflow.digest())
    );
    assert!(
        matches!(events.get(1), Some(vb_storage::JournalEvent::RunAdmission {
        run, seq, input_digest: vb_storage::RunInputBinding::Digest(digest), .. })
        if *run == case.run && seq.get() == 1 && *digest == case.input_digest)
    );
    let record = journal
        .owned_run_inputs(
            case.run,
            case.artifact.digest,
            case.source.digest,
            &case.workflow,
        )?
        .ok_or("owned receipt missing")?;
    record.validate_admission(events.get(1).ok_or("admission missing")?)?;
    assert_eq!(record.checkpoint(), &case.initial);
    Ok(())
}

fn persist(
    workflow: &CompiledWorkflow,
    source: &WorkflowSourceRecord,
    artifact: &AcceptedArtifact,
    bundle: &OwnedRootBundle,
    run: RunId,
    policy: RuntimePolicy,
) -> TestResult<(tempfile::TempDir, WorkflowDigest)> {
    let record =
        OwnedRunInputsRecordV1::new(run, artifact.digest, source.digest, bundle, workflow)?;
    let input_digest = record.input_digest();
    let directory = tempfile::tempdir()?;
    let submission = RunSubmission::owned(
        source.clone(),
        artifact.clone(),
        record,
        header(run, workflow),
    )?;
    {
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, policy)?;
    }
    Ok((directory, input_digest))
}

pub(super) fn additional_run(case: &Case, journal: &FjallJournal, run: RunId) -> TestResult {
    let bundle = OwnedRootBundle::new(
        Box::from([(case.input, SlotValue::List(case.root), Taint::Secret)]),
        case.initial.clone(),
        &case.workflow,
    )?;
    let record = OwnedRunInputsRecordV1::new(
        run,
        case.artifact.digest,
        case.source.digest,
        &bundle,
        &case.workflow,
    )?;
    let submission = RunSubmission::owned(
        case.source.clone(),
        case.artifact.clone(),
        record,
        header(run, &case.workflow),
    )?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    Ok(())
}
