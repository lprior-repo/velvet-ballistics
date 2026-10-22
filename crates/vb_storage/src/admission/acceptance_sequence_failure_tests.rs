//! Failure-mode tests for the artifact acceptance sequence counter and issuance path.
//!
//! These tests cover strict durability failures, batch commit failures,
//! compiled-IR readback failures, invalid workflow digest rejection, and
//! counter overflow at u64::MAX. They prove that the RMW (read-modify-write)
//! sequence in artifact issuance preserves counter monotonicity and that
//! failures do not corrupt the journal.

use crate::error::JournalError;
use vb_core::RuntimePolicy;

// Parent module provides these private helpers for test use.
use super::{minimal_workflow, temp_journal};

/// Strict durability failure is precommit; counter must remain at 0.
#[test]
fn strict_persist_failure_is_precommit_counter_unchanged() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    // Inject strict persist failure.
    journal.fail_next_persist_for_test();

    // Submission should fail with strict durability error.
    let result = crate::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Strict);
    match result {
        Err(JournalError::StrictDurabilityFailed) => {}
        other => {
            return Err(format!(
                "expected StrictDurabilityFailed, got: {other:?}"
            ));
        }
    }

    // Counter must still be 0 — no artifact was issued.
    match journal.artifact_acceptance.get(b"last_issued")? {
        Some(_) => return Err("counter should be None after strict persist failure".to_string()),
        None => {}
    }

    // Compiled IR keyspace must be empty.
    let mut iter = journal.compiled_ir.iter();
    let first = iter.next();
    if first.is_some() {
        return Err("compiled_ir should be empty after strict persist failure".to_string());
    }

    // Retry with same workflow; must succeed at seq 1.
    let artifact = crate::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
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
    let result = crate::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Strict);
    match result {
        Err(JournalError::Fjall(_)) => {}
        other => {
            return Err(format!(
                "expected Fjall error, got: {other:?}"
            ));
        }
    }

    // Counter must still be 0.
    match journal.artifact_acceptance.get(b"last_issued")? {
        Some(_) => return Err("counter should be None after batch commit failure".to_string()),
        None => {}
    }

    // Artifact keyspace must be empty.
    let mut iter = journal.compiled_ir.iter();
    let first = iter.next();
    if first.is_some() {
        return Err("compiled_ir should be empty after batch commit failure".to_string());
    }

    // Retry succeeds at seq 1.
    let artifact = crate::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
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
/// artifact record exists despite the error. This proves there is no all-Err
/// rollback: the failure is detected after durable persistence.
#[test]
fn compiled_ir_readback_failure_is_postcommit_artifact_exists() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    // Inject compiled IR readback failure.
    journal.fail_next_compiled_ir_readback_for_test();

    // Submission should fail with ArtifactMalformed (detected after commit).
    let result = crate::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Strict);
    match result {
        Err(JournalError::ArtifactMalformed) => {}
        other => {
            return Err(format!(
                "expected ArtifactMalformed after readback failure, got: {other:?}"
            ));
        }
    }

    // Counter must be 1 (committed before readback check).
    match journal.artifact_acceptance.get(b"last_issued")? {
        Some(value) => {
            if value.len() != 8 {
                return Err(format!("counter value wrong length: {}", value.len()));
            }
            let bytes: [u8; 8] = value.try_into().map_err(|_| "counter slice try_into failed")?;
            let counter = u64::from_be_bytes(bytes);
            if counter != 1 {
                return Err(format!(
                    "counter should be 1 after postcommit readback failure, got {}",
                    counter
                ));
            }
        }
        None => return Err("counter should be Some after postcommit readback failure".to_string()),
    }

    // Verify the compiled IR record actually exists by looking it up directly.
    // We need the workflow digest to find it.
    let workflow_parts = workflow.to_parts();
    let serialized = postcard::to_allocvec(&workflow_parts)
        .map_err(|e| format!("serialize workflow parts: {e}"))?;
    let digest_bytes = blake3::hash(&serialized);
    let expected_digest = vb_core::WorkflowDigest::from_bytes(digest_bytes.into());

    // The journal's compiled_ir method should find it now (failure flag consumed).
    let record = journal.compiled_ir(expected_digest)
        .map_err(|e| format!("compiled_ir readback failed: {e}"))?;
    match record {
        Some(rec) => {
            if rec.digest != expected_digest {
                return Err("record digest does not match expected");
            }
        }
        None => return Err("compiled IR record should exist after postcommit readback failure".to_string()),
    }

    // Follow-up valid submission at seq 2.
    let artifact2 = crate::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("followup submission failed: {e}"))?;
    if artifact2.accepted_at_seq.get() != 2 {
        return Err(format!(
            "followup artifact should have seq 2, got {}",
            artifact2.accepted_at_seq.get()
        ));
    }

    Ok(())
}

/// Invalid workflow canonical digest is rejected before any counter or artifact
/// mutation. This tests the checksum validation gate.
#[test]
fn invalid_workflow_canonical_digest_rejected_before_mutation() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;

    // Build a valid workflow, then forge its digest to be wrong.
    use vb_core::value::ConstValue;
    use vb_core::workflow::{ResourceContract, WorkflowParts};
    use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, StepIdx, WorkflowDigest};

    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::<str>::from("test_bad_digest"),
        input_slots: Box::new([]),
        digest: WorkflowDigest::from_bytes([0xFFu8; 32]), // intentionally wrong
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Recompute correct digest for try_from_parts to accept the structure.
    let hash_bytes = postcard::to_allocvec(&parts)
        .map_err(|e| format!("serialize parts for digest: {e}"))?;
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());

    // Now forge: change the digest after computing it.
    parts.digest = WorkflowDigest::from_bytes([0xFFu8; 32]);

    // CompiledWorkflow::try_from_parts should reject the forged digest.
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(e) => {
            // If the constructor itself rejects it, that's actually better validation.
            // Check that it's a digest-related error.
            let err_str = e.to_string();
            if !err_str.contains("digest") && !err_str.contains("checksum") {
                return Err(format!(
                    "expected digest-related rejection, got: {err_str}"
                ));
            }
            // Counter should still be 0.
            match journal.artifact_acceptance.get(b"last_issued")? {
                Some(_) => return Err("counter should be None after constructor rejection".to_string()),
                None => {}
            }
            return Ok(());
        }
    };

    // If the constructor accepted it (unlikely), submission should reject it.
    let result = crate::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed);
    match result {
        Err(JournalError::ArtifactChecksumMismatch) => {}
        other => {
            return Err(format!(
                "expected ArtifactChecksumMismatch, got: {other:?}"
            ));
        }
    }

    // Counter must still be 0.
    match journal.artifact_acceptance.get(b"last_issued")? {
        Some(_) => return Err("counter should be None after checksum mismatch".to_string()),
        None => {}
    }

    // Valid workflow should still succeed at seq 1.
    let valid_workflow = minimal_workflow()?;
    let artifact = crate::admission::submit_artifact(&journal, &valid_workflow, RuntimePolicy::Relaxed)
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
    let near_max = u64::MAX.checked_sub(1).ok_or("u64::MAX - 1 underflow")?;
    let counter_bytes = near_max.to_be_bytes();
    journal.artifact_acceptance
        .put(b"last_issued", &counter_bytes)?;

    // First submission should succeed at seq u64::MAX.
    let artifact1 = crate::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("submission at near-max failed: {e}"))?;
    if artifact1.accepted_at_seq.get() != u64::MAX {
        return Err(format!(
            "first artifact should have seq u64::MAX, got {}",
            artifact1.accepted_at_seq.get()
        ));
    }

    // Verify counter is now u64::MAX.
    match journal.artifact_acceptance.get(b"last_issued")? {
        Some(value) => {
            if value.len() != 8 {
                return Err(format!("counter value wrong length: {}", value.len()));
            }
            let bytes: [u8; 8] = value.try_into().map_err(|_| "counter slice try_into failed")?;
            let counter = u64::from_be_bytes(bytes);
            if counter != u64::MAX {
                return Err(format!("counter should be u64::MAX, got {}", counter));
            }
        }
        None => return Err("counter should be Some after overflow".to_string()),
    }

    // Second submission should fail with SequenceOverflow.
    let result = crate::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed);
    match result {
        Err(JournalError::SequenceOverflow) => {}
        other => {
            return Err(format!(
                "expected SequenceOverflow, got: {other:?}"
            ));
        }
    }

    // Verify first artifact is still readable.
    let workflow_parts = workflow.to_parts();
    let serialized = postcard::to_allocvec(&workflow_parts)
        .map_err(|e| format!("serialize workflow parts: {e}"))?;
    let digest_bytes = blake3::hash(&serialized);
    let expected_digest = vb_core::WorkflowDigest::from_bytes(digest_bytes.into());

    let record = journal.compiled_ir(expected_digest)
        .map_err(|e| format!("compiled_ir readback failed: {e}"))?;
    match record {
        Some(rec) => {
            if rec.digest != expected_digest {
                return Err("record digest does not match expected");
            }
        }
        None => return Err("first artifact should still be readable after overflow".to_string()),
    }

    Ok(())
}