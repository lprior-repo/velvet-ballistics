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
fn builder_build_produces_correct_record_count() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(8004);
    let mut builder = BatchBuilder::new();
    builder.push(JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    });
    builder.push(JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    });
    builder.push(JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(2),
        result: SlotIdx::new(0),
        attempt: 1,
    });
    assert_eq!(builder.len(), 3);
    journal
        .append_strict_batch(builder.as_slice())
        .expect("append_strict_batch must succeed");
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 3, "three events must be stored");
}


// --- Batch state tracking (tests 41-44) ---

#[test]
fn batch_initial_len_is_zero() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let batch = journal.batch();
    assert_eq!(batch.len(), 0, "new batch must have len 0");
    assert!(batch.is_empty(), "new batch must be empty");
}


#[test]
fn batch_len_increments_per_put() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let mut batch = journal.batch();
    let source = b"a".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    batch
        .put_workflow_source(&WorkflowSourceRecord { digest, source })
        .expect("put 1 must succeed");
    assert_eq!(batch.len(), 1, "batch must have len 1 after first put");
    let compiled = crate::accepted_compiled_ir_record_for_test(b"ir".to_vec());
    batch
        .put_compiled_ir(&compiled)
        .expect("put 2 must succeed");
    assert_eq!(batch.len(), 2, "batch must have len 2 after second put");
    batch
        .put_run_header(&RunHeaderRecord {
            run: RunId::new(9001),
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 0,
            accepted_at_ms: 0,
        })
        .expect("put 3 must succeed");
    assert_eq!(batch.len(), 3, "batch must have len 3 after third put");
}


#[test]
fn batch_len_resets_after_commit() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let mut batch = journal.batch();
    let source = b"data".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    batch
        .put_workflow_source(&WorkflowSourceRecord { digest, source })
        .expect("put must succeed");
    assert_eq!(batch.len(), 1, "batch must have 1 operation before commit");
    batch.commit().expect("commit must succeed");
    let fresh_batch = journal.batch();
    assert_eq!(
        fresh_batch.len(),
        0,
        "new batch after commit must start at 0"
    );
}


#[test]
fn batch_put_snapshot_increments_len() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let mut batch = journal.batch();
    assert_eq!(batch.len(), 0);
    let snapshot = RunSnapshot {
        run: RunId::new(9002),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0x43; 32]),
        slots: vec![1, 2],
        taint: Vec::new(),
    };
    batch
        .put_snapshot(&snapshot)
        .expect("put_snapshot must succeed");
    assert_eq!(batch.len(), 1, "batch len must be 1 after put_snapshot");
}


// --- Envelope validation (tests 45-47) ---

#[test]
fn decode_valid_envelope_produces_exact_record() {
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0xDD; 32]),
        source: b"exact_match".to_vec(),
    };
    let encoded = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &record,
        MAX_WORKFLOW_SOURCE_BYTES,
    )
    .expect("encode must succeed");
    let (envelope, decoded) = decode_record::<WorkflowSourceRecord>(
        &encoded,
        MAGIC_WORKFLOW_SOURCE,
        MAX_WORKFLOW_SOURCE_BYTES,
    )
    .expect("decode must succeed");
    assert_eq!(envelope.magic, MAGIC_WORKFLOW_SOURCE);
    assert_eq!(envelope.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(envelope.record_kind, RecordKind::WorkflowSource.id());
    assert_eq!(
        decoded, record,
        "decoded record must exactly match original"
    );
}


#[test]
fn envelope_magic_matches_expected_constant() {
    assert_eq!(MAGIC_WORKFLOW_SOURCE, 0x5642_5352, "VBSR in ASCII hex");
    assert_eq!(MAGIC_COMPILED_ARTIFACT, 0x5642_4952, "VBIR in ASCII hex");
    assert_eq!(MAGIC_JOURNAL_EVENT, 0x5642_4A45, "VBJE in ASCII hex");
    assert_eq!(MAGIC_SNAPSHOT, 0x5642_534E, "VBSN in ASCII hex");
    assert_eq!(MAGIC_BLOB, 0x5642_424C, "VBBL in ASCII hex");
    assert_eq!(MAGIC_IPC_FRAME, 0x5642_4C54, "VBLT in ASCII hex");
    assert_eq!(MAGIC_INDEX_RECORD, 0x5642_4958, "VBIX in ASCII hex");
}


#[test]
fn envelope_header_len_is_fixed_at_60() {
    assert_eq!(RECORD_HEADER_LEN, 60, "header length must be exactly 60");
    assert_eq!(RECORD_HEADER_BYTES, 60, "header bytes constant must be 60");
    let header = encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        b"payload",
        128,
    )
    .expect("encode_record_header must succeed");
    assert_eq!(header.len(), 60, "encoded header must be exactly 60 bytes");
}


// --- Cross-keyspace atomicity (tests 48-60) ---

#[test]
fn batch_atomic_all_or_nothing_workflow_source_and_ir() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let source_bytes = b"atomic_source".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
    let compiled = crate::accepted_compiled_ir_record_for_test(b"atomic_ir".to_vec());
    let mut batch = journal.batch();
    batch
        .put_workflow_source(&WorkflowSourceRecord {
            digest,
            source: source_bytes,
        })
        .expect("put_workflow_source must succeed");
    batch
        .put_compiled_ir(&compiled)
        .expect("put_compiled_ir must succeed");
    batch.commit().expect("commit must succeed");
    let source = journal
        .workflow_source(digest)
        .expect("workflow_source must succeed");
    let ir = journal
        .compiled_ir(compiled.digest)
        .expect("compiled_ir must succeed");
    assert!(
        source.is_some(),
        "source must be present after atomic commit"
    );
    assert!(ir.is_some(), "IR must be present after atomic commit");
    assert_eq!(source.unwrap().source, b"atomic_source".to_vec());
    assert_eq!(ir.unwrap(), compiled);
}
