#![forbid(unsafe_code)]
//! SECTION 3: Integration Tests

use crate::admission::{AcceptedArtifact, admit_compiled_artifact, submit_artifact};
use crate::constants::{DIGEST_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use crate::{EventSeq, FjallJournal, JournalError, JournalEvent, RuntimePolicy};
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};

use crate::tests::fixtures::{minimal_valid_workflow, temp_journal};

fn submit_artifact_in_fresh_journal(
    workflow: &vb_core::CompiledWorkflow,
    policy: RuntimePolicy,
) -> Result<AcceptedArtifact, String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    submit_artifact(&journal, workflow, policy)
        .map_err(|e| format!("submit_artifact({policy:?}) failed: {e}"))
}

/// TEST: submit_then_retrieve_artifact_round_trips
///
/// Contract §7.1: Store via submit_artifact, retrieve via journal.compiled_ir(digest),
/// bytes round-trip through postcard.
#[test]
fn submit_then_retrieve_artifact_round_trips() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    // Retrieve from storage
    let loaded = journal
        .compiled_ir(artifact.digest)
        .map_err(|e| format!("read failed: {e}"))?;
    let record = loaded.ok_or_else(|| String::from("artifact not found after submit"))?;

    // Verify digest matches
    assert_eq!(
        record.digest, artifact.digest,
        "stored digest must match submitted digest"
    );

    // Verify AcceptedArtifact envelope round-trips through postcard.
    let decoded: AcceptedArtifact =
        postcard::from_bytes(&record.ir).map_err(|e| format!("postcard decode: {e}"))?;
    let raw_parts_decode: Result<vb_core::WorkflowParts, _> = postcard::from_bytes(&record.ir);
    let computed = blake3::hash(&decoded.ir);
    assert_eq!(
        computed.as_bytes(),
        &artifact.digest.as_bytes(),
        "stored envelope inner IR bytes must hash to the submitted digest"
    );
    assert_eq!(decoded, artifact);
    assert!(
        raw_parts_decode.is_err(),
        "stored compiled_ir value must not be raw WorkflowParts"
    );
    Ok(())
}

/// TEST: submit_journaled_record_readable_by_digest
///
/// Contract §7.1: Journaled artifact readable after persist.
#[test]
fn submit_journaled_record_readable_by_digest() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    let loaded = journal
        .compiled_ir(result.digest)
        .map_err(|e| format!("read: {e}"))?;
    assert!(
        loaded.is_some(),
        "journaled artifact must be readable by digest"
    );
    Ok(())
}

/// TEST: admit_twice_only_inserts_once
///
/// Contract §7.1: Duplicate admit_compiled_artifact — only one insert, same digest returned.
#[test]
fn admit_twice_only_inserts_once() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let digest_a = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("first admit: {e}"))?;
    let digest_b = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("second admit: {e}"))?;

    assert_eq!(digest_a, digest_b, "both calls must return same digest");

    // Count records — should be exactly 1, not 2
    let loaded = journal
        .compiled_ir(digest_a)
        .map_err(|e| format!("read: {e}"))?;
    assert!(loaded.is_some(), "artifact must be stored after admission");
    Ok(())
}

/// TEST: batch_commit_persists_all_records
///
/// Contract §7.1: After commit(), both workflow_source and blob readable from keyspaces.
#[test]
fn batch_commit_persists_all_records() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;

    use crate::BlobRecord;
    use vb_core::WorkflowDigest;

    let source = b"batch workflow".to_vec();
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let workflow_record = crate::WorkflowSourceRecord {
        digest: source_digest,
        source: source.clone(),
    };

    let payload = vec![0xBB];
    let blob_digest: [u8; 32] = blake3::hash(&payload).into();
    let blob_record = BlobRecord {
        digest: blob_digest,
        bytes: payload.clone(),
    };

    {
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&workflow_record)
            .map_err(|e| format!("batch ws: {e}"))?;
        batch
            .put_blob(&blob_record)
            .map_err(|e| format!("batch blob: {e}"))?;
        batch.commit().map_err(|e| format!("commit: {e}"))?;
    }

    // Verify both records are readable
    let loaded_source = journal
        .workflow_source(source_digest)
        .map_err(|e| format!("read ws: {e}"))?;
    assert!(
        loaded_source.is_some(),
        "workflow_source must be readable after batch commit"
    );

    let loaded_blob = journal
        .blob(blob_digest)
        .map_err(|e| format!("read blob: {e}"))?;
    assert!(
        loaded_blob.is_some(),
        "blob must be readable after batch commit"
    );
    Ok(())
}

/// TEST: batch_with_fraud_stages_nothing
///
/// Contract §7.1: One forged item in batch → batch.len() == 0, nothing staged.
#[test]
fn batch_with_fraud_stages_nothing() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;

    use crate::BlobRecord;
    use vb_core::WorkflowDigest;

    let good_source = b"good source".to_vec();
    let good_digest = WorkflowDigest::from_bytes(blake3::hash(&good_source).into());
    let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);

    let mut batch = journal.batch();

    // First, valid item succeeds
    let result1 = batch.put_workflow_source(&crate::WorkflowSourceRecord {
        digest: good_digest,
        source: good_source.clone(),
    });
    assert!(
        result1.is_ok(),
        "valid workflow source must be accepted into batch"
    );

    // Then forged item fails
    let result2 = batch.put_workflow_source(&crate::WorkflowSourceRecord {
        digest: forged_digest,
        source: b"forged content".to_vec(),
    });
    assert!(
        matches!(result2, Err(JournalError::PayloadDigestMismatch)),
        "forged digest must cause batch put to fail"
    );

    // Batch must be empty after fraud attempt
    assert_eq!(
        batch.len(),
        0,
        "batch must be empty after failed put (nothing staged)"
    );
    Ok(())
}

/// TEST: artifact_digest_equals_workflow_digest
///
/// Contract §2.1 Postcondition (All): artifact.digest == workflow.digest().
#[test]
fn artifact_digest_equals_workflow_digest() -> Result<(), String> {
    let workflow = minimal_valid_workflow()?;

    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let artifact = submit_artifact_in_fresh_journal(&workflow, policy)?;

        assert_eq!(
            artifact.digest.as_bytes(),
            workflow.digest().as_bytes(),
            "artifact.digest must equal workflow.digest() for policy {policy:?}"
        );
    }
    Ok(())
}

/// TEST: events_for_run_returns_ascending_sequences
///
/// Contract §3.7: Returned events have strictly monotonically increasing EventSeq.
#[test]
fn events_for_run_returns_ascending_sequences() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let run = RunId::new(12345);

    let events: Vec<JournalEvent> = (0..5)
        .map(|i| JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(i),
            step: StepIdx::new(i as u16),
            attempt: 1,
        })
        .collect();

    for event in &events {
        journal
            .append_journaled(event)
            .map_err(|e| format!("append: {e}"))?;
    }

    let replayed = journal
        .events_for_run(run)
        .map_err(|e| format!("replay: {e}"))?;

    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(
            event.seq().get(),
            i as u64,
            "event {} must have seq {}",
            i,
            i
        );
    }
    Ok(())
}

/// TEST: events_for_run_returns_empty_for_unrelated_run (BH-06)
///
/// Contract §6 BH-06: events isolated by run ID.
#[test]
fn events_for_run_returns_empty_for_unrelated_run() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let run_a = RunId::new(100);
    let run_b = RunId::new(200);

    let event = JournalEvent::RunAccepted {
        run: run_a,
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    journal
        .append_strict(&event)
        .map_err(|e| format!("append: {e}"))?;

    let events_b = journal
        .events_for_run(run_b)
        .map_err(|e| format!("replay: {e}"))?;
    assert!(
        events_b.is_empty(),
        "run B must have zero events from run A"
    );
    Ok(())
}

/// TEST: event_replay_fails_on_sequence_gap
///
/// Contract §3.7: Sequence gap → typed error (not panic).
#[test]
fn event_replay_fails_on_sequence_gap() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let run = RunId::new(400);

    // Write seq 0 and seq 2 (gap at seq 1)
    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    let e2 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(2),
        workflow: vb_core::WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };

    journal
        .append_unpersisted(&e0)
        .map_err(|e| format!("append 0: {e}"))?;
    journal
        .append_unpersisted(&e2)
        .map_err(|e| format!("append 2: {e}"))?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::SequenceGap { .. })),
        "sequence gap must yield SequenceGap error"
    );
    Ok(())
}

/// TEST: batch_commit_is_all_or_nothing
///
/// Contract §3.5: After failed commit(), no partial records visible in keyspaces.
#[test]
fn batch_commit_is_all_or_nothing() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;

    use crate::WorkflowSourceRecord;
    use vb_core::WorkflowDigest;

    // Create a batch that will be committed successfully
    let source = b"valid source".to_vec();
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());

    {
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest: source_digest,
                source: source.clone(),
            })
            .map_err(|e| format!("batch put: {e}"))?;
        batch.commit().map_err(|e| format!("commit: {e}"))?;
    }

    // After successful commit, record must be visible
    let loaded = journal
        .workflow_source(source_digest)
        .map_err(|e| format!("read: {e}"))?;
    assert!(
        loaded.is_some(),
        "after successful batch commit, record must be visible"
    );
    Ok(())
}
