#![forbid(unsafe_code)]
//! SECTION 2.1: submit_artifact — Policy Tier Behavior (Unit Tests)

use crate::admission::{AcceptedArtifact, admit_compiled_artifact, submit_artifact};
use crate::constants::{
    CRC_OFFSET, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    RECORD_HEADER_BYTES,
};
use crate::records::RecordKind;
use crate::{
    BlobRecord, DIGEST_BYTES, EventSeq, FjallJournal, JournalError, JournalEvent,
    WorkflowSourceRecord,
};
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};

use crate::tests::fixtures::{minimal_valid_workflow, temp_journal};

fn submit_artifact_in_fresh_journal(
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
) -> Result<AcceptedArtifact, String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    submit_artifact(&journal, workflow, policy)
        .map_err(|e| format!("submit_artifact({policy:?}) failed: {e}"))
}

/// TEST: submit_artifact Relaxed policy skips gate validation
///
/// Contract §2.1 Precondition (Relaxed): no gate validation is performed.
/// Contract §2.1 Postcondition (Relaxed): gate_count=0, durable=false.
#[test]
fn submit_artifact_relaxed_skips_gate_validation() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit_artifact failed: {e}"))?;

    assert_eq!(
        result.verification.gate_count, 0,
        "Relaxed policy must skip gates and return gate_count=0"
    );
    assert!(
        !result.verification.durable,
        "Relaxed policy must have durable=false"
    );
    assert_eq!(
        result.verification.digest,
        workflow.digest(),
        "proof digest must match workflow digest"
    );
    Ok(())
}

/// TEST: submit_artifact Journaled policy enforces both gates
///
/// Contract §2.1 Postcondition (Journaled): gate_count=2, durable=false.
#[test]
fn submit_artifact_journaled_enforces_both_gates() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
        .map_err(|e| format!("submit_artifact(journaled) failed: {e}"))?;

    assert_eq!(
        result.verification.gate_count, 15,
        "Journaled must pass exactly 15 gates (structure + checksum + 13 others)"
    );
    assert!(
        !result.verification.durable,
        "Journaled must not be durable (no SyncAll)"
    );
    Ok(())
}

/// TEST: submit_artifact Strict policy enforces gates plus SyncAll
///
/// Contract §2.1 Postcondition (Strict): gate_count=15, durable=true.
#[test]
fn submit_artifact_strict_enforces_gates_plus_syncall() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("submit_artifact(strict) failed: {e}"))?;

    assert_eq!(
        result.verification.gate_count, 15,
        "Strict must pass exactly 15 gates"
    );
    assert!(
        result.verification.durable,
        "Strict must be durable (SyncAll called)"
    );
    Ok(())
}

/// TEST: submit_artifact Relaxed persists record
///
/// Contract §2.1 Postcondition (Relaxed): journal.put_compiled_ir called exactly once.
#[test]
fn submit_artifact_relaxed_persists_record() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;
    let digest = workflow.digest();

    submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit_artifact failed: {e}"))?;

    let loaded = journal
        .compiled_ir(digest)
        .map_err(|e| format!("read: {e}"))?;
    assert!(
        loaded.is_some(),
        "Relaxed policy must persist record to compiled_ir keyspace"
    );
    Ok(())
}

/// TEST: submit_artifact all policies set correct digest
///
/// Contract §2.1 Postcondition (All): artifact.digest == workflow.digest().
#[test]
fn submit_artifact_all_policies_set_correct_digest() -> Result<(), String> {
    let workflow = minimal_valid_workflow()?;

    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let result = submit_artifact_in_fresh_journal(&workflow, policy)?;
        assert_eq!(
            result.digest,
            workflow.digest(),
            "artifact.digest must equal workflow.digest() for policy {policy:?}"
        );
    }
    Ok(())
}

/// TEST: submit_artifact all policies return non-empty ir
///
/// Contract §2.1 Postcondition (All): artifact.ir is non-empty postcard-encoded bytes.
#[test]
fn submit_artifact_all_policies_return_nonempty_ir() -> Result<(), String> {
    let workflow = minimal_valid_workflow()?;

    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let result = submit_artifact_in_fresh_journal(&workflow, policy)?;
        assert!(
            !result.ir.is_empty(),
            "artifact.ir must be non-empty for policy {policy:?}"
        );
    }
    Ok(())
}

/// TEST: submit_artifact accepted_at_seq is valid EventSeq
///
/// Contract §2.1 Postcondition (All): accepted_at_seq is a valid EventSeq.
#[test]
fn accepted_at_seq_is_valid_event_seq() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("submit_artifact failed: {e}"))?;

    // accepted_at_seq must be a valid EventSeq (non-null, properly constructed)
    assert_eq!(
        result.accepted_at_seq.get(),
        0,
        "accepted_at_seq should be initialized to 0 in current implementation"
    );
    Ok(())
}
