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
fn prefix_constants_have_expected_values() {
    // Given the prefix constants
    // When inspected
    // Then they match the contract values
    assert_eq!(PREFIX_WORKFLOW_SOURCE, 0x01);
    assert_eq!(PREFIX_COMPILED_IR, 0x02);
    assert_eq!(PREFIX_RUN_HEADER, 0x10);
    assert_eq!(PREFIX_RUN_EVENT, 0x11);
    assert_eq!(PREFIX_RUN_SNAPSHOT, 0x12);
    assert_eq!(PREFIX_BLOB, 0x20);
    assert_eq!(PREFIX_INDEX_STATUS, 0x30);
    assert_eq!(PREFIX_INDEX_WORKFLOW, 0x31);
    assert_eq!(PREFIX_INDEX_ACTION, 0x32);
}

#[test]
fn max_payload_constants_are_sensible() {
    // Given the max payload constants
    // When inspected
    // Then they are non-zero and in reasonable ranges
    assert!(MAX_JOURNAL_EVENT_PAYLOAD_BYTES > 0);
    assert!(MAX_WORKFLOW_SOURCE_BYTES > 0);
    assert!(MAX_COMPILED_IR_BYTES > 0);
    assert!(MAX_RUN_HEADER_BYTES > 0);
    assert!(MAX_SNAPSHOT_BYTES > 0);
    assert!(MAX_BLOB_BYTES > 0);
}

#[test]
fn validate_replayed_event_accepts_matching_run_and_seq() {
    // Given an event with run 42, seq 5
    // When validate_replayed_event is called with matching expected run and seq
    // Then it returns Ok (tested indirectly via events_for_run)
    let (_guard, journal) = open_journal();
    let run = RunId::new(42);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    journal
        .append_journaled(&event)
        .expect("journal.append_journaled must succeed");
    let events = journal
        .events_for_run(run)
        .expect("should succeed with contiguous events");
    assert_eq!(events.len(), 1);
}

#[test]
fn journal_reopen_preserves_multiple_event_types() {
    // Given a journal with multiple event types for a run
    // When the journal is closed and reopened
    // Then all events are preserved
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let run = RunId::new(999);

    {
        let journal = FjallJournal::open(temp_dir.path(), None).expect("open should succeed");
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(2),
                slot: vb_core::SlotIdx::new(0),
                value: None,
                extra: None,
                attempt: 1,
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(1),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(6),
                result: vb_core::SlotIdx::new(1),
                attempt: 1,
            },
        ];

        for event in &events {
            journal
                .append_strict(event)
                .expect("journal.append_strict must succeed");
        }
    }

    let journal2 = FjallJournal::open(temp_dir.path(), None).expect("reopen should succeed");
    let events = journal2
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 7);
    assert_eq!(events[0].seq(), EventSeq::new(0));
    assert_eq!(events[6].seq(), EventSeq::new(6));
}

#[test]
fn run_header_stores_all_fields_correctly() {
    // Given a RunHeaderRecord with specific field values
    // When stored and retrieved
    // Then all fields match exactly
    let (_guard, journal) = open_journal();
    let record = RunHeaderRecord {
        run: RunId::new(42),
        workflow_id: WorkflowId::new(7),
        compiled_digest: test_digest(99),
        status: 3,
        accepted_at_ms: 1700000000,
    };
    journal
        .put_run_header(&record)
        .expect("journal.put_run_header must succeed");
    let retrieved = journal
        .run_header(RunId::new(42))
        .expect("lookup should succeed");
    let Some(found) = retrieved else {
        panic!("expected Some(record)");
    };
    assert_eq!(found.run, record.run);
    assert_eq!(found.workflow_id, record.workflow_id);
    assert_eq!(found.compiled_digest, record.compiled_digest);
    assert_eq!(found.status, record.status);
    assert_eq!(found.accepted_at_ms, record.accepted_at_ms);
}

#[test]
fn journal_stores_and_retrieves_blob_with_zero_bytes() {
    // Given a blob with zero bytes
    // When stored and retrieved
    // Then the record survives with empty bytes
    let (_guard, journal) = open_journal();
    let blob_bytes: Vec<u8> = vec![];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let record = BlobRecord {
        digest,
        bytes: blob_bytes,
    };
    journal
        .put_blob(&record)
        .expect("journal.put_blob must succeed");
    let retrieved = journal.blob(digest).expect("lookup should succeed");
    assert_eq!(retrieved, Some(record));
}

#[test]
fn workflow_source_stores_and_retrieves_empty_source() {
    // Given a workflow source with zero source bytes
    // When stored and retrieved
    // Then the record survives with empty source
    let (_guard, journal) = open_journal();
    let source: Vec<u8> = vec![];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    journal
        .put_workflow_source(&record)
        .expect("journal.put_workflow_source must succeed");
    let retrieved = journal
        .workflow_source(digest)
        .expect("lookup should succeed");
    assert_eq!(retrieved, Some(record));
}

#[test]
fn encode_decode_roundtrip_for_wait_scheduled_record() {
    // Given a WaitScheduledEvent
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::WaitScheduledEvent {
        run: RunId::new(10),
        seq: EventSeq::new(2),
        step: StepIdx::new(3),
        attempt: 1,
        deadline_ms: 30000,
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::WaitScheduled,
        2,
        &event,
        128,
    )
    .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}
