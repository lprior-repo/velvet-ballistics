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
fn adversarial_read_events_with_sequence_gap_returns_exact_gap() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let run = RunId::new(777);
    assert!(
        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1)
            })
            .is_ok()
    );
    assert!(
        journal
            .append_journaled(&JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(5),
                result: vb_core::SlotIdx::new(0),
                attempt: 1,
            })
            .is_ok()
    );
    let Err(JournalError::SequenceGap { expected, actual }) = journal.events_for_run(run)
    else {
        panic!("expected SequenceGap")
    };
    assert_eq!(expected, EventSeq::new(1));
    assert_eq!(actual, EventSeq::new(5));
}


// =========================================================================
// Section: Adversarial Blob / Snapshot / Size Boundary Tests
// =========================================================================

#[test]
fn adversarial_put_blob_exceeding_max_returns_payload_too_large() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let bytes = vec![0u8; (MAX_BLOB_BYTES as usize).saturating_add(1)];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
    let record = BlobRecord { digest, bytes };
    assert!(matches!(
        journal.put_blob(&record),
        Err(JournalError::PayloadTooLarge { .. })
    ));
}


#[test]
fn adversarial_blob_zero_length_round_trips() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let bytes: Vec<u8> = vec![];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
    let record = BlobRecord {
        digest,
        bytes: bytes.clone(),
    };
    journal
        .put_blob(&record)
        .expect("journal.put_blob must succeed");
    assert_eq!(journal.blob(digest).expect("ok"), Some(record));
}


#[test]
fn adversarial_snapshot_exceeding_max_returns_payload_too_large() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let snap = RunSnapshot {
        run: RunId::new(888),
        seq: EventSeq::new(0),
        workflow: test_digest(1),
        slots: vec![0u8; (MAX_SNAPSHOT_BYTES as usize).saturating_add(1)],
        taint: Vec::new(),
    };
    assert!(matches!(
        journal.put_snapshot(&snap),
        Err(JournalError::PayloadTooLarge { .. })
    ));
}


#[test]
fn adversarial_snapshot_corrupt_magic_returns_bad_magic() {
    let snap = RunSnapshot {
        run: RunId::new(889),
        seq: EventSeq::new(0),
        workflow: test_digest(1),
        slots: vec![1, 2, 3],
        taint: Vec::new(),
    };
    let mut enc = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        snap.seq.get(),
        &snap,
        MAX_SNAPSHOT_BYTES,
    )
    .expect("ok");
    if let Some(b) = enc.get_mut(0) {
        *b ^= 0xFF;
    }
    assert!(matches!(
        decode_record::<RunSnapshot>(&enc, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES),
        Err(JournalError::BadMagic { .. })
    ));
}


#[test]
fn adversarial_workflow_source_exceeding_max_returns_payload_too_large() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let source = vec![0u8; (MAX_WORKFLOW_SOURCE_BYTES as usize).saturating_add(1)];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    assert!(matches!(
        journal.put_workflow_source(&record),
        Err(JournalError::PayloadTooLarge { .. })
    ));
}


#[test]
fn adversarial_compiled_ir_exceeding_max_returns_payload_too_large() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let record = CompiledIrRecord {
        digest: test_digest(0xCC),
        ir: vec![0u8; (MAX_COMPILED_IR_BYTES as usize).saturating_add(1)],
        ..Default::default()
    };
    assert!(matches!(
        journal.put_compiled_ir(&record),
        Err(JournalError::PayloadTooLarge { .. })
    ));
}


// =========================================================================
// Section: Adversarial Schema Migration Tests
// =========================================================================

#[test]
fn adversarial_schema_migration_from_zero_exact_fields() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(11),
        seq: EventSeq::new(0),
        workflow: test_digest(11),
    };
    let encoded =
        encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &0u16.to_le_bytes());
    let Err(JournalError::MigrationRequired { from, to }) =
        decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
    else {
        panic!("expected MigrationRequired")
    };
    assert_eq!(from, 0);
    assert_eq!(to, CURRENT_SCHEMA_VERSION);
}


#[test]
fn adversarial_schema_future_version_max_unsupported() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(12),
        seq: EventSeq::new(0),
        workflow: test_digest(12),
    };
    let encoded =
        encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &u16::MAX.to_le_bytes());
    let Err(JournalError::UnsupportedSchemaVersion { version }) =
        decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
    else {
        panic!("expected UnsupportedSchemaVersion")
    };
    assert_eq!(version, u16::MAX);
}


// =========================================================================
// Section: Adversarial Queue Tests
// =========================================================================

#[test]
fn adversarial_queue_zero_capacity_returns_queue_capacity() {
    assert!(matches!(
        JournalWriterQueue::new(0, 1, StorageLimits::DEFAULT),
        Err(JournalError::QueueCapacity)
    ));
}


#[test]
fn adversarial_queue_zero_batch_returns_queue_capacity() {
    assert!(matches!(
        JournalWriterQueue::new(1, 0, StorageLimits::DEFAULT),
        Err(JournalError::QueueCapacity)
    ));
}


#[test]
fn adversarial_queue_full_returns_queue_full() {
    let queue = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT).expect("q");
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    queue
        .enqueue_journaled(event.clone())
        .expect("queue.enqueue_journaled must succeed");
    assert!(matches!(
        queue.enqueue_journaled(event),
        Err(JournalError::QueueFull)
    ));
}
