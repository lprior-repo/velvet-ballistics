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
fn batch_commit_with_header_and_events_cross_keyspace() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9003);
    let digest = WorkflowDigest::from_bytes([0xCD; 32]);
    let mut batch = journal.batch();
    batch
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 555,
        })
        .expect("put_run_header must succeed");
    batch
        .append_event(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        })
        .expect("append_event must succeed");
    batch.commit().expect("commit must succeed");
    let header = journal.run_header(run).expect("run_header must succeed");
    assert!(
        header.is_some(),
        "header must be present after cross-keyspace batch commit, got {:?}",
        header
    );
    let header_record = header.unwrap();
    assert_eq!(header_record.run, run);
    assert_eq!(header_record.workflow_id, WorkflowId::new(1));
    assert_eq!(header_record.compiled_digest, digest);
    assert_eq!(header_record.status, 1);
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1, "event must be present");
    assert!(
        matches!(&events[0], JournalEvent::RunAccepted { run: r, .. } if *r == run),
        "replayed event must be RunAccepted for run {:?}",
        run
    );
}

#[test]
fn batch_strict_commit_all_persisted_durably() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let path = temp_dir.path().to_path_buf();
    let ws_bytes = b"strict_ws".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&ws_bytes).into());
    let compiled = crate::accepted_compiled_ir_record_for_test(b"strict_ir".to_vec());
    let blob_bytes = b"strict_blob".to_vec();
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    {
        let journal = FjallJournal::open(&path, None).expect("setup: journal open");
        let mut batch = journal.batch().strict();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest,
                source: ws_bytes,
            })
            .expect("put_workflow_source must succeed");
        batch
            .put_compiled_ir(&compiled)
            .expect("put_compiled_ir must succeed");
        batch
            .put_blob(&BlobRecord {
                digest: blob_digest,
                bytes: blob_bytes,
            })
            .expect("put_blob must succeed");
        batch.commit().expect("commit must succeed");
    }
    let reopened = FjallJournal::open(&path, None).expect("reopen must succeed");
    let ws = reopened
        .workflow_source(digest)
        .expect("workflow_source must succeed");
    assert_eq!(ws.unwrap().source, b"strict_ws".to_vec());
    let ir = reopened
        .compiled_ir(compiled.digest)
        .expect("compiled_ir must succeed");
    assert_eq!(ir.unwrap(), compiled);
    let bl = reopened.blob(blob_digest).expect("blob must succeed");
    assert_eq!(bl.unwrap().bytes, b"strict_blob".to_vec());
}

#[test]
fn batch_empty_strict_commit_succeeds() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let batch = journal.batch().strict();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
    batch
        .commit()
        .expect("empty strict batch commit must succeed");
}

#[test]
fn batch_commit_after_multiple_puts_persists_all() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let ws_bytes = b"ws".to_vec();
    let digest_1 = WorkflowDigest::from_bytes(blake3::hash(&ws_bytes).into());
    let compiled = crate::accepted_compiled_ir_record_for_test(b"ir".to_vec());
    let blob_bytes = b"blob".to_vec();
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let run = RunId::new(9005);
    let mut batch = journal.batch();
    batch
        .put_workflow_source(&WorkflowSourceRecord {
            digest: digest_1,
            source: ws_bytes,
        })
        .expect("put 1 must succeed");
    batch
        .put_compiled_ir(&compiled)
        .expect("put 2 must succeed");
    batch
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest_1,
            status: 1,
            accepted_at_ms: 100,
        })
        .expect("put 3 must succeed");
    batch
        .put_blob(&BlobRecord {
            digest: blob_digest,
            bytes: blob_bytes,
        })
        .expect("put 4 must succeed");
    batch
        .put_snapshot(&RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: digest_1,
            slots: vec![42],
            taint: Vec::new(),
        })
        .expect("put 5 must succeed");
    batch.commit().expect("commit must succeed");
    let ws = journal.workflow_source(digest_1).expect("ws");
    assert!(
        ws.is_some(),
        "workflow source must be present after multi-put batch commit"
    );
    assert_eq!(ws.unwrap().source, b"ws".to_vec());
    let ir = journal.compiled_ir(compiled.digest).expect("ir");
    assert!(
        ir.is_some(),
        "compiled IR must be present after multi-put batch commit"
    );
    assert_eq!(ir.unwrap(), compiled);
    let rh = journal.run_header(run).expect("rh");
    assert!(rh.is_some(), "run header must be present after multi-put batch commit");
    assert_eq!(rh.unwrap().run, run);
    let bl = journal.blob(blob_digest).expect("bl");
    assert!(
        bl.is_some(),
        "blob must be present after multi-put batch commit"
    );
    assert_eq!(bl.unwrap().bytes, b"blob".to_vec());
    let sn = journal
        .snapshot(run, EventSeq::new(0))
        .expect("sn");
    assert!(
        sn.is_some(),
        "snapshot must be present after multi-put batch commit"
    );
    assert_eq!(sn.unwrap().run, run);
}

#[test]
fn journal_events_for_run_after_batch_commit_matches_input() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9006);
    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
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
        result: SlotIdx::new(1),
        attempt: 1,
    };
    let mut batch = journal.batch();
    batch.append_event(&e0).expect("append 0 must succeed");
    batch.append_event(&e1).expect("append 1 must succeed");
    batch.append_event(&e2).expect("append 2 must succeed");
    batch.commit().expect("commit must succeed");
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events,
        vec![e0, e1, e2],
        "replayed events must match input exactly"
    );
}

#[test]
fn journal_workflow_source_after_batch_commit_matches_input() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let source = b"exact_bytes_source".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    let mut batch = journal.batch();
    batch
        .put_workflow_source(&record)
        .expect("put must succeed");
    batch.commit().expect("commit must succeed");
    let found = journal
        .workflow_source(digest)
        .expect("lookup must succeed");
    let found_record = found.expect("record must exist");
    assert_eq!(
        found_record.source,
        b"exact_bytes_source".to_vec(),
        "source bytes must match exactly"
    );
    assert_eq!(found_record.digest, digest, "digest must match");
}

#[test]
fn journal_compiled_ir_after_batch_commit_matches_input() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let record = crate::accepted_compiled_ir_record_for_test(b"exact_ir_bytes".to_vec());
    let digest = record.digest;
    let mut batch = journal.batch();
    batch.put_compiled_ir(&record).expect("put must succeed");
    batch.commit().expect("commit must succeed");
    let found = journal.compiled_ir(digest).expect("lookup must succeed");
    let found_record = found.expect("record must exist");
    assert_eq!(found_record, record, "IR record must match exactly");
}
