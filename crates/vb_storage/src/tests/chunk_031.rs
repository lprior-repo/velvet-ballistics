#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::prelude::*;

#[test]
fn queue_pending_count_matches_enqueued() {
    let queue = JournalWriterQueue::new(16, 4, StorageLimits::DEFAULT).expect("setup: queue");
    let run = RunId::new(5005);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([5; 32]),
    };
    let counts_empty = queue.pending_profile_counts().expect("counts must succeed");
    assert_eq!(counts_empty.journaled, 0);
    assert_eq!(counts_empty.strict, 0);
    queue
        .enqueue_journaled(event.clone())
        .expect("enqueue 0 must succeed");
    queue
        .enqueue_journaled(event.clone())
        .expect("enqueue 1 must succeed");
    queue.enqueue_strict(event).expect("enqueue 2 must succeed");
    let counts = queue.pending_profile_counts().expect("counts must succeed");
    assert_eq!(counts.journaled, 2, "two journaled events must be counted");
    assert_eq!(counts.strict, 1, "one strict event must be counted");
}

// --- FjallJournal open/close/reopen (tests 23-30) ---

#[test]
fn journal_open_creates_fresh_database() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let events = journal
        .events_for_run(RunId::new(1))
        .expect("events_for_run must succeed");
    assert!(events.is_empty(), "fresh database must have no events");
    let header = journal
        .run_header(RunId::new(1))
        .expect("run_header must succeed");
    assert_eq!(header, None, "fresh database must have no headers");
    let blob = journal.blob([0; 32]).expect("blob must succeed");
    assert_eq!(blob, None, "fresh database must have no blobs");
}

#[test]
fn journal_close_and_reopen_preserves_strict_data() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let path = temp_dir.path().to_path_buf();
    let digest = WorkflowDigest::from_bytes([0xEE; 32]);
    let run = RunId::new(6001);
    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(5),
        compiled_digest: digest,
        status: 3,
        accepted_at_ms: 999,
    };
    {
        let journal = FjallJournal::open(&path, None).expect("setup: journal open");
        journal
            .put_run_header(&header)
            .expect("put_run_header must succeed");
        journal
            .persist_strict()
            .expect("persist_strict must succeed");
    }
    let reopened = FjallJournal::open(&path, None).expect("reopen must succeed");
    let found = reopened.run_header(run).expect("run_header must succeed");
    assert_eq!(found, Some(header), "strict data must survive reopen");
}

#[test]
fn journal_multiple_opens_same_path_fails_or_succeeds() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal1 = FjallJournal::open(temp_dir.path(), None).expect("first open must succeed");
    let journal2_result = FjallJournal::open(temp_dir.path(), None);
    drop(journal1);
    if let Ok(j2) = journal2_result {
        drop(j2);
    }
}

#[test]
fn journal_put_then_get_workflow_source_consistent() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let source = b"consistent_source".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    journal
        .put_workflow_source(&record)
        .expect("put_workflow_source must succeed");
    let found = journal
        .workflow_source(digest)
        .expect("workflow_source must succeed");
    assert_eq!(
        found,
        Some(record),
        "put-then-get must be consistent in same session"
    );
}

#[test]
fn journal_put_then_get_compiled_ir_consistent() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let record = crate::accepted_compiled_ir_record_for_test(b"consistent_ir".to_vec());
    let digest = record.digest;
    journal
        .put_compiled_ir(&record)
        .expect("put_compiled_ir must succeed");
    let found = journal
        .compiled_ir(digest)
        .expect("compiled_ir must succeed");
    assert_eq!(
        found,
        Some(record),
        "put-then-get must be consistent in same session"
    );
}

#[test]
fn journal_put_then_get_run_header_consistent() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(6002);
    let record = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(99),
        compiled_digest: WorkflowDigest::from_bytes([0x99; 32]),
        status: 7,
        accepted_at_ms: 123456789,
    };
    journal
        .put_run_header(&record)
        .expect("put_run_header must succeed");
    let found = journal.run_header(run).expect("run_header must succeed");
    assert_eq!(
        found,
        Some(record),
        "put-then-get must be consistent in same session"
    );
}

#[test]
fn journal_put_then_get_snapshot_consistent() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(6003);
    let seq = EventSeq::new(4);
    let snapshot = RunSnapshot {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([0xAA; 32]),
        slots: vec![0xDE, 0xAD],
        taint: Vec::new(),
    };
    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot must succeed");
    let found = journal.snapshot(run, seq).expect("snapshot must succeed");
    assert_eq!(
        found,
        Some(snapshot),
        "put-then-get must be consistent in same session"
    );
}

#[test]
fn journal_put_then_get_blob_consistent() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let blob_bytes = b"consistent_blob".to_vec();
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let record = BlobRecord {
        digest,
        bytes: blob_bytes,
    };
    journal.put_blob(&record).expect("put_blob must succeed");
    let found = journal.blob(digest).expect("blob must succeed");
    assert_eq!(
        found,
        Some(record),
        "put-then-get must be consistent in same session"
    );
}

// --- Index queries (tests 31-35) ---

#[test]
fn status_index_stores_and_queries_by_state() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let state = IndexStatusState::Other(3);
    let timestamp: u64 = 1700000000;
    let run = RunId::new(7001);
    journal
        .put_status_index(state, timestamp, run)
        .expect("put_status_index must succeed");
    let key = index_status_key(state, timestamp, run).expect("key must succeed");
    let value = journal
        .index_status
        .get(key.as_slice())
        .expect("get must succeed");
    assert!(value.is_some(), "status index entry must exist after put");
}
