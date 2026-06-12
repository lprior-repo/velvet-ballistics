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
fn batch_builder_round_trips_via_append_strict_batch() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(64);
    let mut builder = BatchBuilder::new();
    builder.push(JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([2; 32]),
    });
    builder.push(JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    });

    journal
        .append_strict_batch(builder.as_slice())
        .expect("journal.append_strict_batch must succeed");
    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 2);
}


#[test]
fn flush_profile_batches_strict_events_into_single_fsync() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = open_store(temp_dir.path()).expect("setup: journal open");
    let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
        return;
    };
    let run = RunId::new(58);
    let strict1 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([6; 32]),
    };
    let strict2 = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(1),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };

    queue
        .enqueue_strict(strict1.clone())
        .expect("queue.enqueue_strict must succeed");
    queue
        .enqueue_strict(strict2.clone())
        .expect("queue.enqueue_strict must succeed");
    let report = flush_profile(&queue, &journal);

    let report = report.expect("flush_profile should succeed");
    assert_eq!(report.drained, 2);
    assert_eq!(report.written, 2);
    let events = read_run_events(&journal, run);
    let events = events.expect("read_run_events should succeed");
    assert_eq!(events, vec![strict1, strict2]);
}


#[test]
fn write_batch_commits_cross_keyspace_atomically() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let source_bytes = b"test workflow".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
    let run = RunId::new(42);

    let mut batch = journal.batch();
    batch
        .put_workflow_source(&WorkflowSourceRecord {
            digest,
            source: source_bytes,
        })
        .expect("put_workflow_source must succeed");
    batch
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(7),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 1234,
        })
        .expect("put_run_header must succeed");
    batch.commit().expect("batch.commit must succeed");

    let source = journal
        .workflow_source(digest)
        .expect("workflow source roundtrip");
    assert!(source.is_some());
    assert_eq!(source.unwrap().source, b"test workflow".to_vec());

    let header = journal.run_header(run).expect("run header roundtrip");
    assert!(header.is_some());
    assert_eq!(header.unwrap().run, run);
}


#[test]
fn write_batch_strict_commits_with_durability() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let blob_bytes = b"blob data".to_vec();
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let mut batch = journal.batch().strict();
    batch
        .put_blob(&BlobRecord {
            digest,
            bytes: blob_bytes,
        })
        .expect("action must succeed");
    batch.commit().expect("batch.commit must succeed");

    let blob = journal.blob(digest).expect("blob roundtrip");
    assert!(blob.is_some());
    assert_eq!(blob.unwrap().bytes, b"blob data".to_vec());
}


#[test]
fn write_batch_appends_events_and_indexes() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(99);
    let workflow = WorkflowId::new(5);
    let action = ActionId::new(3);
    let step = StepIdx::new(2);

    let mut batch = journal.batch();
    batch
        .append_event(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        })
        .expect("action must succeed");
    batch
        .put_workflow_index(workflow, run)
        .expect("batch.put_workflow_index must succeed");
    batch
        .put_action_index(action, run, step)
        .expect("batch.put_action_index must succeed");
    batch
        .put_status_index(IndexStatusState::Submitted, 5678, run)
        .expect("batch.put_status_index must succeed");
    batch.commit().expect("batch.commit must succeed");

    let events = journal.events_for_run(run);
    let events = events.expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
}


#[test]
fn write_batch_empty_commit_succeeds() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let batch = journal.batch();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
    batch.commit().expect("batch.commit must succeed");
}


#[test]
fn write_batch_is_empty_after_construction() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let batch = journal.batch();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
}


#[test]
fn write_batch_len_tracks_operations() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let source = b"a".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let mut batch = journal.batch();
    batch
        .put_workflow_source(&WorkflowSourceRecord { digest, source })
        .expect("action must succeed");
    assert_eq!(batch.len(), 1);
    assert!(!batch.is_empty());

    let compiled = crate::accepted_compiled_ir_record_for_test(b"ir".to_vec());
    batch
        .put_compiled_ir(&compiled)
        .expect("action must succeed");
    assert_eq!(batch.len(), 2);
}
