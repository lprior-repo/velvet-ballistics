//! Test support for CLI run submission issuance tests.
//! Provides fixture construction, submission builders, and counter inspection.

use super::RunInputsRecord;
use crate::{
    FjallJournal, JournalError, RunHeaderRecord, RunSubmission, WorkflowSourceRecord, admission,
};
use tempfile;
use vb_core::{RunId, RuntimePolicy, SlotIdx, SlotValue, WorkflowDigest, WorkflowId};

pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Build a test CLI submission for an explicit run with a specific policy and input value.
/// Retains the artifact, workflow, and source for post-commit verification.
pub(super) fn make_submission_for_run(
    policy: RuntimePolicy,
    value: i64,
    run: RunId,
) -> Result<RunSubmission, Box<dyn std::error::Error>> {
    let workflow = super::tests::workflow()?;
    let artifact = admission::prepare_compiled_artifact(&workflow, policy, &[])?;
    let source = b"cli source fixture".to_vec();
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let inputs = RunInputsRecord::new(
        run,
        artifact.digest,
        source_digest,
        &workflow,
        Box::from([(SlotIdx::new(0), SlotValue::I64(value))]),
    )?;
    Ok(RunSubmission::new(
        WorkflowSourceRecord {
            digest: source_digest,
            source,
        },
        artifact,
        inputs,
        RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 1000,
        },
    )?)
}

/// Build a test CLI submission with a specific policy and input value.
/// Retains the artifact, workflow, and source for post-commit verification.
pub(super) fn make_submission(
    policy: RuntimePolicy,
    value: i64,
) -> Result<RunSubmission, Box<dyn std::error::Error>> {
    make_submission_for_run(policy, value, RunId::new(100))
}

/// Build a test CLI submission for run 200.
pub(super) fn make_submission_run200(
    policy: RuntimePolicy,
    value: i64,
) -> Result<RunSubmission, Box<dyn std::error::Error>> {
    make_submission_for_run(policy, value, RunId::new(200))
}

/// Read the raw artifact acceptance counter from the journal.
pub(super) fn read_counter(journal: &FjallJournal) -> Result<Option<u64>, JournalError> {
    match journal.artifact_acceptance.get(b"last_issued")? {
        Some(value) => {
            if value.len() != 8 {
                return Err(JournalError::MalformedKeyspaceRow {
                    prefix: 0,
                    expected_len: 8,
                    actual_len: value.len(),
                });
            }
            let bytes: [u8; 8] =
                value
                    .as_ref()
                    .try_into()
                    .map_err(|_| JournalError::MalformedKeyspaceRow {
                        prefix: 0,
                        expected_len: 8,
                        actual_len: value.len(),
                    })?;
            Ok(Some(u64::from_be_bytes(bytes)))
        }
        None => Ok(None),
    }
}

/// Write a raw counter value to the journal for fault injection.
pub(super) fn write_counter(journal: &FjallJournal, value: u64) -> Result<(), JournalError> {
    let bytes = value.to_be_bytes();
    journal.artifact_acceptance.insert(b"last_issued", &bytes)?;
    Ok(())
}

/// Write a malformed counter row with arbitrary bytes.
pub(super) fn write_malformed_counter(
    journal: &FjallJournal,
    bytes: &[u8],
) -> Result<(), JournalError> {
    journal.artifact_acceptance.insert(b"last_issued", bytes)?;
    Ok(())
}

/// Open a fresh temporary journal for testing.
pub(super) fn open_test_journal() -> Result<(tempfile::TempDir, FjallJournal), JournalError> {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    Ok((dir, journal))
}

/// Verify the stamped artifact sequence number and digest consistency.
pub(super) fn verify_cli_submission_artifact_seq(
    journal: &FjallJournal,
    run: RunId,
    expected_seq: u64,
) -> TestResult {
    let inputs = journal.run_inputs(run)?.ok_or("run inputs not found")?;
    let record = journal
        .compiled_ir(inputs.artifact_digest())?
        .ok_or("compiled IR record not found")?;
    let decoded: crate::AcceptedArtifact = postcard::from_bytes(&record.ir)?;
    assert_eq!(decoded.accepted_at_seq.get(), expected_seq);
    assert_eq!(decoded.digest, inputs.artifact_digest());
    assert_eq!(decoded.verification.digest, inputs.artifact_digest());
    Ok(())
}

/// Verify that all journal keyspaces are empty (clean state).
pub(super) fn verify_empty_journal(journal: &FjallJournal) -> Result<(), JournalError> {
    assert!(
        journal.workflow_source.is_empty()?,
        "workflow_source not empty"
    );
    assert!(journal.compiled_ir.is_empty()?, "compiled_ir not empty");
    assert!(journal.run_header.is_empty()?, "run_header not empty");
    assert!(journal.run_inputs.is_empty()?, "run_inputs not empty");
    assert!(journal.events.is_empty()?, "events not empty");
    assert!(journal.index_status.is_empty()?, "index_status not empty");
    assert!(
        journal.index_workflow.is_empty()?,
        "index_workflow not empty"
    );
    Ok(())
}
