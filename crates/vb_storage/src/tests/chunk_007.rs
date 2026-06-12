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
fn validate_replayed_event_returns_wrong_run_when_run_id_mismatch() {
    // Given events stored for run 10 and a replay request for run 20
    // When events_for_run is called for run 20 on a journal that only has run 10 events
    // Then no events are returned (no prefix match), producing an empty result
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run_a = RunId::new(10);
    let event = JournalEvent::RunAccepted {
        run: run_a,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    journal
        .append_journaled(&event)
        .expect("journal.append_journaled must succeed");

    let run_b = RunId::new(20);
    let result = journal.events_for_run(run_b);
    let events = result.expect("events_for_run should succeed for missing run");
    assert!(events.is_empty(), "no events should exist for run_b");
}


#[test]
fn validate_replayed_event_returns_sequence_gap_when_seq_out_of_order() {
    // Given a journal with seq 0 then seq 2 for the same run
    // When events_for_run replays
    // Then it returns SequenceGap with expected=1, actual=2
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(100);
    let event0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    journal
        .append_journaled(&event0)
        .expect("journal.append_journaled must succeed");

    // Manually insert an event at seq 2 (skipping seq 1)
    let event2 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(2),
        step: StepIdx::new(0),
        attempt: 1,
    };
    journal
        .append_journaled(&event2)
        .expect("journal.append_journaled must succeed");

    let result = journal.events_for_run(run);
    let Err(JournalError::SequenceGap { expected, actual }) = result else {
        panic!("expected SequenceGap, got {:?}", result);
    };
    assert_eq!(expected, EventSeq::new(1));
    assert_eq!(actual, EventSeq::new(2));
}


#[test]
fn next_seq_returns_sequence_overflow_at_max() {
    // Given EventSeq at u64::MAX
    // When the next sequence is computed
    // Then it returns SequenceOverflow
    let seq = EventSeq::new(u64::MAX);
    let result = seq.get().checked_add(1).map(EventSeq::new);
    assert!(result.is_none());
}


#[test]
fn duplicate_event_returns_exact_run_and_seq() {
    // Given a journal with a RunAccepted event for run 42, seq 7
    // When the same event is appended again
    // Then DuplicateEvent is returned with run=42, seq=7
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let event = JournalEvent::RunAccepted {
        run: RunId::new(42),
        seq: EventSeq::new(7),
        workflow: WorkflowDigest::from_bytes([3; 32]),
    };
    journal
        .append_journaled(&event)
        .expect("journal.append_journaled must succeed");

    let result = journal.append_journaled(&event);
    let Err(JournalError::DuplicateEvent { run, seq }) = result else {
        panic!("expected DuplicateEvent, got {:?}", result);
    };
    assert_eq!(run, RunId::new(42));
    assert_eq!(seq, EventSeq::new(7));
}


#[test]
fn decode_record_returns_migration_required_for_old_schema() {
    // Given an encoded record with schema version set to 0
    // When decode_record is called
    // Then it returns MigrationRequired with from=0, to=1
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
    // Patch schema version at offset 4..6 to 0
    encoded[4] = 0;
    encoded[5] = 0;
    // Recompute CRC
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::MigrationRequired { from, to }) = result else {
        panic!("expected MigrationRequired, got {:?}", result);
    };
    assert_eq!(from, 0);
    assert_eq!(to, 1);
}


#[test]
fn decode_record_returns_unsupported_schema_version_for_future() {
    // Given an encoded record with schema version 99
    // When decode_record is called
    // Then it returns UnsupportedSchemaVersion with version=99
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
    encoded[4] = 99;
    encoded[5] = 0;
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::UnsupportedSchemaVersion { version }) = result else {
        panic!("expected UnsupportedSchemaVersion, got {:?}", result);
    };
    assert_eq!(version, 99);
}


#[test]
fn decode_record_returns_unknown_record_kind_for_invalid_kind() {
    // Given an encoded record with kind patched to 200
    // When decode_record is called
    // Then it returns UnknownRecordKind with kind=200
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
    // Patch kind at offset 6..8 to 200
    let kind_bytes = 200u16.to_le_bytes();
    encoded[6] = kind_bytes[0];
    encoded[7] = kind_bytes[1];
    // Recompute CRC
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::UnknownRecordKind { kind }) = result else {
        panic!("expected UnknownRecordKind, got {:?}", result);
    };
    assert_eq!(kind, 200);
}
