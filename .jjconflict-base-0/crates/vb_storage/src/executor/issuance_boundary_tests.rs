//! Issuance boundary tests for executor catalog packed admission.
//! Verifies counter length boundaries, u64::MAX overflow, and
//! overflow-then-retry behavior through the packed submission path.

use super::issuance_test_support::*;
use crate::JournalError;
use crate::executor::ExecutorError;
use vb_core::RuntimePolicy;

/// Counter stored as 7 bytes yields EXACT
/// ExecutorError::Artifact(JournalError::MalformedKeyspaceRow{prefix:0,
/// expected_len:8, actual_len:7}). Raw counter bytes unchanged and compiled_ir
/// empty before any submission.
#[test]
fn malformed_counter_seven_bytes() -> TestResult {
    let short = vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
    check_malformed_counter(&short, 7)
}

/// Counter stored as 9 bytes yields EXACT MalformedKeyspaceRow with
/// actual_len=9.
#[test]
fn malformed_counter_nine_bytes() -> TestResult {
    let long = vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99];
    check_malformed_counter(&long, 9)
}

fn check_malformed_counter(bytes: &[u8], expected_actual_len: usize) -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;
    journal.artifact_acceptance.insert(b"last_issued", bytes)?;

    let result = fixture.submit(&journal, RuntimePolicy::Relaxed);
    assert!(
        matches!(
            &result,
            Err(ExecutorError::Artifact(JournalError::MalformedKeyspaceRow {
                prefix,
                expected_len,
                actual_len
            })) if *prefix == 0 && *expected_len == 8 && *actual_len == expected_actual_len
        ),
        "expected MalformedKeyspaceRow(actual_len={expected_actual_len}), got: {result:?}"
    );

    let stored = journal
        .artifact_acceptance
        .get(b"last_issued")?
        .ok_or("counter row should still exist")?;
    assert_eq!(stored.as_ref(), bytes);
    assert!(journal.compiled_ir.iter().next().is_none());
    Ok(())
}

/// Counter at u64::MAX yields EXACT
/// ExecutorError::Artifact(JournalError::SequenceOverflow). Counter
/// remains at u64::MAX and compiled_ir is empty.
#[test]
fn counter_at_max_yields_sequence_overflow() -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;

    write_counter(&journal, u64::MAX)?;

    assert_sequence_overflow(&fixture, &journal)?;
    assert_eq!(read_counter(&journal)?, Some(u64::MAX));
    assert!(journal.compiled_ir.iter().next().is_none());
    Ok(())
}

fn assert_sequence_overflow(fixture: &PackedFixture, journal: &crate::FjallJournal) -> TestResult {
    let result = fixture.submit(journal, RuntimePolicy::Relaxed);
    assert!(
        matches!(
            &result,
            Err(ExecutorError::Artifact(JournalError::SequenceOverflow))
        ),
        "expected SequenceOverflow, got: {result:?}"
    );
    Ok(())
}

/// Counter at u64::MAX-1 allows one more submission at u64::MAX.
/// Subsequent submission fails with EXACT SequenceOverflow; counter
/// stays at u64::MAX and the prior artifact remains fully readable.
#[test]
fn counter_at_max_minus_one_allows_one_more() -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;

    let predecessor = u64::MAX.checked_sub(1).ok_or("max predecessor")?;
    write_counter(&journal, predecessor)?;

    let artifact = fixture.submit(&journal, RuntimePolicy::Relaxed)?;
    assert_eq!(artifact.accepted_at_seq.get(), u64::MAX);
    fixture.check(&journal, &artifact, RuntimePolicy::Relaxed)?;

    assert_eq!(read_counter(&journal)?, Some(u64::MAX));

    assert_sequence_overflow(&fixture, &journal)?;

    assert_eq!(read_counter(&journal)?, Some(u64::MAX));
    fixture.check(&journal, &artifact, RuntimePolicy::Relaxed)
}
