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
    compatibility_event: &JournalEvent,
) -> RuntimeResult<RunAdmission> {
    validate_policy_pin(event, artifact)?;
    let admission = hydrate_run_admission_from_events(std::slice::from_ref(event))
        .ok_or(RuntimeError::RunNotFound)?;
    let compatibility = compatibility_decision(run, artifact, compatibility_event)?;
    let admission = restore_artifact_admission_facts(admission, artifact, compatibility);
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

fn compatibility_decision(
    run: RunId,
    artifact: &AcceptedArtifact,
    event: &JournalEvent,
) -> RuntimeResult<vb_storage::compat::CompatDecision> {
    let JournalEvent::CompatibilityDecisionEvent {
        run: decision_run,
        seq,
        decision,
    } = event
    else {
        return Err(mismatch());
    };
    let expected = crate::admission::check_artifact_ir_version(artifact).map_err(|_| {
        RuntimeError::AdmissionArtifactInvalid {
            digest: artifact.digest,
        }
    })?;
    if *decision_run == run && seq.get() == 2 && *decision == expected {
        Ok(*decision)
    } else {
        Err(mismatch())
    }
}

fn restore_artifact_admission_facts(
    admission: RunAdmission,
    artifact: &AcceptedArtifact,
    compatibility: vb_storage::compat::CompatDecision,
) -> RunAdmission {
    let mut restored = RunAdmission::with_idempotency_evidence(
        admission.artifact_digest(),
        admission.run_id(),
        admission.granted_capabilities().clone(),
        admission.policy(),
        artifact.verification.idempotency_attested.clone(),
    )
    .with_policy_pin(admission.policy_pin())
    .with_input_digest(admission.input_digest())
    .with_ir_compat_decision(compatibility);
    if let Some(budget) = admission.budget() {
        restored = restored.with_attached_budget(budget);
    }
    if let Some(tenant) = admission.tenant() {
        restored = restored.with_tenant(tenant);
    }
    restored
}

fn validate_policy_pin(event: &JournalEvent, artifact: &AcceptedArtifact) -> RuntimeResult<()> {
    let (policy, pin) = match event {
        JournalEvent::RunAdmission {
            policy,
            policy_pin: RunPolicyPin::Pinned(pin),
            ..
        }
        | JournalEvent::RunAdmissionBudget {
            policy,
            policy_pin: RunPolicyPin::Pinned(pin),
            ..
        } => (policy, pin),
        _ => {
            return Err(mismatch());
        }
    };
    if *policy != RuntimePolicy::Strict || *pin != artifact.policy_digest {
        return Err(mismatch());
    }
    Ok(())
}
