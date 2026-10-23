//! Failure tests for CLI run submission artifact issuance.
//! Verifies duplicate rejection, batch commit failure atomicity,
//! and counter state preservation on failure.

use super::issuance_snapshot_test_support::snapshot_journal;
use super::issuance_test_support::*;
use crate::JournalError;
use fjall;
use vb_core::RuntimePolicy;

/// Duplicate run submission fails with RunAlreadyExists before counter change.
#[test]
fn cli_duplicate_run_rejected_before_counter_publication() -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let submission = make_submission(RuntimePolicy::Strict, 1)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    assert_eq!(read_counter(&journal)?, Some(1));

    let before_inputs = journal.run_inputs(vb_core::RunId::new(100))?;
    let before_events = journal.events_for_run(vb_core::RunId::new(100))?;
    let before_snapshot = snapshot_journal(&journal)?;

    let result = journal.commit_run_submission(&submission, RuntimePolicy::Strict);
    assert!(
        matches!(result, Err(JournalError::RunAlreadyExists)),
        "expected RunAlreadyExists, got: {result:?}"
    );

    // Full journal state unchanged on duplicate: all typed records and index rows identical
    let after_snapshot = snapshot_journal(&journal)?;
    assert_eq!(after_snapshot, before_snapshot);

    // Old field-level assertions still hold
    assert_eq!(read_counter(&journal)?, Some(1));
    assert_eq!(journal.run_inputs(vb_core::RunId::new(100))?, before_inputs);
    assert_eq!(
        journal.events_for_run(vb_core::RunId::new(100))?,
        before_events
    );

    // Verify a fresh distinct run still commits correctly after duplicate
    let submission2 = make_submission_run200(RuntimePolicy::Strict, 2)?;
    journal.commit_run_submission(&submission2, RuntimePolicy::Strict)?;
    assert_eq!(read_counter(&journal)?, Some(2));
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(200), 2)?;
    Ok(())
}

/// Injected batch commit failure: all three policies leave counter and
/// all keyspaces unchanged before retry succeeds at seq 1.
#[test]
fn cli_injected_batch_commit_failure_strict() -> TestResult {
    check_batch_failure(RuntimePolicy::Strict)?;
    Ok(())
}

#[test]
fn cli_injected_batch_commit_failure_journaled() -> TestResult {
    check_batch_failure(RuntimePolicy::Journaled)?;
    Ok(())
}

#[test]
fn cli_injected_batch_commit_failure_relaxed() -> TestResult {
    check_batch_failure(RuntimePolicy::Relaxed)?;
    Ok(())
}

fn check_batch_failure(policy: RuntimePolicy) -> TestResult {
    let (_dir, journal) = open_test_journal()?;
    let submission = make_submission(policy, 42)?;
    journal.fail_next_batch_commit_for_test();
    let result = journal.commit_run_submission(&submission, policy);
    assert!(
        matches!(&result, Err(JournalError::Fjall(fjall::Error::Io(error))) if error.kind() == std::io::ErrorKind::Other),
        "expected Fjall I/O error with Other kind, got: {result:?}"
    );
    assert_eq!(read_counter(&journal)?, None);
    verify_empty_journal(&journal)?;

    journal.commit_run_submission(&submission, policy)?;
    assert_eq!(read_counter(&journal)?, Some(1));
    verify_cli_submission_artifact_seq(&journal, vb_core::RunId::new(100), 1)?;
    Ok(())
}
