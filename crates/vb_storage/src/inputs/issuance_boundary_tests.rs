//! Boundary tests for CLI run submission artifact issuance.
//! Verifies malformed counter, u64::MAX overflow, close/reopen state
//! preservation, and bounded concurrent submission.

use super::compute_input_digest;
use super::issuance_test_support::*;
use crate::{FjallJournal, JournalError};
use std::sync::Arc;
use vb_core::{RunId, RuntimePolicy, SlotIdx, SlotValue};

/// 7-byte counter yields exact MalformedKeyspaceRow error.
#[test]
fn cli_malformed_counter_seven_bytes() -> TestResult {
    check_malformed_counter(&[0, 1, 2, 3, 4, 5, 6], 7)?;
    Ok(())
}

/// 9-byte counter yields exact MalformedKeyspaceRow error.
#[test]
fn cli_malformed_counter_nine_bytes() -> TestResult {
    check_malformed_counter(&[0, 1, 2, 3, 4, 5, 6, 7, 8], 9)?;
    Ok(())
}

fn check_malformed_counter(bytes: &[u8], expected_actual_len: usize) -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    write_malformed_counter(&journal, bytes)?;
    // Verify the raw malformed counter is stored as-is (don't call production read_counter,
    // which returns None on length mismatch and would exit early)
    let before = journal
        .artifact_acceptance
        .get(b"last_issued")?
        .ok_or("counter row missing")?;
    assert_eq!(before.as_ref(), bytes);

    let submission = make_submission(RuntimePolicy::Strict, 99)?;
    let result = journal.commit_run_submission(&submission, RuntimePolicy::Strict);
    match result {
        Err(JournalError::MalformedKeyspaceRow {
            prefix,
            expected_len,
            actual_len,
        }) => {
            assert_eq!(prefix, 0);
            assert_eq!(expected_len, 8);
            assert_eq!(actual_len, expected_actual_len);
        }
        other => return Err(format!("expected MalformedKeyspaceRow, got: {other:?}").into()),
    }
    // Verify counter unchanged and journal otherwise empty
    let after = journal
        .artifact_acceptance
        .get(b"last_issued")?
        .ok_or("counter row missing")?;
    assert_eq!(after.as_ref(), before.as_ref());
    verify_empty_journal(&journal)?;
    Ok(())
}

/// Counter at u64::MAX yields exact SequenceOverflow error.
#[test]
fn cli_counter_at_max_yields_sequence_overflow() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    write_counter(&journal, u64::MAX)?;

    let submission = make_submission(RuntimePolicy::Strict, 1)?;
    let result = journal.commit_run_submission(&submission, RuntimePolicy::Strict);
    match result {
        Err(JournalError::SequenceOverflow) => {}
        other => return Err(format!("expected SequenceOverflow, got: {other:?}").into()),
    }
    assert_eq!(read_counter(&journal)?, Some(u64::MAX));
    Ok(())
}

/// Counter at u64::MAX-1 allows one more submission at u64::MAX,
/// then subsequent submission fails with SequenceOverflow.
#[test]
fn cli_counter_at_max_minus_one_allows_one_more() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    write_counter(&journal, u64::MAX.checked_sub(1).ok_or("max predecessor")?)?;

    let submission = make_submission(RuntimePolicy::Strict, 1)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    assert_eq!(read_counter(&journal)?, Some(u64::MAX));
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(100), u64::MAX)?;
    // Snapshot the prior successful run's artifacts before the overflow submission
    let first_inputs = journal
        .run_inputs(RunId::new(100))?
        .ok_or("first run inputs not found")?;
    let first_artifact = journal
        .compiled_ir(first_inputs.artifact_digest())?
        .ok_or("first run compiled IR not found")?;

    let submission2 = make_submission_run200(RuntimePolicy::Strict, 2)?;
    let result = journal.commit_run_submission(&submission2, RuntimePolicy::Strict);
    match result {
        Err(JournalError::SequenceOverflow) => {}
        other => return Err(format!("expected SequenceOverflow, got: {other:?}").into()),
    }
    // Verify prior successful run's artifacts are preserved and counter is at MAX
    assert_eq!(
        journal.run_inputs(RunId::new(100))?.as_ref(),
        Some(&first_inputs)
    );
    assert_eq!(
        journal.compiled_ir(first_artifact.digest)?.as_ref(),
        Some(&first_artifact)
    );
    assert!(journal.run_inputs(RunId::new(200))?.is_none());
    assert!(journal.run_header(RunId::new(200))?.is_none());
    Ok(())
}

/// Journal closed and reopened on same path preserves counter and all
/// typed submission records; next submission advances counter correctly.
#[test]
fn cli_same_path_reopen_preserves_state_and_advances() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut journal = FjallJournal::open(dir.path(), None)?;
    let submission = make_submission(RuntimePolicy::Strict, 77)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;

    let expected_input_digest = compute_input_digest(&[(SlotIdx::new(0), SlotValue::I64(77))])?;
    let expected_workflow_digest = super::tests::workflow()?.digest();

    // Read back the retained state before dropping
    let inputs = journal
        .run_inputs(vb_core::RunId::new(100))?
        .ok_or("inputs not found before close")?;
    assert_eq!(inputs.input_digest(), expected_input_digest);
    assert_eq!(inputs.workflow_digest(), expected_workflow_digest);
    let compiled = journal
        .compiled_ir(inputs.artifact_digest())?
        .ok_or("compiled not found before close")?;
    let source = journal
        .workflow_source(inputs.source_digest())?
        .ok_or("source not found before close")?;
    let header = journal
        .run_header(vb_core::RunId::new(100))?
        .ok_or("header not found before close")?;
    let events = journal.events_for_run(vb_core::RunId::new(100))?;

    journal.close()?;
    drop(journal);

    // Reopen the same path
    let mut journal = FjallJournal::open(dir.path(), None)?;
    assert_eq!(read_counter(&journal)?, Some(1));

    // Verify retained state survives reopen
    let reopened_inputs = journal
        .run_inputs(vb_core::RunId::new(100))?
        .ok_or("inputs not found after reopen")?;
    assert_eq!(reopened_inputs.input_digest(), inputs.input_digest());
    assert_eq!(reopened_inputs.workflow_digest(), inputs.workflow_digest());
    assert_eq!(reopened_inputs.artifact_digest(), inputs.artifact_digest());
    assert_eq!(
        reopened_inputs.inputs(),
        &[(SlotIdx::new(0), SlotValue::I64(77))]
    );

    let reopened_compiled = journal
        .compiled_ir(reopened_inputs.artifact_digest())?
        .ok_or("compiled not found after reopen")?;
    assert_eq!(reopened_compiled.ir, compiled.ir);

    let reopened_source = journal
        .workflow_source(reopened_inputs.source_digest())?
        .ok_or("source not found after reopen")?;
    assert_eq!(reopened_source.source, source.source);

    let reopened_header = journal
        .run_header(vb_core::RunId::new(100))?
        .ok_or("header not found after reopen")?;
    assert_eq!(reopened_header.compiled_digest, header.compiled_digest);

    let reopened_events = journal.events_for_run(vb_core::RunId::new(100))?;
    assert_eq!(reopened_events.len(), events.len());

    // Verify old retained artifact still passes verification after reopen
    let reopened_decoded: crate::AcceptedArtifact = postcard::from_bytes(&reopened_compiled.ir)?;
    assert_eq!(reopened_decoded.digest, reopened_inputs.artifact_digest());
    assert_eq!(
        reopened_decoded.verification.digest,
        reopened_inputs.artifact_digest()
    );
    assert_eq!(reopened_decoded.accepted_at_seq.get(), 1);

    // Verify the retained record itself
    let accepted_event = reopened_events
        .iter()
        .find(|e| matches!(e, crate::JournalEvent::RunAdmission { .. }));
    if let Some(crate::JournalEvent::RunAdmission {
        artifact_digest, ..
    }) = accepted_event
    {
        assert_eq!(*artifact_digest, reopened_inputs.artifact_digest());
    }

    // Submit a second run after reopen
    let submission2 =
        make_submission_for_run(RuntimePolicy::Journaled, 88, vb_core::RunId::new(200))?;
    journal.commit_run_submission(&submission2, RuntimePolicy::Journaled)?;
    assert_eq!(read_counter(&journal)?, Some(2));
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(200), 2)?;

    journal.close()?;
    Ok(())
}

/// Bounded concurrent distinct-run CLI submissions produce distinct
/// sequences with no collision.
#[test]
fn cli_bounded_concurrent_submissions() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let journal = Arc::new(journal);

    let run_configs = [
        (vb_core::RunId::new(100), 10),
        (vb_core::RunId::new(101), 11),
        (vb_core::RunId::new(102), 12),
        (vb_core::RunId::new(103), 13),
    ];

    let mut handles = Vec::new();
    for (run, value) in run_configs {
        let journal_clone = Arc::clone(&journal);
        let handle = std::thread::spawn(move || {
            let submission = make_submission_for_run(RuntimePolicy::Strict, value, run)
                .map_err(|e| format!("make submission failed: {e}"))?;
            journal_clone
                .commit_run_submission(&submission, RuntimePolicy::Strict)
                .map_err(|e| format!("commit failed: {e}"))
        });
        handles.push(handle);
    }

    for handle in handles.into_iter() {
        handle
            .join()
            .map_err(|_| "submission thread panicked")?
            .map_err(|e| format!("concurrent submission failed: {e}"))?;
    }

    assert_eq!(read_counter(&journal)?, Some(4));

    // Verify each run got a distinct sequence
    let mut seqs = Vec::new();
    for (run, _) in run_configs {
        let inputs = journal.run_inputs(run)?.ok_or("inputs not found")?;
        let compiled = journal
            .compiled_ir(inputs.artifact_digest())?
            .ok_or("compiled not found")?;
        let decoded: crate::AcceptedArtifact = postcard::from_bytes(&compiled.ir)?;
        seqs.push(decoded.accepted_at_seq.get());
    }

    seqs.sort();
    assert_eq!(seqs, [1, 2, 3, 4]);
    Ok(())
}
