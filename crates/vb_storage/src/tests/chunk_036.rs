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
fn adversarial_reopen_after_flushed_journaled_events_preserves_them() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9002);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([2; 32]),
    };
    journal.append_journaled(&event).expect("append journaled");
    drop(journal);
    let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
    let events = journal2
        .events_for_run(run)
        .expect("events_for_run succeeds");
    assert_eq!(
        events.len(),
        1,
        "flushed journaled event must survive reopen"
    );
}

#[test]
fn adversarial_reopen_after_strict_event_preserves_it() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9003);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([3; 32]),
    };
    journal.append_strict(&event).expect("append strict");
    drop(journal);
    let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
    let events = journal2
        .events_for_run(run)
        .expect("events_for_run succeeds");
    assert_eq!(events.len(), 1, "strict event must survive reopen");
}

#[test]
fn adversarial_batch_commit_then_reopen_preserves_all_keys() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let source_bytes = b"source".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
    let run = RunId::new(9004);
    let mut batch = journal.batch();
    batch
        .put_workflow_source(&WorkflowSourceRecord {
            digest,
            source: source_bytes,
        })
        .expect("put_workflow_source");
    batch
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 100,
        })
        .expect("put_run_header");
    let blob_bytes = b"blob".to_vec();
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    batch
        .put_blob(&BlobRecord {
            digest: blob_digest,
            bytes: blob_bytes,
        })
        .expect("put_blob");
    batch.commit().expect("commit");
    drop(journal);
    let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
    let source = journal2.workflow_source(digest).expect("get source");
    assert!(
        source.is_some(),
        "workflow source must survive reopen, got {:?}",
        source
    );
    assert_eq!(source.unwrap().source, b"source".to_vec());
    let header = journal2.run_header(run).expect("get header");
    assert!(
        header.is_some(),
        "run header must survive reopen, got {:?}",
        header
    );
    assert_eq!(header.unwrap().run, run);
    let blob = journal2.blob(blob_digest).expect("get blob");
    assert!(
        blob.is_some(),
        "blob must survive reopen, got {:?}",
        blob
    );
    assert_eq!(blob.unwrap().bytes, b"blob".to_vec());
}

#[test]
fn adversarial_double_append_same_run_seq_returns_duplicate_error() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9005);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([5; 32]),
    };
    journal.append_strict(&event).expect("first append");
    let result = journal.append_strict(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "duplicate append must return DuplicateEvent"
    );
}

#[test]
fn adversarial_events_for_run_on_empty_journal_returns_empty() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let events = journal
        .events_for_run(RunId::new(9999))
        .expect("events_for_run");
    assert_eq!(events.len(), 0, "no events for nonexistent run");
}

#[test]
fn adversarial_run_header_for_never_written_run_returns_none() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let header = journal.run_header(RunId::new(8888)).expect("run_header");
    assert!(header.is_none(), "no header for nonexistent run");
}

#[test]
fn adversarial_snapshot_for_nonexistent_run_returns_none() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let snapshot = journal
        .snapshot(RunId::new(7777), EventSeq::new(0))
        .expect("snapshot");
    assert!(snapshot.is_none(), "no snapshot for nonexistent run");
}

#[test]
fn adversarial_blob_for_nonexistent_digest_returns_none() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let blob = journal.blob([0xAA; 32]).expect("blob");
    assert!(blob.is_none(), "no blob for nonexistent digest");
}

#[test]
fn adversarial_workflow_source_for_wrong_digest_returns_none() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let source = b"data".to_vec();
    let digest_a = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest: digest_a,
        source,
    };
    journal.put_workflow_source(&record).expect("put");
    let digest_b = WorkflowDigest::from_bytes([2; 32]);
    let result = journal.workflow_source(digest_b).expect("get");
    assert!(result.is_none(), "wrong digest must return None");
}

#[test]
fn adversarial_multiple_snapshots_same_run_different_seq_all_retrievable() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9006);
    for seq_val in [0u64, 5, 10] {
        let snap = RunSnapshot {
            run,
            seq: EventSeq::new(seq_val),
            workflow: WorkflowDigest::from_bytes([1; 32]),
            slots: vec![0u8],
            taint: Vec::new(),
        };
        journal.put_snapshot(&snap).expect("put_snapshot");
    }
    for seq_val in [0u64, 5, 10] {
        let loaded = journal.snapshot(run, EventSeq::new(seq_val)).expect("get");
        assert!(
            loaded.is_some(),
            "snapshot at seq {} must exist, got {:?}",
            seq_val,
            loaded
        );
        let snap = loaded.unwrap();
        assert_eq!(snap.run, run);
        assert_eq!(snap.seq, EventSeq::new(seq_val));
    }
}

#[test]
fn adversarial_batch_two_sequential_commits_both_visible() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let source1 = b"first".to_vec();
    let digest1 = WorkflowDigest::from_bytes(blake3::hash(&source1).into());
    let source2 = b"second".to_vec();
    let digest2 = WorkflowDigest::from_bytes(blake3::hash(&source2).into());
    let mut batch1 = journal.batch();
    batch1
        .put_workflow_source(&WorkflowSourceRecord {
            digest: digest1,
            source: source1,
        })
        .expect("put1");
    batch1.commit().expect("commit1");
    let mut batch2 = journal.batch();
    batch2
        .put_workflow_source(&WorkflowSourceRecord {
            digest: digest2,
            source: source2,
        })
        .expect("put2");
    batch2.commit().expect("commit2");
    let ws1 = journal.workflow_source(digest1).expect("get1");
    assert!(
        ws1.is_some(),
        "first workflow source must be visible after second commit"
    );
    assert_eq!(ws1.unwrap().source, b"first".to_vec());
    let ws2 = journal.workflow_source(digest2).expect("get2");
    assert!(
        ws2.is_some(),
        "second workflow source must be visible after second commit"
    );
    assert_eq!(ws2.unwrap().source, b"second".to_vec());
}
