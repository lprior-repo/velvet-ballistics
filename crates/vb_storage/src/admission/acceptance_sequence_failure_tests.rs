//! Failure-mode tests for the artifact acceptance sequence counter and issuance path.
//!
//! These tests cover strict durability failures, batch commit failures,
//! compiled-IR readback failures, invalid workflow digest rejection, and
//! counter overflow at u64::MAX.

use crate::admission::*;
use crate::error::JournalError;
use vb_core::RuntimePolicy;

use super::{minimal_workflow, temp_journal};

/// Strict durability failure is precommit; counter must remain at 0.
#[test]
fn strict_persist_failure_is_precommit_counter_unchanged() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    // Inject strict persist failure.
    journal.fail_next_persist_for_test();

    // Submission should fail with strict durability error.
    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);
    match result {
        Err(JournalError::StrictDurabilityFailed) => {}
        other => {
            return Err(format!("expected StrictDurabilityFailed, got: {other:?}"));
        }
    }

    // Counter must still be 0 — no artifact was issued. Verify by submitting
    // the same workflow again; it must succeed at seq 1.
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("retry submission failed: {e}"))?;
    if artifact.accepted_at_seq.get() != 1 {
        return Err(format!(
            "retry artifact should have seq 1, got {}",
            artifact.accepted_at_seq.get()
        ));
    }

    Ok(())
}

/// Batch commit failure is precommit; counter and artifact keyspace must remain unchanged.
#[test]
fn batch_commit_failure_is_precommit_no_state_change() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    // Inject batch commit failure.
    journal.fail_next_batch_commit_for_test();

    // Submission should fail with Fjall error.
    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);
    match result {
        Err(JournalError::Fjall(_)) => {}
        other => {
            return Err(format!("expected Fjall error, got: {other:?}"));
        }
    }

    // Counter must still be 0. Retry with same workflow; must succeed at seq 1.
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("retry submission failed: {e}"))?;
    if artifact.accepted_at_seq.get() != 1 {
        return Err(format!(
            "retry artifact should have seq 1, got {}",
            artifact.accepted_at_seq.get()
        ));
    }

    Ok(())
}

/// Compiled-IR readback failure occurs AFTER commit; counter advances and
/// artifact record exists despite the error.
#[test]
fn compiled_ir_readback_failure_is_postcommit_artifact_exists() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    // Prepare expected artifact to know its digest before injection.
    let expected = prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])
        .map_err(|e| format!("prepare failed: {e}"))?;

    // Inject compiled IR readback failure.
    journal.fail_next_compiled_ir_readback_for_test();

    // Submission should fail with ArtifactMalformed (detected after commit).
    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);
    match result {
        Err(JournalError::ArtifactMalformed) => {}
        other => {
            return Err(format!(
                "expected ArtifactMalformed after readback failure, got: {other:?}"
            ));
        }
    }

    // Counter must be 1 (committed before readback check). Verify by submitting
    // the same workflow again; it must get seq 2, proving the first was committed.
    let artifact2 = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("followup submission failed: {e}"))?;
    if artifact2.accepted_at_seq.get() != 2 {
        return Err(format!(
            "followup artifact should have seq 2, got {}",
            artifact2.accepted_at_seq.get()
        ));
    }

    // Verify the compiled IR record from the failed submission actually exists
    // by reading it back using the expected artifact digest.
    let expected_digest = accepted_artifact_digest(&expected)
        .map_err(|e| format!("accepted_artifact_digest failed: {e}"))?;
    let record = journal
        .compiled_ir(expected_digest)
        .map_err(|e| format!("compiled_ir readback failed: {e}"))?;
    match record {
        Some(rec) => {
            if rec.digest != expected_digest {
                return Err("record digest does not match expected".to_string());
            }
        }
        None => {
            return Err(
                "compiled IR record should exist after postcommit readback failure".to_string(),
            );
        }
    }

    Ok(())
}

/// Invalid workflow canonical digest is rejected before any counter or artifact
/// mutation. This tests the checksum validation gate.
#[test]
fn invalid_workflow_canonical_digest_rejected_before_mutation() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    // Build invalid workflow: take parts, forge digest, try to reconstruct.
    let mut parts = workflow.to_parts();
    parts.digest = vb_core::WorkflowDigest::from_bytes([0xFF; 32]);
    let invalid = vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|e| {
        format!("constructor must reject forged digest, but it accepted it with error: {e}")
    })?;

    // The constructor accepted the forged digest (it doesn't validate),
    // so submit_artifact must reject it with ArtifactChecksumMismatch.
    let result = submit_artifact(&journal, &invalid, RuntimePolicy::Relaxed);
    match result {
        Err(JournalError::ArtifactChecksumMismatch) => {}
        other => {
            return Err(format!("expected ArtifactChecksumMismatch, got: {other:?}"));
        }
    }

    // Counter must still be 0. Submit valid workflow; must get seq 1.
    let valid_workflow = minimal_workflow()?;
    let artifact = submit_artifact(&journal, &valid_workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("valid workflow submission failed: {e}"))?;
    if artifact.accepted_at_seq.get() != 1 {
        return Err(format!(
            "valid workflow should get seq 1, got {}",
            artifact.accepted_at_seq.get()
        ));
    }

    Ok(())
}

/// Counter overflow: at u64::MAX - 1, one more submission succeeds at u64::MAX,
/// then the next fails with SequenceOverflow.
#[test]
fn counter_overflow_at_u64_max() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    // Set counter to u64::MAX - 1 by directly inserting into the keyspace.
    let near_max = u64::MAX
        .checked_sub(1)
        .ok_or_else(|| "u64::MAX - 1 underflow".to_string())?;
    let counter_bytes = near_max.to_be_bytes();
    journal
        .artifact_acceptance
        .insert(b"last_issued", &counter_bytes)
        .map_err(|e| format!("insert counter failed: {e}"))?;

    // First submission should succeed at seq u64::MAX.
    let artifact1 = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("submission at near-max failed: {e}"))?;
    if artifact1.accepted_at_seq.get() != u64::MAX {
        return Err(format!(
            "first artifact should have seq u64::MAX, got {}",
            artifact1.accepted_at_seq.get()
        ));
    }

    // Verify counter is now u64::MAX.
    let counter_value = journal
        .artifact_acceptance
        .get(b"last_issued")?
        .ok_or_else(|| "counter should be Some after overflow".to_string())?;
    if counter_value.len() != 8 {
        return Err(format!(
            "counter value wrong length: {}",
            counter_value.len()
        ));
    }
    let bytes: [u8; 8] = counter_value
        .try_into()
        .map_err(|_| "counter slice try_into failed".to_string())?;
    let counter = u64::from_be_bytes(bytes);
    if counter != u64::MAX {
        return Err(format!("counter should be u64::MAX, got {}", counter));
    }

    // Second submission should fail with SequenceOverflow.
    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed);
    match result {
        Err(JournalError::SequenceOverflow) => {}
        other => {
            return Err(format!("expected SequenceOverflow, got: {other:?}"));
        }
    }

    // Verify first artifact is still readable via public compiled_ir API.
    let record = journal
        .compiled_ir(artifact1.digest)
        .map_err(|e| format!("compiled_ir readback failed: {e}"))?;
    match record {
        Some(rec) => {
            if rec.digest != artifact1.digest {
                return Err("record digest does not match expected".to_string());
            }
        }
        None => return Err("first artifact should still be readable after overflow".to_string()),
    }

    Ok(())
}
