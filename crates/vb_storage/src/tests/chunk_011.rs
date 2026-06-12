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
fn put_action_index_stores_and_retrieves() {
    // Given an open journal
    // When put_action_index is called
    // Then no error is returned and the index entry exists
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let result = journal.put_action_index(ActionId::new(1), RunId::new(2), StepIdx::new(3));
    result.expect("action must succeed");
}


#[test]
fn put_status_index_stores_and_retrieves() {
    // Given an open journal
    // When put_status_index is called
    // Then no error is returned
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let result =
        journal.put_status_index(IndexStatusState::Submitted, 1700000000, RunId::new(99));
    result.expect("action must succeed");
}


#[test]
fn put_workflow_index_stores_and_retrieves() {
    // Given an open journal
    // When put_workflow_index is called
    // Then no error is returned
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let result = journal.put_workflow_index(WorkflowId::new(7), RunId::new(8));
    result.expect("action must succeed");
}


#[test]
fn events_for_run_returns_only_events_for_target_run() {
    // Given a journal with events for run 10 and run 20
    // When events_for_run is called for run 10
    // Then only run 10 events are returned
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run_a = RunId::new(10);
    let run_b = RunId::new(20);

    let event_a0 = JournalEvent::RunAccepted {
        run: run_a,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let event_b0 = JournalEvent::RunAccepted {
        run: run_b,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([2; 32]),
    };
    let event_a1 = JournalEvent::StepStarted {
        run: run_a,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };

    journal
        .append_journaled(&event_a0)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&event_b0)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&event_a1)
        .expect("journal.append_journaled must succeed");

    let events_a = journal
        .events_for_run(run_a)
        .expect("events_for_run should succeed");
    assert_eq!(events_a.len(), 2);
    assert_eq!(events_a[0], event_a0);
    assert_eq!(events_a[1], event_a1);

    let events_b = journal
        .events_for_run(run_b)
        .expect("events_for_run should succeed");
    assert_eq!(events_b.len(), 1);
    assert_eq!(events_b[0], event_b0);
}


#[test]
fn event_seq_new_returns_correct_value() {
    // Given EventSeq::new(42)
    // When get is called
    // Then it returns 42
    let seq = EventSeq::new(42);
    assert_eq!(seq.get(), 42);
}


#[test]
fn record_kind_id_returns_correct_wire_ids() {
    // Given each RecordKind variant
    // When id() is called
    // Then it returns the expected wire identifier
    assert_eq!(RecordKind::WorkflowSource.id(), 1);
    assert_eq!(RecordKind::CompiledIr.id(), 2);
    assert_eq!(RecordKind::RunHeader.id(), 3);
    assert_eq!(RecordKind::RunAccepted.id(), 10);
    assert_eq!(RecordKind::StepStarted.id(), 11);
    assert_eq!(RecordKind::SlotWritten.id(), 12);
    assert_eq!(RecordKind::ActionScheduled.id(), 13);
    assert_eq!(RecordKind::ActionCompleted.id(), 14);
    assert_eq!(RecordKind::ActionFailed.id(), 15);
    assert_eq!(RecordKind::WaitScheduled.id(), 16);
    assert_eq!(RecordKind::AskScheduled.id(), 17);
    assert_eq!(RecordKind::AskAnswered.id(), 18);
    assert_eq!(RecordKind::RetryScheduled.id(), 19);
    assert_eq!(RecordKind::StepFailed.id(), 20);
    assert_eq!(RecordKind::RunCancelled.id(), 21);
    assert_eq!(RecordKind::RunFinished.id(), 22);
    assert_eq!(RecordKind::RunFailed.id(), 23);
    assert_eq!(RecordKind::Snapshot.id(), 30);
    assert_eq!(RecordKind::Blob.id(), 40);
    assert_eq!(RecordKind::IndexUpdate.id(), 50);
}


#[test]
fn journal_event_run_id_returns_correct_run() {
    // Given a RunAccepted event for run 42
    // When run_id() is called
    // Then it returns 42
    let event = JournalEvent::RunAccepted {
        run: RunId::new(42),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    assert_eq!(event.run_id(), RunId::new(42));
}


#[test]
fn journal_event_seq_returns_correct_seq() {
    // Given a StepStarted event with seq 7
    // When seq() is called
    // Then it returns EventSeq(7)
    let event = JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(7),
        step: StepIdx::new(0),
        attempt: 1,
    };
    assert_eq!(event.seq(), EventSeq::new(7));
}


#[test]
fn journal_event_record_kind_returns_correct_kind() {
    // Given a RunFinished event
    // When record_kind() is called
    // Then it returns RecordKind::RunFinished
    let event = JournalEvent::RunFinished {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };
    assert_eq!(event.record_kind(), RecordKind::RunFinished);
}


#[test]
fn decode_record_returns_postcard_decode_failed_for_garbage_payload() {
    // Given an encoded record with a valid header but corrupted payload bytes
    // that no longer deserialize correctly
    // When decode_record is called
    // Then it returns PostcardDecodeFailed
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encoding should succeed");
    // Corrupt the payload bytes after the header but not the blake3 digest
    // We need to corrupt and re-hash, so instead we construct a manually
    // crafted header with valid CRC/digest pointing to garbage
    let payload_start = 60;
    if let Some(byte) = encoded.get_mut(payload_start) {
        *byte = 0xFF;
    }
    // Now recompute the blake3 digest in the header
    let payload = &encoded[60..];
    let digest = blake3::hash(payload);
    encoded[24..56].copy_from_slice(digest.as_bytes());
    // Recompute CRC
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(result, Err(JournalError::PostcardDecodeFailed)));
}
