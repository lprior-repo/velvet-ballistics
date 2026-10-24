use super::support::{TestResult, expected};
use vb_core::{CompiledWorkflow, RuntimePolicy, SlotIdx, ValueStore};
use vb_runtime::run_continuation::ContinuationExpected;
use vb_storage::{
    AcceptedArtifact, FjallJournal, JournalEvent, OwnedRunInputsRecordV1, RunHeaderRecord,
    RunSubmission, WorkflowSourceRecord,
};

pub(super) struct Receipt {
    pub(super) record: OwnedRunInputsRecordV1,
    pub(super) artifact: AcceptedArtifact,
    pub(super) admission: JournalEvent,
    pub(super) expected: ContinuationExpected,
}

pub(super) fn persist(
    workflow: &CompiledWorkflow,
    store: &ValueStore,
    input: SlotIdx,
    source: &WorkflowSourceRecord,
) -> TestResult<Receipt> {
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(workflow, RuntimePolicy::Strict, &[])?;
    let expected = expected(&artifact, source)?;
    let bundle = super::arenas::bundle(workflow, store, input)?;
    let record = OwnedRunInputsRecordV1::new(
        expected.run,
        artifact.digest,
        source.digest,
        &bundle,
        workflow,
    )?;
    let header = RunHeaderRecord {
        run: expected.run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    };
    let submission = RunSubmission::owned(source.clone(), artifact, record, header)?;
    let directory = tempfile::tempdir()?;
    {
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    }
    load(directory.path(), workflow, source, expected)
}

fn load(
    path: &std::path::Path,
    workflow: &CompiledWorkflow,
    source: &WorkflowSourceRecord,
    expected: ContinuationExpected,
) -> TestResult<Receipt> {
    let journal = FjallJournal::open(path, None)?;
    let stored = journal
        .compiled_ir(expected.artifact_digest)?
        .ok_or("artifact missing")?;
    let artifact: AcceptedArtifact = postcard::from_bytes(&stored.ir)?;
    let verified =
        vb_storage::admission::verify_accepted_artifact_for_policy(&artifact, expected.policy)?;
    assert_eq!(verified.to_parts(), workflow.to_parts());
    assert_eq!(
        journal.workflow_source(source.digest)?,
        Some(source.clone())
    );
    let record = journal
        .owned_run_inputs(expected.run, artifact.digest, source.digest, &verified)?
        .ok_or("owned record missing")?;
    let events = journal.events_for_run_full(expected.run)?;
    assert_eq!(events.len(), 2);
    let admission = events
        .into_iter()
        .find(|event| matches!(event, JournalEvent::RunAdmission { .. }))
        .ok_or("admission missing")?;
    match &admission {
        JournalEvent::RunAdmission { seq, .. } => assert_eq!(*seq, expected.prior_seq),
        _ => return Err("selected non-admission".into()),
    }
    record.validate_admission(&admission)?;
    record.validate_workflow(&verified)?;
    Ok(Receipt {
        record,
        artifact,
        admission,
        expected,
    })
}
