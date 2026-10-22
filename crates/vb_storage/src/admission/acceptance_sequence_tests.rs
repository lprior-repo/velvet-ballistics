//! Behavioral tests for default artifact submission sequence counter.

use crate::FjallJournal;
use crate::admission::*;
use crate::error::JournalError;
use vb_core::RuntimePolicy;

use super::{minimal_workflow, temp_journal};

fn assert_readback_verified(
    journal: &FjallJournal,
    artifact: &AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<(), String> {
    let record = journal
        .compiled_ir(artifact.digest)
        .map_err(|e| format!("compiled_ir read: {e}"))?
        .ok_or_else(|| "compiled_ir read returned None")?;

    assert_eq!(
        digest_artifact(&artifact),
        artifact.digest,
        "readback digest must match accepted artifact digest"
    );

    assert_eq!(
        artifact.verification.digest, artifact.digest,
        "verification proof digest must match artifact digest"
    );

    let compiled = verify_accepted_artifact_for_policy(artifact, policy)
        .map_err(|e| format!("verify_accepted_artifact_for_policy: {e}"))?;
    let _ = compiled;

    let stamped: AcceptedArtifact =
        postcard::from_bytes(&record.ir).map_err(|e| format!("postcard decode: {e}"))?;
    assert_eq!(stamped, *artifact, "stamped artifact should roundtrip");

    Ok(())
}

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

        assert_eq!(
            accepted.accepted_at_seq.get(),
            1,
            "first submission should be seq 1"
        );

        assert_ne!(
            accepted.digest.as_bytes(),
            [0u8; 32],
            "digest must be non-zero"
        );
        assert_ne!(
            accepted.source_digest.as_bytes(),
            [0u8; 32],
            "source_digest must be non-zero"
        );

        assert_readback_verified(&tj.journal, &accepted, policy)?;
    }

    Ok(())
}

#[test]
fn repeated_submissions_get_distinct_digests_and_both_readback() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

    let first = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("first submit: {e}"))?;
    assert_eq!(
        first.accepted_at_seq.get(),
        1,
        "first submission should be seq 1"
    );

    let second = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("second submit: {e}"))?;
    assert_eq!(
        second.accepted_at_seq.get(),
        2,
        "second submission should be seq 2"
    );

    assert_ne!(
        first.digest, second.digest,
        "repeated submissions must have distinct artifact digests"
    );

    assert_readback_verified(journal, &first, RuntimePolicy::Relaxed)?;
    assert_readback_verified(journal, &second, RuntimePolicy::Relaxed)?;

    Ok(())
}

#[test]
fn pure_prepare_stays_seq_0_submit_after_gets_seq_1() -> Result<(), String> {
    let tj = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let journal = &tj.journal;

    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

    let prepared = prepare_compiled_artifact(&workflow, RuntimePolicy::Relaxed, &[])
        .map_err(|e| format!("prepare_compiled_artifact: {e}"))?;
    assert_eq!(
        prepared.accepted_at_seq.get(),
        0,
        "pure prepare should be seq 0"
    );

    let counter_bytes = journal
        .artifact_acceptance
        .get(b"last_issued")
        .map_err(|e| format!("counter read: {e}"))?;
    assert!(
        counter_bytes.is_none(),
        "counter should be absent after pure prepare"
    );

    let accepted = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit_artifact: {e}"))?;
    assert_eq!(
        accepted.accepted_at_seq.get(),
        1,
        "first submit should be seq 1"
    );

    Ok(())
}

#[test]
fn counter_persists_across_reopen() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let journal_path = dir.path().to_path_buf();

    {
        let journal =
            FjallJournal::open(&journal_path, None).map_err(|e| format!("journal open: {e}"))?;

        let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

        let first = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("first submit: {e}"))?;
        assert_eq!(
            first.accepted_at_seq.get(),
            1,
            "first submission should be seq 1"
        );

        assert_readback_verified(&journal, &first, RuntimePolicy::Strict)?;
    }

    {
        let journal =
            FjallJournal::open(&journal_path, None).map_err(|e| format!("journal reopen: {e}"))?;

        let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

        let second = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("second submit: {e}"))?;
        assert_eq!(
            second.accepted_at_seq.get(),
            2,
            "second submission should be seq 2"
        );
    }

    Ok(())
}

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
        "expected MalformedKeyspaceRow with prefix=0 expected=8 actual=7, got {result:?}"
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
        "no compiled_ir should have been written"
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
        "expected MalformedKeyspaceRow with prefix=0 expected=8 actual=9, got {result:?}"
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

    let matches_overflow = matches!(result, Err(JournalError::SequenceOverflow));
    assert!(
        matches_overflow,
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
        "no compiled_ir should have been written"
    );

    Ok(())
}
