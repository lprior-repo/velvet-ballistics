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
fn adversarial_snapshot_with_empty_slots_roundtrips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9010);
    let snap = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
        slots: vec![],
        taint: Vec::new(),
    };
    journal.put_snapshot(&snap).expect("put");
    let loaded = journal
        .snapshot(run, EventSeq::new(0))
        .expect("get")
        .expect("must exist");
    assert_eq!(loaded.slots.len(), 0);
    assert_eq!(loaded.run, run);
}

#[test]
fn adversarial_blob_with_single_byte_roundtrips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let bytes = vec![0xFF];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
    let record = BlobRecord {
        digest,
        bytes: bytes.clone(),
    };
    journal.put_blob(&record).expect("put");
    let loaded = journal.blob(digest).expect("get").expect("must exist");
    assert_eq!(loaded.bytes, bytes);
}

#[test]
fn adversarial_workflow_source_with_empty_bytes_roundtrips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let source: Vec<u8> = vec![];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    journal.put_workflow_source(&record).expect("put");
    let loaded = journal
        .workflow_source(digest)
        .expect("get")
        .expect("must exist");
    assert_eq!(loaded.source, vec![]);
}

#[test]
fn adversarial_run_header_with_max_run_id_roundtrips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(u64::MAX);
    let digest = WorkflowDigest::from_bytes([9; 32]);
    let record = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(u32::MAX),
        compiled_digest: digest,
        status: 2,
        accepted_at_ms: u64::MAX,
    };
    journal.put_run_header(&record).expect("put");
    let loaded = journal.run_header(run).expect("get").expect("must exist");
    assert_eq!(loaded.run, RunId::new(u64::MAX));
    assert_eq!(loaded.workflow_id, WorkflowId::new(u32::MAX));
    assert_eq!(loaded.accepted_at_ms, u64::MAX);
}

#[test]
fn adversarial_batch_strict_commit_survives_immediate_reopen() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let path = temp_dir.path().to_path_buf();
    let journal = FjallJournal::open(&path, None).expect("setup: journal open");
    let run = RunId::new(9020);
    let digest = WorkflowDigest::from_bytes([11; 32]);
    let mut batch = journal.batch().strict();
    batch
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(3),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 500,
        })
        .expect("put");
    batch.strict().commit().expect("strict commit");
    drop(journal);
    let journal2 = FjallJournal::open(&path, None).expect("reopen");
    let header = journal2.run_header(run).expect("get").expect("must exist");
    assert_eq!(header.run, run);
    assert_eq!(header.status, 1);
}

#[test]
fn adversarial_events_for_run_isolates_run_a_from_run_b() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run_a = RunId::new(100);
    let run_b = RunId::new(200);
    let digest = WorkflowDigest::from_bytes([1; 32]);
    journal
        .append_strict(&JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: digest,
        })
        .expect("append a");
    journal
        .append_strict(&JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: digest,
        })
        .expect("append b");
    journal
        .append_strict(&JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: vb_core::StepIdx::ZERO,
            attempt: 1,
        })
        .expect("append a2");
    let events_a = journal.events_for_run(run_a).expect("events a");
    let events_b = journal.events_for_run(run_b).expect("events b");
    assert_eq!(events_a.len(), 2, "run A should have 2 events");
    assert_eq!(events_b.len(), 1, "run B should have 1 event");
}

#[test]
fn adversarial_run_header_overwrite_replaces_previous() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9030);
    let digest = WorkflowDigest::from_bytes([1; 32]);
    journal
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 100,
        })
        .expect("put first");
    journal
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(2),
            compiled_digest: digest,
            status: 3,
            accepted_at_ms: 200,
        })
        .expect("put second");
    let header = journal.run_header(run).expect("get").expect("exists");
    assert_eq!(header.workflow_id, WorkflowId::new(2));
    assert_eq!(header.status, 3);
    assert_eq!(header.accepted_at_ms, 200);
}

#[test]
fn adversarial_batch_commit_with_5_puts_persists_all() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let source = b"s".to_vec();
    let d1 = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let compiled = crate::accepted_compiled_ir_record_for_test(b"ir".to_vec());
    let run = RunId::new(9050);
    let mut batch = journal.batch();
    batch
        .put_workflow_source(&WorkflowSourceRecord { digest: d1, source })
        .expect("put1");
    batch.put_compiled_ir(&compiled).expect("put2");
    batch
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: d1,
            status: 1,
            accepted_at_ms: 0,
        })
        .expect("put3");
    let blob_bytes = b"b".to_vec();
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    batch
        .put_blob(&BlobRecord {
            digest: blob_digest,
            bytes: blob_bytes,
        })
        .expect("put4");
    batch
        .put_status_index(IndexStatusState::Submitted, 0, run)
        .expect("put5");
    batch.commit().expect("commit");
    let ws = journal.workflow_source(d1).expect("g1");
    assert!(
        ws.is_some(),
        "workflow source must persist after 5-put batch commit"
    );
    assert_eq!(ws.unwrap().source, b"s".to_vec());
    let ir = journal.compiled_ir(compiled.digest).expect("g2");
    assert!(
        ir.is_some(),
        "compiled IR must persist after 5-put batch commit"
    );
    assert_eq!(ir.unwrap(), compiled);
    let rh = journal.run_header(run).expect("g3");
    assert!(
        rh.is_some(),
        "run header must persist after 5-put batch commit"
    );
    assert_eq!(rh.unwrap().run, run);
    let bl = journal.blob(blob_digest).expect("g4");
    assert!(bl.is_some(), "blob must persist after 5-put batch commit");
    assert_eq!(bl.unwrap().bytes, b"b".to_vec());
}
