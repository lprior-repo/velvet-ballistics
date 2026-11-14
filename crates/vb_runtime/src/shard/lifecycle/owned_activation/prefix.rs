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
    run: RunId,
    source: WorkflowDigest,
    compatibility: JournalEvent,
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
    let prefix = pristine(&journal, run, source)?;
    validate_origin(&prefix)?;
    let workflow_digest = prefix.workflow_digest;
    let admission_event = prefix.admission.clone();
    let header = journal.run_header(run)?.ok_or(RuntimeError::RunNotFound)?;
    identity::header(&header, run, workflow_digest)?;
    identity::source(&journal, source)?;
    let artifact = artifact::load(shard, &journal, &admission_event)?;
    let workflow = verified_workflow(shard, &artifact, workflow_digest)?;
    owned(&journal, workflow, artifact, prefix)
}

fn pristine(
    journal: &FjallJournal,
    run: RunId,
    source: WorkflowDigest,
) -> RuntimeResult<PristinePrefix> {
    let limit = vb_storage::EventReplayLimit::new(5).ok_or(RuntimeError::UnsupportedOperation {
        operation: "owned_activation_prefix_limit",
    })?;
    let events = journal.events_for_run_full_bounded(run, limit)?;
    pristine_events(run, source, &events)
}

fn pristine_events(
    run: RunId,
    source: WorkflowDigest,
    events: &[JournalEvent],
) -> RuntimeResult<PristinePrefix> {
    let prefix = events::parse(run, events)?;
    let trailing = trailing::parse(run, prefix.trailing, prefix.budgeted)?;
    let journal_bytes_written = crate::recovery::recover_journal_bytes_written(events)?;
    let compatibility = prefix
        .trailing
        .iter()
        .find(|event| matches!(event, JournalEvent::CompatibilityDecisionEvent { .. }))
        .ok_or_else(identity::mismatch)?
        .clone();
    Ok(PristinePrefix {
        run,
        source,
        compatibility,
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
    pristine_events(run, WorkflowDigest::from_bytes([0; 32]), events).map(|_| ())
}

fn owned(
    journal: &FjallJournal,
    workflow: CompiledWorkflow,
    artifact: AcceptedArtifact,
    prefix: PristinePrefix,
) -> RuntimeResult<PreparedOwnedActivation> {
    let record = journal
        .owned_run_inputs(prefix.run, artifact.digest, prefix.source, &workflow)?
        .ok_or(RuntimeError::UnsupportedOperation {
            operation: "owned_activation_requires_retained_owned_inputs",
        })?;
    record.validate_admission(&prefix.admission)?;
    let bundle = record.restore(&workflow)?;
    let admission = restored_admission(&prefix, &workflow, &artifact)?;
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

fn validate_origin(prefix: &PristinePrefix) -> RuntimeResult<()> {
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
    Ok(())
}

fn verified_workflow(
    shard: &Shard,
    artifact: &AcceptedArtifact,
    workflow_digest: WorkflowDigest,
) -> RuntimeResult<CompiledWorkflow> {
    let workflow = crate::admission::verify_stored_artifact(
        shard.artifact_store.as_ref(),
        artifact,
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
    Ok(workflow)
}

fn restored_admission(
    prefix: &PristinePrefix,
    workflow: &CompiledWorkflow,
    artifact: &AcceptedArtifact,
) -> RuntimeResult<RunAdmission> {
    let admission = identity::admission(
        prefix.run,
        workflow,
        artifact,
        &prefix.admission,
        &prefix.compatibility,
    )?;
    let admission = match prefix.tenant {
        Some(tenant) if admission.tenant().is_none() => admission.with_tenant(tenant),
        _ => admission,
    };
    Ok(admission)
}
