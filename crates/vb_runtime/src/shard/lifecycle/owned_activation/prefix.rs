use crate::admission::RunAdmission;
use crate::shard::Shard;
use crate::{RuntimeError, RuntimeResult};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, WorkflowDigest};
use vb_storage::{AcceptedArtifact, EventSeq, FjallJournal, JournalEvent};

#[path = "prefix/artifact.rs"]
mod artifact;
#[path = "prefix/events.rs"]
mod events;
#[path = "prefix/identity.rs"]
mod identity;
#[path = "prefix/trailing.rs"]
mod trailing;

pub(super) struct PreparedOwnedActivation {
    pub(super) workflow: CompiledWorkflow,
    pub(super) bundle: OwnedRootBundle,
    pub(super) admission: RunAdmission,
    pub(super) contracts: Box<[vb_core::ActionContract]>,
    pub(super) next_seq: EventSeq,
    pub(super) run_started_millis: Option<u64>,
    pub(super) journal_bytes_written: u64,
}

struct PristinePrefix {
    workflow_digest: WorkflowDigest,
    admission: JournalEvent,
    next_seq: EventSeq,
    run_started_millis: Option<u64>,
    journal_bytes_written: u64,
    tenant: Option<vb_core::ids::TenantId>,
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
    let prefix = pristine(&journal, run)?;
    if let Some(origin) = prefix.run_started_millis {
        let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or(
            RuntimeError::RecoveryCannotResume {
                reason: String::from("wall_clock_unavailable"),
            },
        )?;
        let budget = match &prefix.admission {
            JournalEvent::RunAdmissionBudget { budget, .. } => Some(budget),
            _ => None,
        };
        crate::recovery::validate_lifetime_budget(budget, origin, now)?;
    }
    let workflow_digest = prefix.workflow_digest;
    let admission_event = prefix.admission.clone();
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
    shard.validate_secret_presence(&workflow)?;
    super::super::execution_binding::validate(
        shard.artifact_store.as_ref(),
        RuntimePolicy::Strict,
        &workflow,
        &artifact.action_contracts,
    )?;
    owned(&journal, run, source, workflow, artifact, prefix)
}

fn pristine(journal: &FjallJournal, run: RunId) -> RuntimeResult<PristinePrefix> {
    let limit = vb_storage::EventReplayLimit::new(5).ok_or(RuntimeError::UnsupportedOperation {
        operation: "owned_activation_prefix_limit",
    })?;
    let events = journal.events_for_run_full_bounded(run, limit)?;
    pristine_events(run, &events)
}

fn pristine_events(run: RunId, events: &[JournalEvent]) -> RuntimeResult<PristinePrefix> {
    let prefix = events::parse(run, events)?;
    let trailing = trailing::parse(run, prefix.trailing, prefix.budgeted)?;
    let journal_bytes_written = crate::recovery::recover_journal_bytes_written(events)?;
    Ok(PristinePrefix {
        workflow_digest: prefix.workflow_digest,
        admission: prefix.admission.clone(),
        next_seq: trailing.next_seq,
        run_started_millis: trailing.origin,
        journal_bytes_written,
        tenant: trailing.tenant,
    })
}
#[cfg(test)]
pub(crate) fn validate_pristine_prefix_events(
    run: RunId,
    events: &[JournalEvent],
) -> RuntimeResult<()> {
    pristine_events(run, events).map(|_| ())
}

fn owned(
    journal: &FjallJournal,
    run: RunId,
    source: WorkflowDigest,
    workflow: CompiledWorkflow,
    artifact: AcceptedArtifact,
    prefix: PristinePrefix,
) -> RuntimeResult<PreparedOwnedActivation> {
    let event = prefix.admission;
    let record = journal
        .owned_run_inputs(run, artifact.digest, source, &workflow)?
        .ok_or(RuntimeError::UnsupportedOperation {
            operation: "owned_activation_requires_retained_owned_inputs",
        })?;
    record.validate_admission(&event)?;
    let bundle = record.restore(&workflow)?;
    let admission = identity::admission(run, &workflow, &artifact, &event)?;
    let admission = match prefix.tenant {
        Some(tenant) if admission.tenant().is_none() => admission.with_tenant(tenant),
        _ => admission,
    };
    Ok(PreparedOwnedActivation {
        workflow,
        bundle,
        admission,
        contracts: artifact.action_contracts,
        next_seq: prefix.next_seq,
        run_started_millis: prefix.run_started_millis,
        journal_bytes_written: prefix.journal_bytes_written,
    })
}
