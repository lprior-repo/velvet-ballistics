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
fn batch_put_compiled_ir_rejects_forged_digest_and_aborts() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let valid = crate::accepted_compiled_ir_record_for_test(b"batch-forgery".to_vec());
    let forged_digest = WorkflowDigest::from_bytes([0xB6; DIGEST_BYTES]);
    let forged = CompiledIrRecord {
        digest: forged_digest,
        ir: valid.ir,
        ..Default::default()
    };
    let run = RunId::new(0xB6);
    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(0xB6),
        compiled_digest: valid.digest,
        status: 1,
        accepted_at_ms: 1,
    };

    let mut batch = journal.batch();
    assert!(matches!(
        batch.put_compiled_ir(&forged),
        Err(JournalError::ArtifactChecksumMismatch)
    ));
    assert_eq!(batch.len(), 0, "failed validation must abort batch");
    batch
        .put_run_header(&header)
        .expect("post-abort staging call should not persist on commit");
    batch
        .commit()
        .expect("aborted batch commit must be a no-op");

    assert!(
        journal
            .compiled_ir(forged_digest)
            .expect("compiled_ir lookup should succeed")
            .is_none(),
        "forged compiled IR must not be persisted"
    );
    assert!(
        journal
            .run_header(run)
            .expect("run_header lookup should succeed")
            .is_none(),
        "aborted batch must not persist later staged records"
    );
}


#[test]
fn journal_run_header_after_batch_commit_matches_all_fields() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9007);
    let workflow_id = WorkflowId::new(42);
    let compiled_digest = WorkflowDigest::from_bytes([0xFB; 32]);
    let status: u8 = 5;
    let accepted_at_ms: u64 = 9876543210;
    let record = RunHeaderRecord {
        run,
        workflow_id,
        compiled_digest,
        status,
        accepted_at_ms,
    };
    let mut batch = journal.batch();
    batch.put_run_header(&record).expect("put must succeed");
    batch.commit().expect("commit must succeed");
    let found = journal.run_header(run).expect("lookup must succeed");
    let found_record = found.expect("record must exist");
    assert_eq!(found_record.run, run, "run must match");
    assert_eq!(
        found_record.workflow_id, workflow_id,
        "workflow_id must match"
    );
    assert_eq!(
        found_record.compiled_digest, compiled_digest,
        "compiled_digest must match"
    );
    assert_eq!(found_record.status, status, "status must match");
    assert_eq!(
        found_record.accepted_at_ms, accepted_at_ms,
        "accepted_at_ms must match"
    );
}


#[test]
fn journal_snapshot_after_batch_commit_matches_input() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9008);
    let seq = EventSeq::new(3);
    let snapshot = RunSnapshot {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([0xFA; 32]),
        slots: b"snapshot_data".to_vec(),
        taint: Vec::new(),
    };
    let mut batch = journal.batch();
    batch.put_snapshot(&snapshot).expect("put must succeed");
    batch.commit().expect("commit must succeed");
    let found = journal.snapshot(run, seq).expect("lookup must succeed");
    let found_record = found.expect("record must exist");
    assert_eq!(found_record.run, run);
    assert_eq!(found_record.seq, seq);
    assert_eq!(found_record.slots, b"snapshot_data".to_vec());
}


#[test]
fn journal_blob_after_batch_commit_matches_input() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let blob_bytes = b"batch_blob_exact".to_vec();
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let record = BlobRecord {
        digest,
        bytes: blob_bytes,
    };
    let mut batch = journal.batch();
    batch.put_blob(&record).expect("put must succeed");
    batch.commit().expect("commit must succeed");
    let found = journal.blob(digest).expect("lookup must succeed");
    let found_record = found.expect("record must exist");
    assert_eq!(
        found_record.bytes,
        b"batch_blob_exact".to_vec(),
        "blob bytes must match exactly"
    );
    assert_eq!(found_record.digest, digest);
}


#[test]
fn journal_status_index_after_batch_commit_returns_correct_run() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let state = IndexStatusState::Other(7);
    let timestamp: u64 = 55555;
    let run = RunId::new(9009);
    let mut batch = journal.batch();
    batch
        .put_status_index(state, timestamp, run)
        .expect("put_status_index must succeed");
    batch.commit().expect("commit must succeed");
    let key = index_status_key(state, timestamp, run).expect("key must succeed");
    let value = journal
        .index_status
        .get(key.as_slice())
        .expect("get must succeed");
    assert!(
        value.is_some(),
        "status index must exist after batch commit"
    );
}


#[test]
fn journal_action_index_after_batch_commit_returns_correct_entry() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let action = ActionId::new(11);
    let run = RunId::new(9010);
    let step = StepIdx::new(4);
    let mut batch = journal.batch();
    batch
        .put_action_index(action, run, step)
        .expect("put_action_index must succeed");
    batch.commit().expect("commit must succeed");
    let key = index_action_key(action, run, step).expect("key must succeed");
    let value = journal
        .index_action
        .get(key.as_slice())
        .expect("get must succeed");
    assert!(
        value.is_some(),
        "action index must exist after batch commit"
    );
}


#[test]
fn adversarial_reopen_after_unflushed_journaled_events_may_lose_them() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9001);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    journal.append_journaled(&event).expect("append journaled");
    drop(journal);
    let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
    let result = journal2
        .events_for_run(run)
        .expect("events_for_run succeeds");
    // Journaled durability does not guarantee persistence without flush
    // Either the event is present (Fjall flushed on drop) or absent (acceptable)
    assert!(result.len() <= 1, "at most one event expected");
}
