//! Behavioral tests for default artifact submission sequence counter.
//!
//! Validates that submit_artifact and submit_artifact_with_contracts
//! monotonically issue EventSeq values starting at 1, persist the
//! counter across reopen, and produce correct readback behavior.

use crate::FjallJournal;
use crate::admission::*;
use crate::error::JournalError;
use vb_core::RuntimePolicy;

use super::{minimal_workflow, temp_journal};

/// Asserts that an accepted artifact is readable back from the journal
/// with the correct digest and the full serialized artifact content matches.
fn assert_readback_verified(
    journal: &FjallJournal,
    artifact: &AcceptedArtifact,
) -> Result<(), String> {
    let record = journal
        .compiled_ir(artifact.artifact_digest)
        .map_err(|e| format!("compiled_ir read: {e}"))?
        .ok_or_else(|| "compiled_ir read returned None")?;

    let verified = verify_accepted_artifact_for_policy(&record, &policy)?;
    assert!(verified, "artifact verification should succeed");

    let stamped: AcceptedArtifact =
        postcard::from_bytes(record.ir.as_slice()).map_err(|e| format!("postcard decode: {e}"))?;
    assert_eq!(stamped, *artifact, "stamped artifact should roundtrip");

    Ok(())
}

/// First submission to a fresh journal gets sequence 1 for all policies.
#[test]
fn first_submission_is_seq_1_for_all_policies() -> Result<(), String> {
    let policies = [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ];

    for policy in policies {
        let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
        let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;
        let accepted = submit_artifact(&tj.journal, &workflow, policy)
            .map_err(|e| format!("submit_artifact policy={policy:?}: {e}"))?;

        assert_eq!(accepted.sequence, 1, "first submission should be seq 1");

        // Both digests must be non-zero and verified
        assert_ne!(
            accepted.artifact_digest.as_bytes(),
            [0u8; 32],
            "artifact_digest must be non-zero"
        );
        assert_ne!(
            accepted.source_digest.as_bytes(),
            [0u8; 32],
            "source_digest must be non-zero"
        );

        // Full readback verification
        assert_readback_verified(&tj.journal, &accepted)?;
    }

    Ok(())
}

/// Repeated identical workflows get strictly increasing sequences
/// and distinct digests, both readable.
#[test]
fn repeated_submissions_get_distinct_digests_and_both_readback() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

    let first = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("first submit: {e}"))?;
    assert_eq!(first.sequence, 1, "first submission should be seq 1");

    let second = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("second submit: {e}"))?;
    assert_eq!(second.sequence, 2, "second submission should be seq 2");

    // Digests must be different
    assert_ne!(
        first.artifact_digest, second.artifact_digest,
        "repeated submissions must have distinct artifact digests"
    );

    // Both must be readable
    assert_readback_verified(journal, &first)?;
    assert_readback_verified(journal, &second)?;

    Ok(())
}

/// Pure prepare stays at sequence 0; submit after prepare gets seq 1.
#[test]
fn pure_prepare_stays_seq_0_submit_after_gets_seq_1() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

    // Pure prepare does not issue a sequence
    let prepared = prepare_compiled_artifact(&workflow)
        .map_err(|e| format!("prepare_compiled_artifact: {e}"))?;
    assert_eq!(prepared.sequence, 0, "pure prepare should be seq 0");

    // Counter should still be 0
    let counter_bytes = journal
        .artifact_acceptance
        .get(b"last_issued")
        .map_err(|e| format!("counter read: {e}"))?;
    assert!(
        counter_bytes.is_none(),
        "counter should be absent after pure prepare"
    );

    // Now submit
    let accepted = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit_artifact: {e}"))?;
    assert_eq!(accepted.sequence, 1, "first submit should be seq 1");

    Ok(())
}

/// Counter persists across journal close/reopen.
#[test]
fn counter_persists_across_reopen() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal_path = tj.path.clone();
    let journal = &tj.journal;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

    // First submission gets seq 1
    let first = submit_artifact(journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("first submit: {e}"))?;
    assert_eq!(first.sequence, 1, "first submission should be seq 1");

    // Readback verification before close
    assert_readback_verified(journal, &first)?;

    // Close journal
    drop(tj);

    // Reopen journal
    let tj2 = temp_journal_at(&journal_path)?;
    let journal2 = &tj2.journal;

    // Old artifact should still be readable
    assert_readback_verified(journal2, &first)?;

    // Next submission should be seq 2
    let second = submit_artifact(journal2, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("second submit: {e}"))?;
    assert_eq!(second.sequence, 2, "second submission should be seq 2");

    Ok(())
}

/// Corrupted counter with length 7 causes MalformedKeyspaceRow with exact lengths.
#[test]
fn corrupted_counter_length_7_causes_malformed_row() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    // Insert a 7-byte counter value
    journal
        .artifact_acceptance
        .insert(b"last_issued", &[0, 0, 0, 0, 0, 0, 0])
        .map_err(|e| format!("insert 7-byte counter: {e}"))?;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

    let result = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed);

    match result {
        Err(JournalError::MalformedKeyspaceRow {
            keyspace,
            key,
            actual_len,
            expected_len,
        }) => {
            assert_eq!(keyspace, "artifact_acceptance");
            assert_eq!(key, b"last_issued");
            assert_eq!(actual_len, 7);
            assert_eq!(expected_len, 8);
        }
        Err(e) => {
            return Err(format!("expected MalformedKeyspaceRow, got {e}"));
        }
        Ok(_) => {
            return Err("expected MalformedKeyspaceRow, got success");
        }
    }

    // Counter bytes should be unchanged
    let counter = journal
        .artifact_acceptance
        .get(b"last_issued")
        .map_err(|e| format!("read counter: {e}"))?
        .ok_or_else(|| "counter should still exist")?;
    assert_eq!(counter.len(), 7, "counter should be unchanged");

    // No compiled_ir should have been written
    let compiled = journal
        .compiled_ir_iter()
        .map_err(|e| format!("iter: {e}"))?;
    let count = compiled.count();
    assert_eq!(count, 0, "no compiled_ir should have been written");

    Ok(())
}

/// Corrupted counter with length 9 causes MalformedKeyspaceRow with exact lengths.
#[test]
fn corrupted_counter_length_9_causes_malformed_row() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    // Insert a 9-byte counter value
    journal
        .artifact_acceptance
        .insert(b"last_issued", &[0, 0, 0, 0, 0, 0, 0, 0, 0])
        .map_err(|e| format!("insert 9-byte counter: {e}"))?;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

    let result = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed);

    match result {
        Err(JournalError::MalformedKeyspaceRow {
            keyspace,
            key,
            actual_len,
            expected_len,
        }) => {
            assert_eq!(keyspace, "artifact_acceptance");
            assert_eq!(key, b"last_issued");
            assert_eq!(actual_len, 9);
            assert_eq!(expected_len, 8);
        }
        Err(e) => {
            return Err(format!("expected MalformedKeyspaceRow, got {e}"));
        }
        Ok(_) => {
            return Err("expected MalformedKeyspaceRow, got success");
        }
    }

    // Counter bytes should be unchanged
    let counter = journal
        .artifact_acceptance
        .get(b"last_issued")
        .map_err(|e| format!("read counter: {e}"))?
        .ok_or_else(|| "counter should still exist")?;
    assert_eq!(counter.len(), 9, "counter should be unchanged");

    Ok(())
}

/// Counter at u64::MAX causes SequenceOverflow on next submission.
#[test]
fn counter_at_u64_max_causes_sequence_overflow() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    // Write counter at u64::MAX
    let max_bytes = u64::MAX.to_be_bytes();
    journal
        .artifact_acceptance
        .insert(b"last_issued", &max_bytes)
        .map_err(|e| format!("insert u64::MAX counter: {e}"))?;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

    let result = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed);

    match result {
        Err(JournalError::SequenceOverflow) => {}
        Err(e) => {
            return Err(format!("expected SequenceOverflow, got {e}"));
        }
        Ok(artifact) => {
            return Err(format!(
                "expected SequenceOverflow, got success with seq {}",
                artifact.sequence
            ));
        }
    }

    // Counter should be unchanged
    let counter = journal
        .artifact_acceptance
        .get(b"last_issued")
        .map_err(|e| format!("read counter: {e}"))?
        .ok_or_else(|| "counter should still exist")?;
    assert_eq!(counter, &max_bytes, "counter should be unchanged");

    // No compiled_ir should have been written
    let compiled = journal
        .compiled_ir_iter()
        .map_err(|e| format!("iter: {e}"))?;
    let count = compiled.count();
    assert_eq!(count, 0, "no compiled_ir should have been written");

    Ok(())
}

/// Opens a temporary journal at a specific path.
fn temp_journal_at(path: &std::path::Path) -> Result<TestJournal, JournalError> {
    let journal = FjallJournal::open(path, None)?;
    Ok(TestJournal {
        path: path.to_path_buf(),
        journal,
    })
}

/// Test journal wrapper that holds a path and journal reference.
struct TestJournal {
    path: std::path::PathBuf,
    journal: FjallJournal,
}
