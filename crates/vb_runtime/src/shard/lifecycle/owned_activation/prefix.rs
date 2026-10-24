use crate::admission::RunAdmission;
use crate::shard::Shard;
use crate::{RuntimeError, RuntimeResult};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, WorkflowDigest};
use vb_storage::{AcceptedArtifact, EventSeq, FjallJournal, JournalEvent};

#[path = "prefix/artifact.rs"]
mod artifact;
#[path = "prefix/identity.rs"]
mod identity;

pub(super) struct PreparedOwnedActivation {
    pub(super) workflow: CompiledWorkflow,
    pub(super) bundle: OwnedRootBundle,
    pub(super) admission: RunAdmission,
    pub(super) contracts: Box<[vb_core::ActionContract]>,
    pub(super) next_seq: EventSeq,
}

pub(super) fn load(
    shard: &Shard,
    run: RunId,
    source: WorkflowDigest,
) -> RuntimeResult<PreparedOwnedActivation> {
    let journal = shard
        .journal
        .storage_journal()
        .ok_or(RuntimeError::UnsupportedOperation {
            operation: "owned_activation_requires_storage",
        })?;
    let (workflow_digest, admission_event) = pristine(&journal, run)?;
    let header = journal.run_header(run)?.ok_or(RuntimeError::RunNotFound)?;
    identity::header(&header, run, workflow_digest)?;
    identity::source(&journal, source)?;
    let artifact = artifact::load(shard, &journal, &admission_event)?;
    let workflow = crate::admission::verify_stored_artifact(
        shard.artifact_store.as_ref(),
        &artifact,
        Some(RuntimePolicy::Strict),
    )
    .map_err(|_| RuntimeError::AdmissionArtifactInvalid {
        digest: artifact.digest,
    })?;
    if workflow.digest() != workflow_digest || artifact.source_digest != workflow_digest {
        return Err(identity::mismatch());
    }
    super::super::execution_binding::validate(
        shard.artifact_store.as_ref(),
        RuntimePolicy::Strict,
        &workflow,
        &artifact.action_contracts,
    )?;
    owned(&journal, run, source, workflow, artifact, admission_event)
}

fn pristine(journal: &FjallJournal, run: RunId) -> RuntimeResult<(WorkflowDigest, JournalEvent)> {
    let limit = vb_storage::EventReplayLimit::new(2).ok_or(RuntimeError::UnsupportedOperation {
        operation: "owned_activation_prefix_limit",
    })?;
    let events = journal.events_for_run_full_bounded(run, limit)?;
    if events.is_empty() {
        return Err(RuntimeError::RunNotFound);
    }
    let [
        JournalEvent::RunAccepted {
            run: accepted_run,
            seq: accepted_seq,
            workflow,
        },
        admission @ JournalEvent::RunAdmission {
            run: admitted_run,
            seq: admitted_seq,
            ..
        },
    ] = events.as_slice()
    else {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "owned_activation_requires_pristine_prefix",
        });
    };
    if *accepted_run != run
        || *admitted_run != run
        || accepted_seq.get() != 0
        || admitted_seq.get() != 1
    {
        return Err(identity::mismatch());
    }
    Ok((*workflow, admission.clone()))
}

fn owned(
    journal: &FjallJournal,
    run: RunId,
    source: WorkflowDigest,
    workflow: CompiledWorkflow,
    artifact: AcceptedArtifact,
    event: JournalEvent,
) -> RuntimeResult<PreparedOwnedActivation> {
    let record = journal
        .owned_run_inputs(run, artifact.digest, source, &workflow)?
        .ok_or(RuntimeError::UnsupportedOperation {
            operation: "owned_activation_requires_retained_owned_inputs",
        })?;
    record.validate_admission(&event)?;
    let bundle = record.restore(&workflow)?;
    let admission = identity::admission(run, &workflow, &artifact, &event)?;
    let next_seq = event
        .seq()
        .get()
        .checked_add(1)
        .map(EventSeq::new)
        .ok_or(vb_storage::JournalError::SequenceOverflow)?;
    Ok(PreparedOwnedActivation {
        workflow,
        bundle,
        admission,
        contracts: artifact.action_contracts,
        next_seq,
    })
}
