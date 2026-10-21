use super::*;
use crate::{RunHeaderRecord, WorkflowSourceRecord};
use vb_core::RuntimePolicy;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn submission(
    policy: RuntimePolicy,
    value: i64,
) -> Result<RunSubmission, Box<dyn std::error::Error>> {
    let workflow = super::tests::workflow()?;
    let artifact = crate::admission::prepare_compiled_artifact(&workflow, policy, &[])?;
    let source = b"original source fixture".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let inputs = RunInputsRecord::new(
        RunId::new(7),
        artifact.digest,
        digest,
        &workflow,
        Box::from([(SlotIdx::new(0), SlotValue::I64(value))]),
    )?;
    Ok(RunSubmission::new(
        WorkflowSourceRecord { digest, source },
        artifact,
        inputs,
        RunHeaderRecord {
            run: RunId::new(7),
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?)
}

fn assert_empty(journal: &FjallJournal) -> TestResult {
    for keyspace in [
        &journal.workflow_source,
        &journal.compiled_ir,
        &journal.run_header,
        &journal.run_inputs,
        &journal.events,
        &journal.index_status,
        &journal.index_workflow,
    ] {
        assert!(keyspace.is_empty()?);
    }
    Ok(())
}

#[test]
fn injected_atomic_commit_failure_exposes_no_record_or_index_and_retry_succeeds() -> TestResult {
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        let submission = submission(policy, 73)?;
        journal.fail_next_batch_commit_for_test();
        assert!(matches!(
            journal.commit_run_submission(&submission, policy),
            Err(JournalError::Fjall(_))
        ));
        assert_empty(&journal)?;
        journal.commit_run_submission(&submission, policy)?;
        let retained = journal.run_inputs(RunId::new(7))?.ok_or("missing inputs")?;
        assert_eq!(retained.inputs(), [(SlotIdx::new(0), SlotValue::I64(73))]);
        assert!(journal.compiled_ir(retained.artifact_digest())?.is_some());
        assert!(journal.workflow_source(retained.source_digest())?.is_some());
        assert!(journal.run_header(retained.run())?.is_some());
        let expected_event_count = if policy == RuntimePolicy::Relaxed {
            0
        } else {
            2
        };
        assert_eq!(
            journal.events_for_run(retained.run())?.len(),
            expected_event_count
        );
        assert!(
            journal.has_workflow_index_entry(crate::keys::index_workflow_key(
                vb_core::WorkflowId::new(0),
                retained.run()
            )?)?
        );
        assert!(
            journal.has_status_index_entry(crate::keys::index_status_key(
                crate::IndexStatusState::Submitted,
                123,
                retained.run()
            )?)?
        );
    }
    Ok(())
}

#[test]
fn colliding_run_cannot_replace_retained_input_or_existing_admission() -> TestResult {
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission(policy, 19)?, policy)?;
        let before = journal.run_inputs(RunId::new(7))?;
        let events_before = journal.events_for_run(RunId::new(7))?;
        assert!(matches!(
            journal.commit_run_submission(&submission(policy, 99)?, policy),
            Err(JournalError::RunAlreadyExists)
        ));
        assert_eq!(journal.run_inputs(RunId::new(7))?, before);
        assert_eq!(journal.events_for_run(RunId::new(7))?, events_before);
    }
    Ok(())
}

#[test]
fn journaled_artifact_cannot_be_committed_as_strict_or_relaxed() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    let submission = submission(RuntimePolicy::Journaled, 1)?;
    assert!(matches!(
        journal.commit_run_submission(&submission, RuntimePolicy::Strict),
        Err(JournalError::ArtifactMalformed)
    ));
    assert_empty(&journal)?;
    assert!(matches!(
        journal.commit_run_submission(&submission, RuntimePolicy::Relaxed),
        Err(JournalError::ArtifactMalformed)
    ));
    assert_empty(&journal)?;
    Ok(())
}
