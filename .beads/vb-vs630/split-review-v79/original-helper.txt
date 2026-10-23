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

/// Full binding verification for a CLI submission.
/// Validates exact typed inputs, digests, header, indices, artifact,
/// and acceptance events. Returns the actual accepted_at_seq stamped
/// on the artifact for the caller to collect.
pub(super) fn verify_concurrent_submission_binding(
    journal: &FjallJournal,
    run: RunId,
    value: i64,
) -> Result<u64, Box<dyn std::error::Error>> {
    let workflow = super::tests::workflow()?;
    let expected_input_digest =
        super::compute_input_digest(&[(SlotIdx::new(0), SlotValue::I64(value))])?;
    let expected_workflow_digest = workflow.digest();

    // Inputs record: typed values and digests
    let inputs = journal
        .run_inputs(run)?
        .ok_or(format!("inputs missing for run {run:?}"))?;
    assert_eq!(
        inputs.inputs(),
        &[(SlotIdx::new(0), SlotValue::I64(value))],
        "run {run:?}: typed input values mismatch"
    );
    assert_eq!(
        inputs.input_digest(),
        expected_input_digest,
        "run {run:?}: input digest mismatch"
    );
    assert_eq!(
        inputs.workflow_digest(),
        expected_workflow_digest,
        "run {run:?}: workflow digest mismatch"
    );

    // Workflow source: exact hash and bytes
    let source = journal
        .workflow_source(inputs.source_digest())?
        .ok_or(format!("source missing for run {run:?}"))?;
    let expected_source_bytes = b"cli source fixture".to_vec();
    assert_eq!(
        source.source, expected_source_bytes,
        "run {run:?}: source bytes mismatch"
    );
    let expected_source_digest =
        vb_core::WorkflowDigest::from_bytes(blake3::hash(&expected_source_bytes).into());
    assert_eq!(
        source.digest, expected_source_digest,
        "run {run:?}: source digest mismatch"
    );
    assert_eq!(
        inputs.source_digest(),
        expected_source_digest,
        "run {run:?}: inputs source digest mismatch"
    );

    // Compiled artifact and AcceptedArtifact decoding
    let record = journal
        .compiled_ir(inputs.artifact_digest())?
        .ok_or(format!("compiled missing for run {run:?}"))?;
    assert_eq!(
        record.digest,
        inputs.artifact_digest(),
        "run {run:?}: record digest mismatch"
    );
    let decoded: crate::AcceptedArtifact =
        postcard::from_bytes(&record.ir).map_err(|e| format!("run {run:?}: decode failed: {e}"))?;
    let actual_seq = decoded.accepted_at_seq.get();
    assert_eq!(
        decoded.digest,
        inputs.artifact_digest(),
        "run {run:?}: artifact digest mismatch"
    );
    assert_eq!(
        decoded.verification.digest,
        inputs.artifact_digest(),
        "run {run:?}: verification digest mismatch"
    );

    // Actual admission verification API restores the exact workflow
    let restored = crate::admission::verify_accepted_artifact(&decoded)
        .map_err(|e| format!("run {run:?}: verify failed: {e}"))?;
    assert_eq!(
        &restored, &workflow,
        "run {run:?}: restored workflow mismatch"
    );

    // RunHeaderRecord: all fields
    let header = journal
        .run_header(run)?
        .ok_or(format!("header missing for run {run:?}"))?;
    assert_eq!(header.run, run, "run {run:?}: header run mismatch");
    assert_eq!(
        header.workflow_id,
        vb_core::WorkflowId::new(0),
        "run {run:?}: header workflow_id mismatch"
    );
    assert_eq!(header.status, 0, "run {run:?}: header status mismatch");
    assert_eq!(
        header.accepted_at_ms, 1000,
        "run {run:?}: header accepted_at_ms mismatch"
    );
    assert_eq!(
        header.compiled_digest, expected_workflow_digest,
        "run {run:?}: header compiled_digest mismatch"
    );

    // Status index row via actual key API
    let status_key =
        crate::keys::index_status_key(crate::types::IndexStatusState::Submitted, 1000, run)?;
    let status_value = journal
        .index_status
        .get(&status_key)?
        .ok_or(format!("run {run:?}: status index row missing"))?;
    // put_status_index inserts Vec::<u8>::new() as the marker value
    assert_eq!(
        status_value.as_ref(),
        b"",
        "run {run:?}: status index value should be empty marker"
    );

    // Workflow index row via actual key API
    let workflow_key = crate::keys::index_workflow_key(vb_core::WorkflowId::new(0), run)?;
    let workflow_value = journal
        .index_workflow
        .get(&workflow_key)?
        .ok_or(format!("run {run:?}: workflow index row missing"))?;
    // put_workflow_index inserts Vec::<u8>::new() as the marker value
    assert_eq!(
        workflow_value.as_ref(),
        b"",
        "run {run:?}: workflow index value should be empty marker"
    );

    // Two acceptance events: RunAccepted (seq 0) and RunAdmission (seq 1)
    let events = journal.events_for_run(run)?;
    assert_eq!(events.len(), 2, "run {run:?}: expected 2 events");

    // Event 0: RunAccepted with run, seq 0, workflow digest
    let event0 = events
        .first()
        .ok_or(format!("run {run:?}: first event missing"))?;
    match event0 {
        crate::JournalEvent::RunAccepted {
            run: ev_run,
            seq: ev_seq,
            workflow: ev_workflow,
        } => {
            assert_eq!(*ev_run, run, "run {run:?}: event0 run mismatch");
            assert_eq!(ev_seq.get(), 0, "run {run:?}: event0 seq mismatch");
            assert_eq!(
                *ev_workflow, expected_workflow_digest,
                "run {run:?}: event0 workflow mismatch"
            );
        }
        other => return Err(format!("run {run:?}: event0 is {other:?}").into()),
    }

    // Event 1: RunAdmission with all expected fields
    let event1 = events
        .get(1)
        .ok_or(format!("run {run:?}: second event missing"))?;
    match event1 {
        crate::JournalEvent::RunAdmission {
            run: ev_run,
            seq: ev_seq,
            artifact_digest: ev_artifact,
            input_digest: ev_input_digest,
            ..
        } => {
            assert_eq!(*ev_run, run, "run {run:?}: event1 run mismatch");
            assert_eq!(ev_seq.get(), 1, "run {run:?}: event1 seq mismatch");
            assert_eq!(
                *ev_artifact,
                inputs.artifact_digest(),
                "run {run:?}: event1 artifact digest mismatch"
            );
            if let crate::RunInputBinding::Digest(ev_digest) = ev_input_digest {
                assert_eq!(
                    *ev_digest, expected_input_digest,
                    "run {run:?}: event1 input digest mismatch"
                );
            } else {
                return Err(format!("run {run:?}: event1 input digest not Digest").into());
            }
        }
        other => return Err(format!("run {run:?}: event1 is {other:?}").into()),
    }

    Ok(actual_seq)
}
