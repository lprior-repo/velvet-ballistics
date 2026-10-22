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

    let computed =
        accepted_artifact_digest(artifact).map_err(|e| format!("accepted_artifact_digest: {e}"))?;
    assert_eq!(
        computed, artifact.digest,
        "readback digest must match artifact digest"
    );

    assert_eq!(
        artifact.verification.digest, artifact.digest,
        "verification proof digest must match artifact digest"
    );

    let compiled = verify_accepted_artifact_for_policy(artifact, policy)
        .map_err(|e| format!("verify_accepted_artifact_for_policy: {e}"))?;
    assert!(
        compiled.node_count() > 0,
        "compiled workflow must have nodes"
    );

    let stamped: AcceptedArtifact =
        postcard::from_bytes(&record.ir).map_err(|e| format!("postcard decode: {e}"))?;
    assert_eq!(stamped, *artifact, "stamped artifact should roundtrip");

    Ok(())
}

fn assert_first_seq(accepted: &AcceptedArtifact) {
    assert_eq!(
        accepted.accepted_at_seq.get(),
        1,
        "first submission should be seq 1"
    );
}

fn assert_second_seq(accepted: &AcceptedArtifact) {
    assert_eq!(
        accepted.accepted_at_seq.get(),
        2,
        "second submission should be seq 2"
    );
}

fn assert_nonzero_digest(accepted: &AcceptedArtifact) {
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

        assert_first_seq(&accepted);
        assert_nonzero_digest(&accepted);
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
    assert_first_seq(&first);

    let second = submit_artifact(journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("second submit: {e}"))?;
    assert_second_seq(&second);

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
    assert_first_seq(&accepted);

    Ok(())
}

#[test]
fn counter_persists_across_reopen() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let journal_path = dir.path().to_path_buf();
    let workflow = minimal_workflow().map_err(|e| format!("minimal_workflow: {e}"))?;

    let first;
    {
        let journal =
            FjallJournal::open(&journal_path, None).map_err(|e| format!("journal open: {e}"))?;
        first = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("first submit: {e}"))?;
        assert_first_seq(&first);
    }

    {
        let journal =
            FjallJournal::open(&journal_path, None).map_err(|e| format!("journal reopen: {e}"))?;

        let counter_bytes = journal
            .artifact_acceptance
            .get(b"last_issued")
            .map_err(|e| format!("counter read: {e}"))?
            .ok_or_else(|| "counter should be present after first submit")?;
        assert_eq!(
            counter_bytes,
            [0, 0, 0, 0, 0, 0, 0, 1],
            "counter should be exactly 1 (u64 BE)"
        );

        assert_readback_verified(&journal, &first, RuntimePolicy::Strict)?;

        let second = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("second submit: {e}"))?;
        assert_second_seq(&second);
        assert_readback_verified(&journal, &second, RuntimePolicy::Strict)?;
    }

    Ok(())
}

#[path = "acceptance_sequence_boundary_tests.rs"]
mod boundary_tests;
