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
    pub(super) started_millis: u64,
}

pub(super) fn persist(
    workflow: &CompiledWorkflow,
    store: &ValueStore,
    input: SlotIdx,
    source: &WorkflowSourceRecord,
) -> TestResult<Receipt> {
    let bundle = super::arenas::bundle(workflow, store, input)?;
    persist_owned(workflow, source, &bundle, &[])
}

pub(super) fn persist_owned(
    workflow: &CompiledWorkflow,
    source: &WorkflowSourceRecord,
    bundle: &vb_core::root_bundle::OwnedRootBundle,
    contracts: &[vb_core::action::ActionContract],
) -> TestResult<Receipt> {
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        workflow,
        RuntimePolicy::Strict,
        contracts,
    )?;
    let expected = expected(&artifact, source)?;
    let record = OwnedRunInputsRecordV1::new(
        expected.run,
        artifact.digest,
        source.digest,
        bundle,
        workflow,
    )?;
    let started_millis = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let header = RunHeaderRecord {
        run: expected.run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: started_millis,
    };
    let submission = RunSubmission::owned(source.clone(), artifact.clone(), record, header)?;
    let directory = tempfile::tempdir()?;
    {
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    }
    load(
        directory.path(),
        workflow,
        source,
        &artifact,
        expected,
        started_millis,
    )
}

fn load(
    path: &std::path::Path,
    workflow: &CompiledWorkflow,
    source: &WorkflowSourceRecord,
    prepared: &AcceptedArtifact,
    mut expected: ContinuationExpected,
    expected_started_millis: u64,
) -> TestResult<Receipt> {
    let journal = FjallJournal::open(path, None)?;
    let events = journal.events_for_run_full(expected.run)?;
    let issued_digest = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAdmissionBudget {
                artifact_digest, ..
            } => Some(*artifact_digest),
            _ => None,
        })
        .ok_or("budget receipt missing")?;
    let stored = journal
        .compiled_ir(issued_digest)?
        .ok_or("artifact missing")?;
    let artifact: AcceptedArtifact = postcard::from_bytes(&stored.ir)?;
    assert_eq!(artifact.digest, issued_digest);
    assert_eq!(artifact.accepted_at_seq, vb_storage::EventSeq::new(1));
    assert_ne!(artifact.digest, expected.artifact_digest);
    let mut expected_artifact = prepared.clone();
    expected_artifact.accepted_at_seq = vb_storage::EventSeq::new(1);
    expected_artifact.digest = vb_storage::admission::accepted_artifact_digest(&expected_artifact)?;
    expected_artifact.verification.digest = expected_artifact.digest;
    assert_eq!(artifact, expected_artifact);
    expected.artifact_digest = artifact.digest;
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
    // Submission writes the accepted, budgeted admission, compatibility, and origin prefix.
    assert_eq!(events.len(), 4);
    let [
        JournalEvent::RunAccepted {
            run,
            seq,
            workflow: accepted_workflow,
        },
        admission @ JournalEvent::RunAdmissionBudget { .. },
        JournalEvent::CompatibilityDecisionEvent {
            run: compatibility_run,
            seq: compatibility_seq,
            ..
        },
        JournalEvent::RunBudgetOriginEvent {
            run: origin_run,
            seq: origin_seq,
            started_millis,
        },
    ] = events.as_slice()
    else {
        return Err(
            "submission event prefix must be accepted, budgeted admission, compatibility, origin"
                .into(),
        );
    };
    assert_eq!(
        (*run, *seq, *accepted_workflow),
        (
            expected.run,
            vb_storage::EventSeq::new(0),
            workflow.digest()
        )
    );
    assert_eq!(
        (*compatibility_run, *compatibility_seq),
        (expected.run, vb_storage::EventSeq::new(2))
    );
    assert_eq!(
        (*origin_run, *origin_seq),
        (expected.run, vb_storage::EventSeq::new(3))
    );
    assert_eq!(*started_millis, expected_started_millis);
    let admission = admission.clone();
    match &admission {
        JournalEvent::RunAdmissionBudget {
            run,
            seq,
            artifact_digest,
            input_digest,
            policy_pin,
            granted_capabilities,
            policy,
            budget,
        } => {
            assert_eq!((*run, *seq), (expected.run, vb_storage::EventSeq::new(1)));
            assert_eq!(*artifact_digest, expected.artifact_digest);
            assert_eq!(
                *input_digest,
                vb_storage::RunInputBinding::Digest(record.input_digest())
            );
            assert_eq!(
                *policy_pin,
                vb_storage::RunPolicyPin::Pinned(artifact.policy_digest)
            );
            assert_eq!(*granted_capabilities, vb_core::CapabilitySet::empty());
            assert_eq!(*policy, expected.policy);
            assert_eq!(
                *budget,
                vb_core::AggregateResourceBudget::from_workflow(&verified)?
            );
        }
        _ => return Err("submission admission must carry the current budget".into()),
    }
    assert_eq!(admission.seq(), expected.prior_seq);
    record.validate_admission(&admission)?;
    record.validate_workflow(&verified)?;
    Ok(Receipt {
        record,
        artifact,
        admission,
        expected,
        started_millis: expected_started_millis,
    })
}
