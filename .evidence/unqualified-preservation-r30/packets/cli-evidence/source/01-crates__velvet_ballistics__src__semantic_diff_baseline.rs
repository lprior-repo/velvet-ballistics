#![forbid(unsafe_code)]
use std::path::Path;
use vb_core::{CompiledWorkflow, RunId, WorkflowDigest};
use vb_storage::{AcceptedArtifact, FjallJournal};

type Accepted = (AcceptedArtifact, CompiledWorkflow);

pub(super) struct Baseline {
    pub(super) workflow: CompiledWorkflow,
    pub(super) source: Option<Vec<u8>>,
    pub(super) label: String,
    pub(super) action_contracts: Option<Box<[vb_core::ActionContract]>>,
}

pub(super) fn load(db: Option<&Path>, name: &str) -> Result<Baseline, String> {
    let db = db.ok_or("previous-artifact comparison requires --db or --against")?;
    if !db.is_dir() || !db.join("version").is_file() {
        return Err("baseline database does not exist".into());
    }
    let journal = FjallJournal::open(db, None).map_err(|error| error.to_string())?;
    let accepted = latest_accepted(&journal, name)?;
    let source = retained_source(&journal, &accepted)?;
    Ok(Baseline {
        workflow: accepted.1,
        source,
        label: format!(
            "{}:accepted_at_seq={}",
            db.display(),
            accepted.0.accepted_at_seq.get()
        ),
        action_contracts: Some(canonical_contracts(&accepted.0)?),
    })
}

pub(crate) fn canonical_contracts(
    artifact: &AcceptedArtifact,
) -> Result<Box<[vb_core::ActionContract]>, String> {
    let policy = match &artifact.verification.evidence {
        vb_core::artifact::ArtifactEvidence::Verified(base) => base.admission_policy,
        vb_core::artifact::ArtifactEvidence::Packed(packed) => packed.base.admission_policy,
        vb_core::artifact::ArtifactEvidence::Unverified => {
            return Err("baseline artifact is unverified".into());
        }
    };
    let bytes = postcard::to_allocvec(artifact).map_err(|error| error.to_string())?;
    vb_storage::verify_retained_artifact(&bytes, policy)
        .map_err(|error| format!("baseline artifact cannot be verified: {error}"))?
        .try_canonical()
        .map_err(|error| format!("baseline action contracts cannot be projected: {error}"))
}

fn latest_accepted(journal: &FjallJournal, name: &str) -> Result<Accepted, String> {
    journal
        .list_artifacts()
        .map_err(|error| error.to_string())?
        .into_iter()
        .try_fold(None, |latest, digest| {
            Ok::<_, String>(select_latest(latest, candidate(journal, digest, name)?))
        })?
        .ok_or_else(|| {
            "no prior accepted artifact matches the workflow name: no issued acceptance sequence"
                .into()
        })
}

fn candidate(
    journal: &FjallJournal,
    digest: WorkflowDigest,
    name: &str,
) -> Result<Option<Accepted>, String> {
    let record = journal
        .compiled_ir(digest)
        .map_err(|error| error.to_string())?
        .ok_or("baseline artifact disappeared")?;
    let artifact: AcceptedArtifact = postcard::from_bytes(&record.ir)
        .map_err(|error| format!("baseline artifact cannot be decoded: {error}"))?;
    let workflow = vb_storage::admission::verify_accepted_artifact(&artifact)
        .map_err(|error| format!("baseline artifact cannot be verified: {error}"))?;
    if workflow.name() != name || artifact.accepted_at_seq.get() == 0 {
        return Ok(None);
    }
    Ok(Some((artifact, workflow)))
}

fn select_latest(latest: Option<Accepted>, candidate: Option<Accepted>) -> Option<Accepted> {
    let Some(candidate) = candidate else {
        return latest;
    };
    match latest {
        Some(previous) if previous.0.accepted_at_seq >= candidate.0.accepted_at_seq => {
            Some(previous)
        }
        _ => Some(candidate),
    }
}

fn retained_source(journal: &FjallJournal, accepted: &Accepted) -> Result<Option<Vec<u8>>, String> {
    journal
        .run_headers()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|header| header.compiled_digest == accepted.1.digest())
        .try_fold(None, |source, header| {
            if source.is_some() {
                return Ok(source);
            }
            source_for_run(journal, header.run, accepted)
        })
}

fn source_for_run(
    journal: &FjallJournal,
    run: RunId,
    accepted: &Accepted,
) -> Result<Option<Vec<u8>>, String> {
    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    let Some(admission) = admission_for_artifact(&events, accepted.0.digest) else {
        return Ok(None);
    };
    let digest = retained_input_source(journal, run, accepted, admission)?;
    match digest {
        Some(digest) => matching_source(journal, digest, &accepted.1),
        None => Ok(None),
    }
}

fn admission_for_artifact(
    events: &[vb_storage::JournalEvent],
    selected: WorkflowDigest,
) -> Option<&vb_storage::JournalEvent> {
    events.iter().find(|event| {
        matches!(event,
            vb_storage::JournalEvent::RunAdmissionBudget { artifact_digest, .. }
                if *artifact_digest == selected
        )
    })
}

fn retained_input_source(
    journal: &FjallJournal,
    run: RunId,
    accepted: &Accepted,
    admission: &vb_storage::JournalEvent,
) -> Result<Option<WorkflowDigest>, String> {
    let inputs = journal
        .admitted_run_inputs(run, accepted.0.digest, &accepted.1, admission)
        .map_err(|error| error.to_string())?;
    validate_policy_pin(&accepted.0, admission)?;
    Ok(match inputs {
        Some(vb_storage::RetainedRunInputs::Scalar(inputs)) => Some(inputs.source_digest()),
        Some(vb_storage::RetainedRunInputs::Owned(inputs)) => Some(inputs.source_digest()),
        None => None,
    })
}

fn validate_policy_pin(
    artifact: &AcceptedArtifact,
    admission: &vb_storage::JournalEvent,
) -> Result<(), String> {
    match admission {
        vb_storage::JournalEvent::RunAdmissionBudget {
            policy_pin: vb_storage::RunPolicyPin::Pinned(digest),
            ..
        } if *digest == artifact.policy_digest => Ok(()),
        _ => Err(vb_storage::JournalError::PayloadDigestMismatch.to_string()),
    }
}

fn matching_source(
    journal: &FjallJournal,
    digest: WorkflowDigest,
    workflow: &CompiledWorkflow,
) -> Result<Option<Vec<u8>>, String> {
    let Some(source) = journal
        .workflow_source(digest)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let compiled = vb_compile::compile_workflow(&source.source)
        .map_err(|errors| format!("retained baseline source cannot be compiled: {errors:?}"))?;
    if compiled != *workflow {
        return Err("retained baseline source does not match the accepted workflow".into());
    }
    Ok(Some(source.source))
}
