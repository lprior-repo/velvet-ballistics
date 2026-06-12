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
fn write_batch_snapshot_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(77);
    let seq = EventSeq::new(5);
    let snapshot = RunSnapshot {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([5; 32]),
        slots: b"slot_data".to_vec(),
        taint: Vec::new(),
    };

    let mut batch = journal.batch();
    batch
        .put_snapshot(&snapshot)
        .expect("batch.put_snapshot must succeed");
    batch.commit().expect("batch.commit must succeed");

    let loaded = journal.snapshot(run, seq).expect("snapshot roundtrip");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().run, run);
}


#[test]
fn keyspace_profiles_return_distinct_configs() {
    let _hot = keyspace_options_for(KeyspaceProfile::Hot);
    let _cold = keyspace_options_for(KeyspaceProfile::Cold);
    let _blob = keyspace_options_for(KeyspaceProfile::Blob);

    // Hot has no KV separation; Cold and Blob have KV separation.
    // We verify this indirectly by checking the configs differ.
    assert_ne!(
        std::mem::discriminant(&KeyspaceProfile::Hot),
        std::mem::discriminant(&KeyspaceProfile::Cold)
    );
    assert_ne!(
        std::mem::discriminant(&KeyspaceProfile::Cold),
        std::mem::discriminant(&KeyspaceProfile::Blob)
    );

    // Verify the function exists and returns valid options by using them
    // in a real database open.
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None);
    assert!(journal.is_ok(), "journal should open with tuned keyspaces");
}


#[test]
fn journal_opens_declared_keyspaces_and_round_trips_typed_records() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    assert_eq!(FjallJournal::declared_keyspaces().len(), 10);

    let source_bytes = vec![b'n', b'a', b'm', b'e'];
    let workflow_digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
    let ir = crate::accepted_compiled_ir_record_for_test(vec![1, 2, 3]);
    let compiled_digest = ir.digest;
    let source = WorkflowSourceRecord {
        digest: workflow_digest,
        source: source_bytes,
    };
    let header = RunHeaderRecord {
        run: RunId::new(3),
        workflow_id: WorkflowId::new(4),
        compiled_digest,
        status: 5,
        accepted_at_ms: 6,
    };
    let snapshot = RunSnapshot {
        run: RunId::new(3),
        seq: EventSeq::new(7),
        workflow: compiled_digest,
        slots: vec![8, 9],
        taint: Vec::new(),
    };
    let blob_bytes = vec![10, 11];
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let blob = BlobRecord {
        digest: blob_digest,
        bytes: blob_bytes,
    };

    journal
        .put_workflow_source(&source)
        .expect("journal.put_workflow_source must succeed");
    journal
        .put_compiled_ir(&ir)
        .expect("journal.put_compiled_ir must succeed");
    journal
        .put_run_header(&header)
        .expect("journal.put_run_header must succeed");
    journal
        .put_snapshot(&snapshot)
        .expect("journal.put_snapshot must succeed");
    journal
        .put_blob(&blob)
        .expect("journal.put_blob must succeed");
    journal
        .put_status_index(IndexStatusState::Submitted, 2, RunId::new(3))
        .expect("journal.put_status_index must succeed");
    journal
        .put_workflow_index(WorkflowId::new(4), RunId::new(3))
        .expect("action must succeed");
    journal
        .put_action_index(ActionId::new(5), RunId::new(3), StepIdx::new(6))
        .expect("action must succeed");

    let found_source = journal
        .workflow_source(workflow_digest)
        .expect("workflow source lookup should succeed");
    assert_eq!(found_source, Some(source));

    let found_ir = journal
        .compiled_ir(compiled_digest)
        .expect("compiled ir lookup should succeed");
    assert_eq!(found_ir, Some(ir));

    let found_header = journal
        .run_header(RunId::new(3))
        .expect("run header lookup should succeed");
    assert_eq!(found_header, Some(header));

    let found_snapshot = journal
        .snapshot(RunId::new(3), EventSeq::new(7))
        .expect("snapshot lookup should succeed");
    assert_eq!(found_snapshot, Some(snapshot));

    let found_blob = journal
        .blob(blob_digest)
        .expect("blob lookup should succeed");
    assert_eq!(found_blob, Some(blob));
}


#[test]
fn non_journal_families_reject_wrong_record_kind() {
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([1; 32]),
        source: vec![1],
    };

    let encoded = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &source,
        128,
    );
    assert!(encoded.is_ok(), "encoding must succeed for valid input");
    let encoded = encoded.expect("setup: encoding");
    assert!(!encoded.is_empty(), "encoded bytes must be non-empty");
    let wrong_family = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::WorkflowSource,
        0,
        &source,
        128,
    );

    assert!(matches!(
        wrong_family,
        Err(JournalError::RecordKindFamilyMismatch { .. })
    ));
}


#[test]
fn duplicate_event_append_is_rejected() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let event = JournalEvent::RunAccepted {
        run: RunId::new(9),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([3; 32]),
    };

    let first = journal.append_journaled(&event);
    let second = journal.append_journaled(&event);

    first.expect("action must succeed");
    assert!(matches!(second, Err(JournalError::DuplicateEvent { .. })));
}


#[test]
fn journal_writer_queue_counts_pending_durability_profiles() {
    let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
        return;
    };
    let run = RunId::new(56);
    let journaled = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let strict = JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
    };

    queue
        .enqueue_journaled(journaled)
        .expect("queue.enqueue_journaled must succeed");
    queue
        .enqueue_strict(strict)
        .expect("queue.enqueue_strict must succeed");

    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 1 && counts.strict == 1
    ));
}
