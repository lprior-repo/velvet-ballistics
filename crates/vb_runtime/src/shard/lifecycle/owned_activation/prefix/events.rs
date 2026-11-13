use super::{JournalEvent, RunId, RuntimeError, RuntimeResult, WorkflowDigest, identity};

pub(super) struct AdmissionPrefix<'a> {
    pub(super) workflow_digest: WorkflowDigest,
    pub(super) admission: &'a JournalEvent,
    pub(super) trailing: &'a [JournalEvent],
    pub(super) budgeted: bool,
}

pub(super) fn parse(run: RunId, events: &[JournalEvent]) -> RuntimeResult<AdmissionPrefix<'_>> {
    let (accepted, rest) = events.split_first().ok_or(RuntimeError::RunNotFound)?;
    let (admission, trailing) = rest.split_first().ok_or(RuntimeError::RunNotFound)?;
    let workflow_digest = accepted_workflow(run, accepted)?;
    let budgeted = admission_is_budgeted(run, admission)?;
    Ok(AdmissionPrefix {
        workflow_digest,
        admission,
        trailing,
        budgeted,
    })
}

fn accepted_workflow(run: RunId, event: &JournalEvent) -> RuntimeResult<WorkflowDigest> {
    match event {
        JournalEvent::RunAccepted {
            run: event_run,
            seq,
            workflow,
        } if *event_run == run && seq.get() == 0 => Ok(*workflow),
        JournalEvent::RunAccepted { .. } => Err(identity::mismatch()),
        _ => Err(invalid_prefix()),
    }
}

fn admission_is_budgeted(run: RunId, event: &JournalEvent) -> RuntimeResult<bool> {
    match event {
        JournalEvent::RunAdmission {
            run: event_run,
            seq,
            ..
        } if *event_run == run && seq.get() == 1 => Ok(false),
        JournalEvent::RunAdmissionBudget {
            run: event_run,
            seq,
            ..
        } if *event_run == run && seq.get() == 1 => Ok(true),
        JournalEvent::RunAdmission { .. } | JournalEvent::RunAdmissionBudget { .. } => {
            Err(identity::mismatch())
        }
        _ => Err(invalid_prefix()),
    }
}

fn invalid_prefix() -> RuntimeError {
    RuntimeError::UnsupportedOperation {
        operation: "owned_activation_requires_pristine_prefix",
    }
}
