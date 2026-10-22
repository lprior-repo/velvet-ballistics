//! Boundary tests for sequence counter corruption and overflow conditions.

use crate::admission::*;
use crate::error::JournalError;
use vb_core::RuntimePolicy;

use super::{minimal_workflow, temp_journal};

#[test]
fn corrupted_counter_length_7_causes_malformed_row() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    journal
        .artifact_acceptance
        .insert(b"last_issued", &[0, 0, 0, 0, 0, 0, 0])
        .map_err(|e| format!("insert 7-byte counter: {e}"))?;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;
    let result = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed);

    let matches_malformed = matches!(
        result,
        Err(JournalError::MalformedKeyspaceRow {
            prefix: 0,
            expected_len: 8,
            actual_len: 7
        })
    );
    assert!(
        matches_malformed,
        "expected MalformedKeyspaceRow actual=7, got {result:?}"
    );

    let counter = journal
        .artifact_acceptance
        .get(b"last_issued")
        .map_err(|e| format!("read counter: {e}"))?
        .ok_or_else(|| "counter should still exist")?;
    assert_eq!(
        counter,
        &[0, 0, 0, 0, 0, 0, 0],
        "counter should be unchanged"
    );
    assert!(
        journal.compiled_ir.iter().next().is_none(),
        "no compiled_ir written"
    );

    Ok(())
}

#[test]
fn corrupted_counter_length_9_causes_malformed_row() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    journal
        .artifact_acceptance
        .insert(b"last_issued", &[0, 0, 0, 0, 0, 0, 0, 0, 0])
        .map_err(|e| format!("insert 9-byte counter: {e}"))?;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;
    let result = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed);

    let matches_malformed = matches!(
        result,
        Err(JournalError::MalformedKeyspaceRow {
            prefix: 0,
            expected_len: 8,
            actual_len: 9
        })
    );
    assert!(
        matches_malformed,
        "expected MalformedKeyspaceRow actual=9, got {result:?}"
    );

    let counter = journal
        .artifact_acceptance
        .get(b"last_issued")
        .map_err(|e| format!("read counter: {e}"))?
        .ok_or_else(|| "counter should still exist")?;
    assert_eq!(
        counter,
        &[0, 0, 0, 0, 0, 0, 0, 0, 0],
        "counter should be unchanged"
    );
    assert!(
        journal.compiled_ir.iter().next().is_none(),
        "no compiled_ir written"
    );

    Ok(())
}

#[test]
fn counter_at_u64_max_causes_sequence_overflow() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    let max_bytes = u64::MAX.to_be_bytes();
    journal
        .artifact_acceptance
        .insert(b"last_issued", &max_bytes)
        .map_err(|e| format!("insert u64::MAX counter: {e}"))?;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;
    let result = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed);

    assert!(
        matches!(result, Err(JournalError::SequenceOverflow)),
        "expected SequenceOverflow, got {result:?}"
    );

    let counter = journal
        .artifact_acceptance
        .get(b"last_issued")
        .map_err(|e| format!("read counter: {e}"))?
        .ok_or_else(|| "counter should still exist")?;
    assert_eq!(counter, &max_bytes, "counter should be unchanged");
    assert!(
        journal.compiled_ir.iter().next().is_none(),
        "no compiled_ir written"
    );

    Ok(())
}
