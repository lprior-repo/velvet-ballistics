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
fn public_wrappers_delegate_to_journal_storage_paths() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = open_store(temp_dir.path()).expect("setup: journal open");
    let run = RunId::new(70);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(1),
        workflow: WorkflowDigest::from_bytes([7; 32]),
    };
    let blob_bytes = vec![1, 2, 3];
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let blob = BlobRecord {
        digest: blob_digest,
        bytes: blob_bytes,
    };
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([7; 32]),
        slots: vec![4, 5, 6],
        taint: Vec::new(),
    };

    append_journal_event(&journal, &event).expect("append_journal_event must succeed");
    journal
        .put_blob(&blob)
        .expect("journal.put_blob must succeed");
    write_snapshot(&journal, &snapshot).expect("write_snapshot must succeed");

    // Snapshot at seq 0 covers events 0..0; event at seq 1 is after snapshot
    let events = read_run_events(&journal, run);
    let events = events.expect("read_run_events should succeed");
    assert_eq!(events, vec![event.clone()]);
    let loaded_blob = read_blob(&journal, blob.digest);
    let loaded_blob = loaded_blob.expect("read_blob should succeed");
    assert_eq!(loaded_blob, Some(blob));
    let loaded_snapshot = journal.snapshot(run, EventSeq::new(0));
    let loaded_snapshot = loaded_snapshot.expect("snapshot lookup should succeed");
    assert_eq!(loaded_snapshot, Some(snapshot));
}

#[test]
fn replay_journal_wrapper_uses_recovery_replay() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = open_store(temp_dir.path()).expect("setup: journal open");
    let run = RunId::new(71);
    let digest = WorkflowDigest::from_bytes([8; 32]);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    let admission = JournalEvent::RunAdmission {
        run,
        seq: EventSeq::new(1),
        artifact_digest: digest,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Relaxed,
    };
    append_journal_event(&journal, &event).expect("append_journal_event must succeed");
    append_journal_event(&journal, &admission).expect("append_journal_event must succeed");

    let mut tracker = ActionReplayTracker::new();
    let replayed = replay_journal(&journal, run, &mut tracker, &[], &[]);

    let replayed = replayed.expect("replay_journal should succeed");
    assert_eq!(replayed, vec![event, admission]);
}

#[test]
fn append_strict_persists_submitted_event() {
    // Given an open journal
    // When append_strict is called with a RunAccepted event
    // Then the event can be retrieved via events_for_run
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(55);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let result = journal.append_strict(&event);
    result.expect("action must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], event);
}

#[test]
fn append_strict_rejects_out_of_order_sequence() {
    // Given an open journal with a seq-0 event
    // When append_strict is called with seq 2 (skipping seq 1)
    // Then events_for_run returns SequenceGap
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(60);
    let event0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    journal
        .append_strict(&event0)
        .expect("journal.append_strict must succeed");

    let event2 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(2),
        step: StepIdx::new(0),
        attempt: 1,
    };
    journal
        .append_strict(&event2)
        .expect("journal.append_strict must succeed");

    let result = journal.events_for_run(run);
    let Err(JournalError::SequenceGap { expected, actual }) = result else {
        panic!("expected SequenceGap, got {:?}", result);
    };
    assert_eq!(expected, EventSeq::new(1));
    assert_eq!(actual, EventSeq::new(2));
}

#[test]
fn persist_strict_flushes_and_reopens_cleanly() {
    // Given an open journal with a persisted event
    // When the journal is closed and reopened
    // Then the same event is visible
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");

    let run = RunId::new(77);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([5; 32]),
    };
    {
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");
    }

    let journal2 = FjallJournal::open(temp_dir.path(), None);
    let journal2 = journal2.expect("journal should reopen cleanly");
    let events = journal2
        .events_for_run(run)
        .expect("events_for_run should succeed after reopen");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], event);
}

#[test]
fn put_workflow_source_stores_and_retrieves() {
    // Given an open journal and a workflow source record
    // When put_workflow_source is called
    // Then the record can be retrieved by digest
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let source = vec![b'h', b'e', b'l', b'l', b'o'];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    journal
        .put_workflow_source(&record)
        .expect("journal.put_workflow_source must succeed");

    let retrieved = journal
        .workflow_source(digest)
        .expect("workflow_source lookup should succeed");
    assert_eq!(retrieved, Some(record));
}

#[test]
fn put_workflow_source_returns_none_for_missing_digest() {
    // Given an open journal with no stored workflow source
    // When workflow_source is called with an arbitrary digest
    // Then it returns None
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let missing = WorkflowDigest::from_bytes([99; 32]);
    let result = journal
        .workflow_source(missing)
        .expect("lookup should succeed");
    assert_eq!(result, None);
}
