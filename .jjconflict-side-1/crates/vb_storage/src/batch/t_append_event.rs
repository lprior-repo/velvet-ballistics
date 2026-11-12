#![forbid(unsafe_code)]
use super::*;

#[test]
fn batch_append_event_commits_and_is_readable() {
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
    let (_temp, journal) = temp_journal();
    let run = RunId::new(200);
    let event = make_event(run, 0);

    let mut batch1 = JournalWriteBatch::new(&journal);
    batch1
        .append_event(&event)
        .expect("first append should succeed");
    batch1.commit().expect("commit should succeed");

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
fn batch_append_event_rejects_invalid_event_without_staging() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(201);
    let event = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(1),
        attempt: 0,
    };

    let mut batch = JournalWriteBatch::new(&journal);
    let result = batch.append_event(&event);

    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "invalid event must be rejected before staging, got {:?}",
        result
    );
    assert_eq!(batch.len(), 0);
    assert!(!batch.is_aborted());
}

#[test]
fn len_equals_staged_count_after_random_operations() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(400);

    let mut batch = JournalWriteBatch::new(&journal);
    let mut expected_len = 0;

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

    let header = make_run_header(run);
    batch.put_run_header(&header).expect("put header");
    expected_len += 1;
    assert_eq!(batch.len(), expected_len);

    batch.commit().expect("commit should succeed");
}

#[test]
fn is_empty_equals_len_zero_invariant() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(500);

    let mut batch = JournalWriteBatch::new(&journal);

    assert!(
        batch.is_empty() == (batch.len() == 0),
        "is_empty() must match (len() == 0) for new batch"
    );

    batch.append_event(&make_event(run, 0)).expect("append");
    assert!(
        batch.is_empty() == (batch.len() == 0),
        "is_empty() must match (len() == 0) after one operation"
    );

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
