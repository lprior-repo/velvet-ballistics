#![forbid(unsafe_code)]
use super::*;

#[test]
fn commit_with_single_event_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(100);
    let event = make_event(run, 0);

    let mut batch = JournalWriteBatch::new(&journal);
    batch.append_event(&event).expect("append should succeed");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let events = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(
        events.len(),
        1,
        "should find 1 event after single-event batch commit"
    );
    assert_eq!(events[0], event);
}

#[test]
fn commit_with_multiple_events_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(200);
    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]),
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
        result: SlotIdx::new(0),
        attempt: 1,
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch.append_event(&e0).expect("append 0");
    batch.append_event(&e1).expect("append 1");
    batch.append_event(&e2).expect("append 2");
    assert_eq!(batch.len(), 3);
    batch.commit().expect("commit should succeed");

    let events = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(
        events.len(),
        3,
        "should find 3 events after multi-event batch"
    );
    assert_eq!(events[0], e0);
    assert_eq!(events[1], e1);
    assert_eq!(events[2], e2);
}

#[test]
fn batch_put_workflow_source_with_valid_digest_commits() {
    let (_temp, journal) = temp_journal();
    let source = b"workflow: batch_test".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_workflow_source(&record)
        .expect("put workflow source");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let loaded = journal.workflow_source(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("workflow source should be found after batch commit");
    };
    assert_eq!(found.source, source);
}

#[test]
fn batch_put_compiled_ir_with_valid_digest_commits() {
    let (_temp, journal) = temp_journal();
    let ir = b"compiled-batch-test".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    let record = CompiledIrRecord {
        digest,
        ir: ir.clone(),
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_compiled_ir(&record).expect("put compiled ir");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let loaded = journal.compiled_ir(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("compiled IR should be found after batch commit");
    };
    assert_eq!(found.ir, ir);
}

#[test]
fn batch_put_run_header_commits_and_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(300);
    let header = make_run_header(run);

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_run_header(&header).expect("put run header");
    batch.commit().expect("commit should succeed");

    let loaded = journal.run_header(run).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("run header should be found after batch commit");
    };
    assert_eq!(found.run, run);
    assert_eq!(found.status, 1);
}

#[test]
fn batch_put_snapshot_commits_and_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(400);
    let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow,
        slots: vec![1, 2, 3],
        taint: vec![0],
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_snapshot(&snapshot).expect("put snapshot");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let loaded = journal
        .snapshot(run, EventSeq::new(5))
        .expect("get should succeed")
        .expect("snapshot should exist");
    assert_eq!(loaded.run, run);
    assert_eq!(loaded.slots, vec![1, 2, 3]);
    assert_eq!(loaded.taint, vec![0]);
}

#[test]
fn batch_put_blob_with_valid_digest_commits() {
    let (_temp, journal) = temp_journal();
    let payload = vec![0xCA, 0xFE];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
    let record = BlobRecord {
        digest,
        bytes: payload.clone(),
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_blob(&record).expect("put blob");
    assert_eq!(batch.len(), 1);
    batch.commit().expect("commit should succeed");

    let loaded = journal.blob(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("blob should be found after batch commit");
    };
    assert_eq!(found.bytes, payload);
}