#![forbid(unsafe_code)]

use std::path::Path;
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, WorkflowDigest, WorkflowId};
use vb_storage::constants::{
    KEYSPACE_RUN_EVENT, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
};
use vb_storage::{AcceptedArtifact, EventSeq, FjallJournal, JournalEvent, RunPolicyPin};

#[path = "../../velvet_ballistics/src/semantic_diff_baseline.rs"]
mod baseline;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const SOURCE: &str = "version: velvet-ballistics/v1\nname: provenance\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: value, value: '42'}\n  - id: done\n    finish: {result: '$step.seed'}\n";

struct Case {
    directory: tempfile::TempDir,
    workflow: CompiledWorkflow,
    artifact: AcceptedArtifact,
    receipt: JournalEvent,
}

fn submission(journal: &FjallJournal, run: RunId, source: &str) -> TestResult {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    assert_eq!(artifact.accepted_at_seq, EventSeq::ZERO);
    let digest = WorkflowDigest::from_bytes(*blake3::hash(source.as_bytes()).as_bytes());
    let inputs =
        vb_storage::RunInputsRecord::new(run, artifact.digest, digest, &workflow, Box::default())?;
    let submission = vb_storage::RunSubmission::new(
        vb_storage::WorkflowSourceRecord {
            digest,
            source: source.as_bytes().to_vec(),
        },
        artifact,
        inputs,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 1,
        },
    )?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    Ok(())
}

fn selected(journal: &FjallJournal) -> TestResult<AcceptedArtifact> {
    journal
        .list_artifacts()?
        .into_iter()
        .map(|digest| {
            let bytes = journal.compiled_ir(digest)?.ok_or("artifact")?;
            Ok(postcard::from_bytes::<AcceptedArtifact>(&bytes.ir)?)
        })
        .collect::<TestResult<Vec<_>>>()?
        .into_iter()
        .max_by_key(|artifact| artifact.accepted_at_seq)
        .ok_or("issued artifact".into())
}

fn case() -> TestResult<Case> {
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    submission(&journal, RunId::new(10), SOURCE)?;
    let artifact = selected(&journal)?;
    assert!(artifact.accepted_at_seq > EventSeq::ZERO);
    let receipt = journal
        .events_for_run(RunId::new(10))?
        .into_iter()
        .find(|event| matches!(event, JournalEvent::RunAdmissionBudget { .. }))
        .ok_or("receipt")?;
    let workflow = vb_storage::admission::verify_accepted_artifact(&artifact)?;
    drop(journal);
    Ok(Case {
        directory,
        workflow,
        artifact,
        receipt,
    })
}

fn rewrite(case: &Case, event: &JournalEvent) -> TestResult {
    let bytes = vb_storage::encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(vb_storage::journal::parse_event(&bytes)?, *event);
    let database = fjall::Database::builder(case.directory.path())
        .cache_size(1 << 20)
        .open()?;
    let space = database.keyspace(KEYSPACE_RUN_EVENT, fjall::KeyspaceCreateOptions::default)?;
    let key = vb_storage::keys::run_event_key(event.run_id(), event.seq())?;
    assert_eq!(
        key.first().copied(),
        Some(vb_storage::constants::PREFIX_RUN_EVENT)
    );
    space.insert(key, bytes)?;
    database.persist(fjall::PersistMode::SyncAll)?;
    Ok(())
}

type Rows = Vec<(String, Vec<(Vec<u8>, Vec<u8>)>)>;
fn snapshot(path: &Path) -> TestResult<Rows> {
    let database = fjall::Database::builder(path).cache_size(1 << 20).open()?;
    let mut result = Vec::new();
    for name in FjallJournal::declared_keyspaces() {
        let space = database.keyspace(name, fjall::KeyspaceCreateOptions::default)?;
        let mut rows = Vec::new();
        for entry in space.iter() {
            let (key, value) = entry.into_inner()?;
            rows.push((key.to_vec(), value.to_vec()));
        }
        result.push((name.to_string(), rows));
    }
    Ok(result)
}

fn load(case: &Case) -> Result<baseline::Baseline, String> {
    baseline::load(Some(case.directory.path()), case.workflow.name())
}

fn assert_baseline(case: &Case, actual: &baseline::Baseline, source: Option<&[u8]>) {
    assert_eq!(actual.workflow, case.workflow);
    assert_eq!(actual.source.as_deref(), source);
    assert_eq!(
        actual.label,
        format!(
            "{}:accepted_at_seq={}",
            case.directory.path().display(),
            case.artifact.accepted_at_seq.get()
        )
    );
    assert_eq!(
        actual.action_contracts.as_deref(),
        Some(case.artifact.action_contracts.as_ref())
    );
}

fn assert_selected_receipt_refused(case: &Case, event: &JournalEvent) -> TestResult {
    rewrite(case, event)?;
    let before = snapshot(case.directory.path())?;
    match load(case) {
        Ok(_) => return Err("malformed selected receipt supplied source provenance".into()),
        Err(error) => assert_eq!(
            error,
            vb_storage::JournalError::PayloadDigestMismatch.to_string()
        ),
    }
    assert_eq!(snapshot(case.directory.path())?, before);
    Ok(())
}

#[test]
fn current_submission_receipt_proves_all_baseline_fields_without_writes() -> TestResult {
    let case = case()?;
    let before = snapshot(case.directory.path())?;
    assert_baseline(&case, &load(&case)?, Some(SOURCE.as_bytes()));
    assert_eq!(snapshot(case.directory.path())?, before);
    Ok(())
}

#[test]
fn mismatched_input_digest_cannot_supply_selected_source_provenance() -> TestResult {
    let case = case()?;
    let mut event = case.receipt.clone();
    if let JournalEvent::RunAdmissionBudget { input_digest, .. } = &mut event {
        *input_digest = vb_storage::RunInputBinding::Digest(WorkflowDigest::from_bytes([9; 32]));
    }
    assert_selected_receipt_refused(&case, &event)
}

#[test]
fn unverified_policy_pin_cannot_supply_selected_source_provenance() -> TestResult {
    let case = case()?;
    let mut event = case.receipt.clone();
    if let JournalEvent::RunAdmissionBudget { policy_pin, .. } = &mut event {
        *policy_pin = RunPolicyPin::Unverified;
    }
    assert_selected_receipt_refused(&case, &event)
}

#[test]
fn wrong_pinned_digest_cannot_supply_selected_source_provenance() -> TestResult {
    let case = case()?;
    let mut event = case.receipt.clone();
    let wrong = WorkflowDigest::from_bytes([8; 32]);
    assert_ne!(wrong, case.artifact.policy_digest);
    if let JournalEvent::RunAdmissionBudget { policy_pin, .. } = &mut event {
        *policy_pin = RunPolicyPin::Pinned(wrong);
    }
    assert_selected_receipt_refused(&case, &event)
}

#[test]
fn unbudgeted_admission_leaves_text_unavailable_and_preserves_artifact_semantics() -> TestResult {
    let case = case()?;
    let event = match &case.receipt {
        JournalEvent::RunAdmissionBudget {
            run,
            seq,
            artifact_digest,
            input_digest,
            policy_pin,
            granted_capabilities,
            policy,
            ..
        } => JournalEvent::RunAdmission {
            run: *run,
            seq: *seq,
            artifact_digest: *artifact_digest,
            input_digest: *input_digest,
            policy_pin: *policy_pin,
            granted_capabilities: granted_capabilities.clone(),
            policy: *policy,
        },
        _ => return Err("expected budgeted receipt".into()),
    };
    rewrite(&case, &event)?;
    let before = snapshot(case.directory.path())?;
    assert_baseline(&case, &load(&case)?, None);
    assert_eq!(snapshot(case.directory.path())?, before);
    Ok(())
}

#[test]
fn same_workflow_other_artifact_is_ignored_and_later_matching_header_is_used() -> TestResult {
    let mut case = case()?;
    let journal = FjallJournal::open(case.directory.path(), None)?;
    case.artifact = vb_storage::submit_artifact(&journal, &case.workflow, RuntimePolicy::Strict)?;
    drop(journal);
    let before = snapshot(case.directory.path())?;
    assert_baseline(&case, &load(&case)?, None);
    assert_eq!(snapshot(case.directory.path())?, before);
    let source = format!("# selected current producer\n{SOURCE}");
    let journal = FjallJournal::open(case.directory.path(), None)?;
    submission(&journal, RunId::new(11), &source)?;
    case.artifact = selected(&journal)?;
    drop(journal);
    let before = snapshot(case.directory.path())?;
    assert_baseline(&case, &load(&case)?, Some(source.as_bytes()));
    assert_eq!(snapshot(case.directory.path())?, before);
    Ok(())
}
