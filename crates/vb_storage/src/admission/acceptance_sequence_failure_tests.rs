//! Failure-mode tests for the artifact acceptance sequence counter and issuance path.

use crate::admission::*;
use crate::error::JournalError;
use crate::types::EventSeq;
use vb_core::RuntimePolicy;

use super::{minimal_workflow, temp_journal};

/// Shared helper: read counter value or report error/absent.
fn counter_value(journal: &crate::FjallJournal) -> Result<Option<u64>, String> {
    let raw = journal
        .artifact_acceptance
        .get(b"last_issued")
        .map_err(|e| format!("counter read failed: {e}"))?;
    match raw {
        None => Ok(None),
        Some(slice) => {
            let bytes = slice.as_ref();
            if bytes.len() != 8 {
                return Err(format!("counter wrong length: {}", bytes.len()));
            }
            let arr: [u8; 8] = bytes
                .try_into()
                .map_err(|_| "counter slice conversion failed".to_string())?;
            Ok(Some(u64::from_be_bytes(arr)))
        }
    }
}

/// Shared helper: verify compiled-IR keyspace is empty.
fn compiled_empty(journal: &crate::FjallJournal) -> Result<(), String> {
    if journal.compiled_ir.iter().next().is_some() {
        return Err("compiled_ir should be empty but has entries".to_string());
    }
    Ok(())
}

/// Shared helper: full artifact readback and verification.
fn verify_artifact_readback(
    journal: &crate::FjallJournal,
    digest: vb_core::WorkflowDigest,
    expected: &AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<(), String> {
    let record = journal
        .compiled_ir(digest)
        .map_err(|e| format!("compiled_ir readback failed: {e}"))?
        .ok_or_else(|| "artifact record not found".to_string())?;
    if record.digest != digest {
        return Err("record digest does not match expected".to_string());
    }
    let decoded: AcceptedArtifact =
        postcard::from_bytes(&record.ir).map_err(|e| format!("postcard decode failed: {e}"))?;
    if accepted_artifact_digest(&decoded)
        .map_err(|e| format!("accepted_artifact_digest failed: {e}"))?
        != decoded.digest
    {
        return Err("decoded artifact digest is inconsistent".to_string());
    }
    if decoded.verification.digest != decoded.digest {
        return Err("verification.digest mismatch".to_string());
    }
    if decoded != *expected {
        return Err("decoded artifact does not match expected".to_string());
    }
    verify_accepted_artifact_for_policy(&decoded, policy)
        .map_err(|e| format!("verify_accepted_artifact_for_policy failed: {e}"))?;
    Ok(())
}

/// Strict durability failure is precommit; counter must remain at 0.
#[test]
fn strict_persist_failure_is_precommit_counter_unchanged() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    journal.fail_next_persist_for_test();

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);
    match result {
        Err(JournalError::StrictDurabilityFailed) => {}
        other => return Err(format!("expected StrictDurabilityFailed, got: {other:?}")),
    }

    let counter = counter_value(&journal)?;
    if counter.is_some() {
        return Err(format!(
            "counter should be 0 after strict failure, got: {counter:?}"
        ));
    }
    compiled_empty(&journal)?;

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

    journal.fail_next_batch_commit_for_test();

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);
    let got_fjall_io = matches!(
        &result,
        Err(JournalError::Fjall(e)) if matches!(e, fjall::Error::Io(io) if io.kind() == std::io::ErrorKind::Other)
    );
    if !got_fjall_io {
        return Err(format!("expected Fjall(Io(Other)) error, got: {result:?}"));
    }

    let counter = counter_value(&journal)?;
    if counter.is_some() {
        return Err(format!(
            "counter should be 0 after batch failure, got: {counter:?}"
        ));
    }
    compiled_empty(&journal)?;

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

    let mut expected = prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])
        .map_err(|e| format!("prepare failed: {e}"))?;
    expected.accepted_at_seq = EventSeq::new(1);
    expected.digest = accepted_artifact_digest(&expected)
        .map_err(|e| format!("expected digest recompute failed: {e}"))?;
    expected.verification.digest = expected.digest;

    journal.fail_next_compiled_ir_readback_for_test();

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);
    match result {
        Err(JournalError::ArtifactMalformed) => {}
        other => {
            return Err(format!(
                "expected ArtifactMalformed after readback failure, got: {other:?}"
            ));
        }
    }

    let counter = counter_value(&journal)?;
    if counter != Some(1) {
        return Err(format!(
            "counter should be 1 after postcommit failure, got: {counter:?}"
        ));
    }

    verify_artifact_readback(&journal, expected.digest, &expected, RuntimePolicy::Strict)?;

    let artifact2 = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
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

    let mut parts = minimal_workflow()?.to_parts();
    parts.digest = vb_core::WorkflowDigest::from_bytes([0xFF; 32]);
    let invalid = vb_core::CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| format!("expected forged-digest workflow to reconstruct, got: {e}"))?;

    let result = submit_artifact(&journal, &invalid, RuntimePolicy::Relaxed);
    match result {
        Err(JournalError::ArtifactChecksumMismatch) => {}
        other => return Err(format!("expected ArtifactChecksumMismatch, got: {other:?}")),
    }

    let counter = counter_value(&journal)?;
    if counter.is_some() {
        return Err(format!(
            "counter should be 0 after digest rejection, got: {counter:?}"
        ));
    }
    compiled_empty(&journal)?;

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

    let near_max = u64::MAX
        .checked_sub(1)
        .ok_or_else(|| "u64::MAX - 1 underflow".to_string())?;
    journal
        .artifact_acceptance
        .insert(b"last_issued", &near_max.to_be_bytes())
        .map_err(|e| format!("insert counter failed: {e}"))?;

    let artifact1 = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("submission at near-max failed: {e}"))?;
    if artifact1.accepted_at_seq.get() != u64::MAX {
        return Err(format!(
            "first artifact should have seq u64::MAX, got {}",
            artifact1.accepted_at_seq.get()
        ));
    }

    let counter = counter_value(&journal)?;
    if counter != Some(u64::MAX) {
        return Err(format!("counter should be u64::MAX, got: {counter:?}"));
    }

    verify_artifact_readback(
        &journal,
        artifact1.digest,
        &artifact1,
        RuntimePolicy::Relaxed,
    )?;

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed);
    match result {
        Err(JournalError::SequenceOverflow) => {}
        other => return Err(format!("expected SequenceOverflow, got: {other:?}")),
    }

    let counter_after = counter_value(&journal)?;
    if counter_after != Some(u64::MAX) {
        return Err(format!(
            "counter should remain u64::MAX after overflow, got: {counter_after:?}"
        ));
    }

    verify_artifact_readback(
        &journal,
        artifact1.digest,
        &artifact1,
        RuntimePolicy::Relaxed,
    )?;

    Ok(())
}
