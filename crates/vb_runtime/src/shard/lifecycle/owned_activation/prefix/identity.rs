use crate::admission::RunAdmission;
use crate::recovery::{
    RecoveredAdmissionContext, RecoveredAdmissionEvidence, hydrate_run_admission_from_events,
    validate_recovered_admission_evidence,
};
use crate::{RuntimeError, RuntimeResult};
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, WorkflowDigest};
use vb_storage::{AcceptedArtifact, FjallJournal, JournalEvent, RunHeaderRecord, RunPolicyPin};

pub(super) fn mismatch() -> RuntimeError {
    vb_storage::JournalError::OwnedInputs(vb_storage::OwnedInputError::IdentityMismatch).into()
}

pub(super) fn header(
    header: &RunHeaderRecord,
    run: RunId,
    workflow: WorkflowDigest,
) -> RuntimeResult<()> {
    if header.run != run || header.compiled_digest != workflow || header.status != 0 {
        return Err(mismatch());
    }
    Ok(())
}

pub(super) fn source(journal: &FjallJournal, expected: WorkflowDigest) -> RuntimeResult<()> {
    let record = journal
        .workflow_source(expected)?
        .ok_or(RuntimeError::RunNotFound)?;
    let actual = WorkflowDigest::from_bytes(blake3::hash(&record.source).into());
    if record.digest != expected || actual != expected {
        return Err(mismatch());
    }
    Ok(())
}

pub(super) fn admission(
    run: RunId,
    workflow: &CompiledWorkflow,
    artifact: &AcceptedArtifact,
    event: &JournalEvent,
) -> RuntimeResult<RunAdmission> {
    let JournalEvent::RunAdmission {
        policy,
        policy_pin: RunPolicyPin::Pinned(pin),
        ..
    } = event
    else {
        return Err(mismatch());
    };
    if *policy != RuntimePolicy::Strict || *pin != artifact.policy_digest {
        return Err(mismatch());
    }
    let admission = hydrate_run_admission_from_events(std::slice::from_ref(event))
        .ok_or(RuntimeError::RunNotFound)?;
    validate_recovered_admission_evidence(RecoveredAdmissionEvidence {
        context: RecoveredAdmissionContext {
            run,
            expected_workflow: workflow.digest(),
            runtime_policy: RuntimePolicy::Strict,
        },
        admission: &admission,
        artifact,
        workflow,
    })?;
    Ok(admission)
}
