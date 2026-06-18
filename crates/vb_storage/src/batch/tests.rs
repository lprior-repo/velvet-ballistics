use super::*;
use crate::{
    BlobRecord, EventSeq, IndexStatusState, JournalEvent, constants::DIGEST_BYTES,
    recovery::RunSnapshot,
};
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

fn temp_journal() -> (tempfile::TempDir, crate::FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal =
        crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    }
}

fn make_run_header(run: RunId) -> RunHeaderRecord {
    RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(1),
        compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
        status: 1,
        accepted_at_ms: 1000,
    }
}

// =========================================================================
// JournalWriteBatch construction and initial state
// =========================================================================

#[test]
fn new_batch_is_empty_with_zero_length() {
    let (_temp, journal) = temp_journal();
    let batch = JournalWriteBatch::new(&journal);
    assert!(batch.is_empty(), "newly constructed batch must be empty");
    assert_eq!(
        batch.len(),
        0,
        "newly constructed batch must report length 0"
    );
}

#[test]
fn new_batch_from_journal_batch_method_is_empty() {
    let (_temp, journal) = temp_journal();
    let batch = journal.batch();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
}

// =========================================================================
// JournalWriteBatch len/is_empty tracking
// =========================================================================

#[test]
fn len_increments_after_each_append_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(1);
    let mut batch = JournalWriteBatch::new(&journal);

    batch.append_event(&make_event(run, 0)).expect("append 0");
    assert_eq!(batch.len(), 1);
    assert!(!batch.is_empty());

    batch.append_event(&make_event(run, 1)).expect("append 1");
    assert_eq!(batch.len(), 2);

    batch.append_event(&make_event(run, 2)).expect("append 2");
    assert_eq!(batch.len(), 3);
}

#[test]
fn len_increments_after_put_run_header() {
    let (_temp, journal) = temp_journal();
    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_run_header(&make_run_header(RunId::new(10)))
        .expect("put header");
    assert_eq!(batch.len(), 1);
    assert!(!batch.is_empty());
}

#[test]
fn len_increments_after_put_status_index() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20);
    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_status_index(IndexStatusState::Active, 12345, run)
        .expect("put status index");
    assert_eq!(batch.len(), 1);
}

#[test]
fn len_increments_after_put_workflow_index() {
    let (_temp, journal) = temp_journal();
    let wf = WorkflowId::new(5);
    let run = RunId::new(30);
    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_workflow_index(wf, run)
        .expect("put workflow index");
    assert_eq!(batch.len(), 1);
}

#[test]
fn len_increments_after_put_action_index() {
    let (_temp, journal) = temp_journal();
    let action = vb_core::ActionId::new(99);
    let run = RunId::new(40);
    let step = StepIdx::new(0);
    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_action_index(action, run, step)
        .expect("put action index");
    assert_eq!(batch.len(), 1);
}

// =========================================================================
// Batch flush: empty batch commit
// =========================================================================

#[test]
fn empty_batch_commit_succeeds() {
    let (_temp, journal) = temp_journal();
    let batch = JournalWriteBatch::new(&journal);
    let result = batch.commit();
    assert!(
        result.is_ok(),
        "committing an empty batch should succeed, got {:?}",
        result
    );
}

// =========================================================================
// Batch flush: single event
// =========================================================================

#[test]
fn commit_with_single_event_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(100);
    let event = make_event(run, 0);

    let mut batch = JournalWriteBatch::new(&journal);
    batch.append_event(&event).expect("append should succeed");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let events = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(
        events.len(),
        1,
        "should find 1 event after single-event batch commit"
    );
    assert_eq!(events[0], event);
}

// =========================================================================
// Batch flush: multiple events of different kinds
// =========================================================================

#[test]
fn commit_with_multiple_events_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(200);
    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]),
    };
    let e1 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    let e2 = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(2),
        result: SlotIdx::new(0),
        attempt: 1,
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch.append_event(&e0).expect("append 0");
    batch.append_event(&e1).expect("append 1");
    batch.append_event(&e2).expect("append 2");
    assert_eq!(batch.len(), 3);
    batch.commit().expect("commit should succeed");

    let events = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(
        events.len(),
        3,
        "should find 3 events after multi-event batch"
    );
    assert_eq!(events[0], e0);
    assert_eq!(events[1], e1);
    assert_eq!(events[2], e2);
}

// =========================================================================
// Batch with put_workflow_source (valid digest)
// =========================================================================

#[test]
fn batch_put_workflow_source_with_valid_digest_commits() {
    let (_temp, journal) = temp_journal();
    let source = b"workflow: batch_test".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_workflow_source(&record)
        .expect("put workflow source");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let loaded = journal.workflow_source(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("workflow source should be found after batch commit");
    };
    assert_eq!(found.source, source);
}

// =========================================================================
// Batch with put_compiled_ir (valid)
// =========================================================================

#[test]
fn batch_put_compiled_ir_with_valid_digest_commits() {
    let (_temp, journal) = temp_journal();
    let record = crate::accepted_compiled_ir_record_for_test(b"compiled-batch-test".to_vec());
    let digest = record.digest;

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_compiled_ir(&record).expect("put compiled ir");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let loaded = journal.compiled_ir(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("compiled IR should be found after batch commit");
    };
    assert_eq!(found, record);
}

// =========================================================================
// Batch with put_run_header
// =========================================================================

#[test]
fn batch_put_run_header_commits_and_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(300);
    let header = make_run_header(run);

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_run_header(&header).expect("put run header");
    batch.commit().expect("commit should succeed");

    let loaded = journal.run_header(run).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("run header should be found after batch commit");
    };
    assert_eq!(found.run, run);
    assert_eq!(found.status, 1);
}

// =========================================================================
// Batch with put_snapshot
// =========================================================================

#[test]
fn batch_put_snapshot_commits_and_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(400);
    let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow,
        slots: vec![1, 2, 3],
        taint: vec![0],
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_snapshot(&snapshot).expect("put snapshot");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let loaded = journal
        .snapshot(run, EventSeq::new(5))
        .expect("get should succeed")
        .expect("snapshot should exist");
    assert_eq!(loaded.run, run);
    assert_eq!(loaded.slots, vec![1, 2, 3]);
    assert_eq!(loaded.taint, vec![0]);
}

// =========================================================================
// Batch with put_blob (valid digest)
// =========================================================================

#[test]
fn batch_put_blob_with_valid_digest_commits() {
    let (_temp, journal) = temp_journal();
    let payload = vec![0xCA, 0xFE];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
    let record = BlobRecord {
        digest,
        bytes: payload.clone(),
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_blob(&record).expect("put blob");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let loaded = journal.blob(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("blob should be found after batch commit");
    };
    assert_eq!(found.bytes, payload);
}

// =========================================================================
// Batch strict durability mode
// =========================================================================

#[test]
fn batch_strict_mode_commits_successfully() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(500);
    let event = make_event(run, 0);

    let batch = JournalWriteBatch::new(&journal);
    let mut batch = batch.strict();
    batch.append_event(&event).expect("append should succeed");
    batch.commit().expect("strict commit should succeed");

    let events = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(events.len(), 1);
}

// =========================================================================
// Cross-keyspace batch with mixed operation types
// =========================================================================

#[test]
fn batch_mixed_operations_across_keyspaces_commit_atomically() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(600);
    let digest = WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]);

    let source = b"batch mixed ops source".to_vec();
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let workflow_record = WorkflowSourceRecord {
        digest: source_digest,
        source,
    };

    let ir_record = crate::accepted_compiled_ir_record_for_test(b"batch mixed ops ir".to_vec());

    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(42),
        compiled_digest: digest,
        status: 1,
        accepted_at_ms: 9999,
    };

    let event = make_event(run, 0);

    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_workflow_source(&workflow_record)
        .expect("workflow source");
    batch.put_compiled_ir(&ir_record).expect("compiled ir");
    batch.put_run_header(&header).expect("run header");
    batch.append_event(&event).expect("event");
    batch
        .put_status_index(IndexStatusState::Active, 100, run)
        .expect("status index");
    batch
        .put_workflow_index(WorkflowId::new(42), run)
        .expect("workflow index");
    batch
        .put_action_index(vb_core::ActionId::new(1), run, StepIdx::new(0))
        .expect("action index");

    assert_eq!(batch.len(), 7, "batch should track 7 operations");
    batch.commit().expect("mixed batch commit should succeed");

    // Verify all keyspaces were written
    assert!(
        journal
            .workflow_source(source_digest)
            .expect("get ws")
            .is_some(),
        "workflow source should exist"
    );
    assert!(
        journal
            .compiled_ir(ir_record.digest)
            .expect("get ir")
            .is_some(),
        "compiled IR should exist"
    );
    assert!(
        journal.run_header(run).expect("get header").is_some(),
        "run header should exist"
    );
    let events = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(events.len(), 1, "should have 1 event");
}

// =========================================================================
// Edge case: batch put_workflow_source with wrong digest
// =========================================================================

#[test]
fn batch_put_workflow_source_rejects_digest_mismatch() {
    let (_temp, journal) = temp_journal();
    let source = b"real content".to_vec();
    let wrong_digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
    let record = WorkflowSourceRecord {
        digest: wrong_digest,
        source,
    };

    let mut batch = JournalWriteBatch::new(&journal);
    let result = batch.put_workflow_source(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "batch must reject digest mismatch, got {:?}",
        result
    );
    // Batch length should still be 0 since the put failed
    assert_eq!(batch.len(), 0);
}

// =========================================================================
// Edge case: batch put_blob with wrong digest
// =========================================================================

#[test]
fn batch_put_blob_rejects_digest_mismatch() {
    let (_temp, journal) = temp_journal();
    let payload = vec![1, 2, 3];
    let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; DIGEST_BYTES];
    let record = BlobRecord {
        digest: wrong_digest,
        bytes: payload,
    };

    let mut batch = JournalWriteBatch::new(&journal);
    let result = batch.put_blob(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "batch must reject blob digest mismatch, got {:?}",
        result
    );
    assert_eq!(batch.len(), 0);
}

// =========================================================================
// Edge case: zero-length batch followed by strict commit
// =========================================================================

#[test]
fn empty_strict_batch_commit_succeeds() {
    let (_temp, journal) = temp_journal();
    let batch = JournalWriteBatch::new(&journal);
    let batch = batch.strict();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
    batch
        .commit()
        .expect("empty strict batch commit should succeed");
}

// =========================================================================
// Edge case: put index operations do not carry payloads
// =========================================================================

#[test]
fn batch_index_operations_increment_len_without_payloads() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(700);
    let wf = WorkflowId::new(10);
    let action = vb_core::ActionId::new(20);
    let step = StepIdx::new(1);

    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_status_index(IndexStatusState::Completed, 5000, run)
        .expect("status idx");
    batch.put_workflow_index(wf, run).expect("workflow idx");
    batch
        .put_action_index(action, run, step)
        .expect("action idx");
    assert_eq!(batch.len(), 3, "three index operations should yield len 3");
    assert!(!batch.is_empty());
    batch.commit().expect("index batch commit should succeed");

    // Verify index markers are present
    let mut status_count = 0usize;
    for item in journal.index_status.iter() {
        let _ = item.key();
        status_count = status_count.saturating_add(1);
    }
    assert_eq!(status_count, 1, "should have 1 status index marker");

    let mut wf_count = 0usize;
    for item in journal.index_workflow.iter() {
        let _ = item.key();
        wf_count = wf_count.saturating_add(1);
    }
    assert_eq!(wf_count, 1, "should have 1 workflow index marker");

    let mut action_count = 0usize;
    for item in journal.index_action.iter() {
        let _ = item.key();
        action_count = action_count.saturating_add(1);
    }
    assert_eq!(action_count, 1, "should have 1 action index marker");
}

// =========================================================================
// RED PHASE: vb-fb52 failing tests — Atomic Journal and Index Write Batches
// =========================================================================
// NOTE: batch_is_not_send_or_sync is now enforced at the type level via
// PhantomData<*mut FjallJournal>. The type literally cannot implement Send
// or Sync without unsafe code, so no runtime test is needed.

#[test]
fn batch_put_compiled_ir_commits_and_is_readable() {
    // I3: compiled_ir readable after batch commit
    let (_temp, journal) = temp_journal();
    let record = crate::accepted_compiled_ir_record_for_test(b"compiled-artifact-bytes".to_vec());
    let digest = record.digest;

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_compiled_ir(&record).expect("batch compiled ir");
    batch.commit().expect("commit should succeed");

    let loaded = journal.compiled_ir(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("compiled IR should be found after batch commit");
    };
    assert_eq!(found, record);
}

#[test]
fn batch_append_event_commits_and_is_readable() {
    // I6: event readable after batch commit
    let (_temp, journal) = temp_journal();
    let run = RunId::new(100);
    let event = make_event(run, 0);

    let mut batch = JournalWriteBatch::new(&journal);
    batch.append_event(&event).expect("append event");
    batch.commit().expect("commit should succeed");

    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 1, "should have 1 event after batch commit");
    assert_eq!(replayed[0], event);
}

#[test]
fn batch_append_event_rejects_duplicate_event() {
    // EP-7, I20: DuplicateEvent on second batch append with same run_id+seq
    let (_temp, journal) = temp_journal();
    let run = RunId::new(200);
    let event = make_event(run, 0);

    // First append via batch
    let mut batch1 = JournalWriteBatch::new(&journal);
    batch1
        .append_event(&event)
        .expect("first append should succeed");
    batch1.commit().expect("commit should succeed");

    // Second append with same run_id+seq should fail
    let mut batch2 = JournalWriteBatch::new(&journal);
    let result = batch2.append_event(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "duplicate event must be rejected with DuplicateEvent, got {:?}",
        result
    );
    assert_eq!(
        batch2.len(),
        0,
        "batch len should remain 0 after failed append"
    );
}

#[test]
fn len_equals_staged_count_after_random_operations() {
    // P1: len() always equals actual staged operation count
    let (_temp, journal) = temp_journal();
    let run = RunId::new(400);

    let mut batch = JournalWriteBatch::new(&journal);
    let mut expected_len = 0;

    // Stage 3 events
    for i in 0..3 {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        batch.append_event(&evt).expect("append should succeed");
        expected_len += 1;
        assert_eq!(
            batch.len(),
            expected_len,
            "len() must equal staged count after each operation"
        );
    }

    // Stage a header
    let header = make_run_header(run);
    batch.put_run_header(&header).expect("put header");
    expected_len += 1;
    assert_eq!(batch.len(), expected_len);

    batch.commit().expect("commit should succeed");
}

#[test]
fn is_empty_equals_len_zero_invariant() {
    // P2: is_empty() == (len() == 0) holds after every operation
    let (_temp, journal) = temp_journal();
    let run = RunId::new(500);

    let mut batch = JournalWriteBatch::new(&journal);

    // Initially empty
    assert!(
        batch.is_empty() == (batch.len() == 0),
        "is_empty() must match (len() == 0) for new batch"
    );

    // After one operation
    batch.append_event(&make_event(run, 0)).expect("append");
    assert!(
        batch.is_empty() == (batch.len() == 0),
        "is_empty() must match (len() == 0) after one operation"
    );

    // After more operations
    batch
        .put_run_header(&make_run_header(run))
        .expect("put header");
    assert!(
        batch.is_empty() == (batch.len() == 0),
        "is_empty() must match (len() == 0) after multiple operations"
    );
}

#[test]
fn batch_len_never_decreases() {
    // P3: len() monotonically increases (never decreases)
    let (_temp, journal) = temp_journal();
    let run = RunId::new(600);

    let mut batch = JournalWriteBatch::new(&journal);
    let mut prev_len = 0;

    let operations = 5;
    for i in 0..operations {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        batch.append_event(&evt).expect("append");
        assert!(
            batch.len() > prev_len,
            "len() must increase monotonically, prev={}, new={}",
            prev_len,
            batch.len()
        );
        prev_len = batch.len();
    }

    batch.commit().expect("commit");
}

#[test]
fn all_or_nothing_commit_across_keyspaces() {
    // P5: commit is all-or-nothing; no partial state visible
    let (_temp, journal) = temp_journal();
    let run = RunId::new(800);
    let digest = WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]);

    let source = b"batch atomic test".to_vec();
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let workflow_record = WorkflowSourceRecord {
        digest: source_digest,
        source,
    };

    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(42),
        compiled_digest: digest,
        status: 1,
        accepted_at_ms: 9999,
    };

    {
        let mut batch = JournalWriteBatch::new(&journal);
        batch.put_workflow_source(&workflow_record).expect("ws");
        batch.put_run_header(&header).expect("header");
        batch.commit().expect("commit should succeed");
    }

    // All or nothing: both must be present or neither
    let ws_present = journal
        .workflow_source(source_digest)
        .expect("get ws")
        .is_some();
    let header_present = journal.run_header(run).expect("get header").is_some();
    assert_eq!(
        ws_present, header_present,
        "commit must be all-or-nothing across keyspaces"
    );
}

#[test]
fn digest_verification_mandatory_on_workflow_source() {
    // P7: BLAKE3 digest verification cannot be skipped for workflow_source
    let (_temp, journal) = temp_journal();
    let source = b"content to forge".to_vec();
    let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);

    let record = WorkflowSourceRecord {
        digest: forged_digest,
        source,
    };

    let mut batch = JournalWriteBatch::new(&journal);
    let result = batch.put_workflow_source(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "workflow_source digest verification must be mandatory"
    );
}

#[test]
fn digest_verification_mandatory_on_blob() {
    // P8: BLAKE3 digest verification cannot be skipped for blob
    let (_temp, journal) = temp_journal();
    let payload = vec![1, 2, 3, 4, 5];
    let forged_digest: [u8; DIGEST_BYTES] = [0xAB; 32];

    let record = BlobRecord {
        digest: forged_digest,
        bytes: payload,
    };

    let mut batch = JournalWriteBatch::new(&journal);
    let result = batch.put_blob(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "blob digest verification must be mandatory"
    );
}
