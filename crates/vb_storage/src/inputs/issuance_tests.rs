//! Behavioral tests for CLI run submission artifact issuance.
//! Verifies counter increment, accepted_at_seq stamping, digest rebinding,
//! policy durability, event sequencing, and mixed submission ordering.

use super::compute_input_digest;
use super::issuance_test_support::*;
use crate::admission;
use vb_core::{RuntimePolicy, SlotIdx, SlotValue};

/// All three CLI policies stamp accepted_at_seq=1 and advance the counter.
#[test]
fn cli_strict_submits_artifact_sequence_one() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let submission = make_submission(RuntimePolicy::Strict, 42)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    assert_eq!(read_counter(&journal)?, Some(1));
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(100), 1)?;
    Ok(())
}

#[test]
fn cli_journaled_submits_artifact_sequence_one() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let submission = make_submission(RuntimePolicy::Journaled, 42)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Journaled)?;
    assert_eq!(read_counter(&journal)?, Some(1));
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(100), 1)?;
    Ok(())
}

#[test]
fn cli_relaxed_submits_artifact_sequence_one() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let submission = make_submission(RuntimePolicy::Relaxed, 42)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Relaxed)?;
    assert_eq!(read_counter(&journal)?, Some(1));
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(100), 1)?;
    Ok(())
}

/// Stamped artifact digest matches retained inputs and RunAdmission event.
#[test]
fn cli_stamped_digest_matches_retained_and_admission_event() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let submission = make_submission(RuntimePolicy::Journaled, 73)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Journaled)?;

    let inputs = journal
        .run_inputs(vb_core::RunId::new(100))?
        .ok_or("run inputs not found")?;
    let events = journal.events_for_run(vb_core::RunId::new(100))?;
    assert_eq!(events.len(), 2);

    let admission_event = events
        .iter()
        .find(|e| matches!(e, crate::JournalEvent::RunAdmission { .. }));
    let run_accepted = events
        .iter()
        .find(|e| matches!(e, crate::JournalEvent::RunAccepted { .. }));
    assert!(run_accepted.is_some(), "expected RunAccepted event");
    assert!(admission_event.is_some(), "expected RunAdmission event");

    if let Some(crate::JournalEvent::RunAdmission {
        artifact_digest, ..
    }) = admission_event
    {
        assert_eq!(*artifact_digest, inputs.artifact_digest());
    }
    Ok(())
}

/// Input digest, typed values, source, workflow, and header remain unchanged.
#[test]
fn cli_submission_preserves_input_identity() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let submission = make_submission(RuntimePolicy::Journaled, 88)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Journaled)?;

    let inputs = journal
        .run_inputs(vb_core::RunId::new(100))?
        .ok_or("run inputs not found")?;

    // Compute expected input digest independently
    let expected_input_digest = compute_input_digest(&[(SlotIdx::new(0), SlotValue::I64(88))])?;
    assert_eq!(inputs.input_digest(), expected_input_digest);
    assert_eq!(inputs.inputs(), &[(SlotIdx::new(0), SlotValue::I64(88))]);

    // Workflow digest comes from the workflow itself
    let expected_workflow_digest = super::tests::workflow()?.digest();
    assert_eq!(inputs.workflow_digest(), expected_workflow_digest);

    // Source digest is computed from the fixture source bytes
    let expected_source_digest =
        vb_core::WorkflowDigest::from_bytes(blake3::hash(b"cli source fixture").into());
    assert_eq!(inputs.source_digest(), expected_source_digest);

    let source = journal
        .workflow_source(expected_source_digest)?
        .ok_or("source not found")?;
    assert_eq!(source.source, b"cli source fixture");

    let header = journal
        .run_header(vb_core::RunId::new(100))?
        .ok_or("header not found")?;
    assert_eq!(header.compiled_digest, expected_workflow_digest);
    Ok(())
}

/// Per-run event sequences: RunAccepted at 0, RunAdmission at 1.
#[test]
fn cli_journaled_events_have_correct_sequences() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let submission = make_submission(RuntimePolicy::Journaled, 1)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Journaled)?;

    let events = journal.events_for_run(vb_core::RunId::new(100))?;
    assert_eq!(events.len(), 2);

    let first = events.first().ok_or("first event not found")?;
    let second = events.get(1).ok_or("second event not found")?;

    match first {
        crate::JournalEvent::RunAccepted { seq, .. } => {
            assert_eq!(seq.get(), 0);
        }
        _ => return Err("expected RunAccepted as first event".into()),
    }

    match second {
        crate::JournalEvent::RunAdmission { seq, .. } => {
            assert_eq!(seq.get(), 1);
        }
        _ => return Err("expected RunAdmission as second event".into()),
    }
    Ok(())
}

/// Relaxed policy writes no acceptance events.
#[test]
fn cli_relaxed_writes_no_events() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let submission = make_submission(RuntimePolicy::Relaxed, 1)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Relaxed)?;

    let events = journal.events_for_run(vb_core::RunId::new(100))?;
    assert!(events.is_empty());
    Ok(())
}

/// Sequential CLI submissions increment the global counter.
#[test]
fn cli_sequential_submissions_increment_counter() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let sub1 = make_submission(RuntimePolicy::Strict, 1)?;
    let sub2 = make_submission_run200(RuntimePolicy::Journaled, 2)?;
    journal.commit_run_submission(&sub1, RuntimePolicy::Strict)?;
    journal.commit_run_submission(&sub2, RuntimePolicy::Journaled)?;
    assert_eq!(read_counter(&journal)?, Some(2));
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(100), 1)?;
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(200), 2)?;
    Ok(())
}

/// Mixed default and CLI submissions share the same global counter.
#[test]
fn mixed_default_and_cli_submissions_share_counter() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let workflow = super::tests::workflow()?;
    let default_artifact = admission::submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)?;
    assert_eq!(default_artifact.accepted_at_seq.get(), 1);

    let cli_submission = make_submission(RuntimePolicy::Strict, 99)?;
    journal.commit_run_submission(&cli_submission, RuntimePolicy::Strict)?;
    assert_eq!(read_counter(&journal)?, Some(2));
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(100), 2)?;

    let default2 = admission::submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)?;
    assert_eq!(default2.accepted_at_seq.get(), 3);
    assert_eq!(read_counter(&journal)?, Some(3));
    Ok(())
}

/// Pure preparation (no commit) does not advance the counter.
#[test]
fn cli_pure_prepare_does_not_advance_counter() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let _submission = make_submission(RuntimePolicy::Strict, 1)?;
    assert_eq!(read_counter(&journal)?, None);
    verify_empty_journal(&journal)?;
    Ok(())
}
