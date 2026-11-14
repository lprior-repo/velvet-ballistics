use super::*;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value_store::ValueStore;
use vb_storage::{
    ExecutorCatalogSubmissionContext, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord,
};

pub(super) fn commit(
    journal: &FjallJournal,
    catalog: &ExecutorCatalog,
    workflow: &CompiledWorkflow,
    artifact: &vb_storage::AcceptedArtifact,
    run: RunId,
) -> Result<WorkflowDigest, Box<dyn std::error::Error>> {
    let header = header(run, workflow)?;
    commit_with_header(journal, catalog, workflow, artifact, header)
}

pub(super) fn commit_with_header(
    journal: &FjallJournal,
    catalog: &ExecutorCatalog,
    workflow: &CompiledWorkflow,
    artifact: &vb_storage::AcceptedArtifact,
    header: RunHeaderRecord,
) -> Result<WorkflowDigest, Box<dyn std::error::Error>> {
    assert_rejected(
        journal,
        workflow,
        artifact,
        header.run,
        empty_context(catalog),
    )?;
    assert_rejected(
        journal,
        workflow,
        artifact,
        header.run,
        wrong_context(catalog),
    )?;
    let context = ExecutorCatalogSubmissionContext::new(catalog, required_capabilities());
    let run = header.run;
    let submission =
        prepared_submission_with_header(workflow, artifact, header.run, context, header)?;
    journal.commit_run_submission_with_executor_catalog(
        &submission,
        RuntimePolicy::Strict,
        catalog,
    )?;
    assert_admitted_capability(journal, run)?;
    Ok(source().digest)
}

fn assert_admitted_capability(journal: &FjallJournal, run: RunId) -> TestResult {
    let events = journal.events_for_run_full(run)?;
    let admission = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAdmission {
                granted_capabilities,
                ..
            }
            | JournalEvent::RunAdmissionBudget {
                granted_capabilities,
                ..
            } => Some(granted_capabilities),
            _ => None,
        })
        .ok_or("catalog submission omitted RunAdmission")?;
    assert!(admission.grants(&Capability::new(
        "velvet.register.write".into(),
        ActionId::new(1),
    )));
    assert_eq!(admission.len(), 1);
    Ok(())
}

fn assert_rejected(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    artifact: &vb_storage::AcceptedArtifact,
    run: RunId,
    context: ExecutorCatalogSubmissionContext<'_>,
) -> TestResult {
    let generic = RunSubmission::owned(
        source(),
        artifact.clone(),
        owned_inputs(workflow, artifact, run)?,
        header(run, workflow)?,
    );
    if !matches!(generic, Err(vb_storage::JournalError::ArtifactMalformed)) {
        return Err("generic owned submission must reject packed evidence".into());
    }
    let result = prepared_submission(workflow, artifact, run, context);
    if !matches!(result, Err(vb_storage::JournalError::CapabilityDenied)) {
        return Err("expected exact capability refusal".into());
    }
    assert!(journal.run_header(run)?.is_none());
    assert!(journal.run_inputs(run)?.is_none());
    assert!(journal.events_for_run_full(run)?.is_empty());
    Ok(())
}

fn prepared_submission(
    workflow: &CompiledWorkflow,
    artifact: &vb_storage::AcceptedArtifact,
    run: RunId,
    context: ExecutorCatalogSubmissionContext<'_>,
) -> Result<RunSubmission, vb_storage::JournalError> {
    let header = header(run, workflow)?;
    prepared_submission_with_header(workflow, artifact, run, context, header)
}

fn prepared_submission_with_header(
    workflow: &CompiledWorkflow,
    artifact: &vb_storage::AcceptedArtifact,
    run: RunId,
    context: ExecutorCatalogSubmissionContext<'_>,
    header: RunHeaderRecord,
) -> Result<RunSubmission, vb_storage::JournalError> {
    let source = source();
    let record = owned_inputs(workflow, artifact, run)?;
    RunSubmission::owned_with_executor_catalog(source, artifact.clone(), record, header, context)
}

fn owned_inputs(
    workflow: &CompiledWorkflow,
    artifact: &vb_storage::AcceptedArtifact,
    run: RunId,
) -> Result<OwnedRunInputsRecordV1, vb_storage::JournalError> {
    let source = source();
    OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source.digest,
        &owned_bundle(workflow)?,
        workflow,
    )
}

fn owned_bundle(workflow: &CompiledWorkflow) -> Result<OwnedRootBundle, vb_storage::JournalError> {
    let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    OwnedRootBundle::new(
        Box::from([(SlotIdx::ZERO, SlotValue::I64(42), vb_core::Taint::Clean)]),
        store.checkpoint_v1(),
        workflow,
    )
    .map_err(vb_storage::OwnedInputError::Bundle)
    .map_err(Into::into)
}

fn header(
    run: RunId,
    workflow: &CompiledWorkflow,
) -> Result<RunHeaderRecord, vb_storage::JournalError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| vb_storage::JournalError::ClockUnavailable)?;
    let accepted_at_ms =
        u64::try_from(now.as_millis()).map_err(|_| vb_storage::JournalError::InvalidConfig {
            field: "accepted_at_ms",
            reason: "clock_overflow",
        })?;
    Ok(RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms,
    })
}

fn source() -> WorkflowSourceRecord {
    let bytes = b"vb_ifn5b_owned_registered_input";
    WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(bytes).into()),
        source: bytes.to_vec(),
    }
}

fn required_capabilities() -> vb_core::CapabilitySet {
    vb_core::CapabilitySet::from_grants(Box::from([Capability::new(
        "velvet.register.write".into(),
        ActionId::new(1),
    )]))
}

fn empty_context(catalog: &ExecutorCatalog) -> ExecutorCatalogSubmissionContext<'_> {
    ExecutorCatalogSubmissionContext::new(catalog, vb_core::CapabilitySet::empty())
}

fn wrong_context(catalog: &ExecutorCatalog) -> ExecutorCatalogSubmissionContext<'_> {
    let wrong = vb_core::CapabilitySet::from_grants(Box::from([Capability::new(
        "velvet.register.write".into(),
        ActionId::new(0),
    )]));
    ExecutorCatalogSubmissionContext::new(catalog, wrong)
}
