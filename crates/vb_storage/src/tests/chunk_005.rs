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
fn flush_profile_wrapper_flushes_queued_events() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = open_store(temp_dir.path()).expect("setup: journal open");
    let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
        return;
    };
    let run = RunId::new(57);
    let journaled = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([5; 32]),
    };
    let strict = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(1),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };

    queue
        .enqueue_journaled(journaled.clone())
        .expect("queue.enqueue_journaled must succeed");
    queue
        .enqueue_strict(strict.clone())
        .expect("queue.enqueue_strict must succeed");
    let report = flush_profile(&queue, &journal);

    let report = report.expect("flush_profile should succeed");
    assert_eq!(report.drained, 2);
    assert_eq!(report.written, 2);
    let events = read_run_events(&journal, run);
    let events = events.expect("read_run_events should succeed");
    assert_eq!(events, vec![journaled, strict]);
}

#[test]
fn replay_returns_contiguous_events_for_run() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(11);
    let accepted = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([4; 32]),
    };
    let finished = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(1),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };

    journal
        .append_journaled(&accepted)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&finished)
        .expect("journal.append_journaled must succeed");

    let replay = journal
        .events_for_run(run)
        .expect("event replay should succeed");
    assert_eq!(replay, vec![accepted, finished]);
}

#[test]
fn decode_rejects_truncated_header() {
    // Given a byte slice shorter than the required 60-byte header
    // When decode_record is called
    // Then it returns UnexpectedEof
    let truncated = [0u8; 30];

    let result = decode_record::<JournalEvent>(&truncated, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(result, Err(JournalError::UnexpectedEof)));
}

#[test]
fn decode_rejects_migration_required_schema() {
    // Given a valid record whose schema version byte is 0 (less than current)
    // When decode_record is called
    // Then it returns MigrationRequired with from=0, to=CURRENT_SCHEMA_VERSION
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
    // Set schema version to 0 (two LE bytes at offset 4..6)
    encoded[4] = 0;
    encoded[5] = 0;
    // Recompute CRC32C for the modified header prefix
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(
        result,
        Err(JournalError::MigrationRequired { from: 0, to: 1 })
    ));
}

#[test]
fn decode_rejects_unsupported_future_schema() {
    // Given a valid record whose schema version byte is 99 (greater than current)
    // When decode_record is called
    // Then it returns UnsupportedSchemaVersion
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
    // Set schema version to 99 (two LE bytes at offset 4..6)
    encoded[4] = 99;
    encoded[5] = 0;
    // Recompute CRC32C for the modified header prefix
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(
        result,
        Err(JournalError::UnsupportedSchemaVersion { version: 99 })
    ));
}

#[test]
fn decode_rejects_record_kind_family_mismatch() {
    // Given a record encoded with MAGIC_JOURNAL_EVENT but a kind outside 10..=23
    // When decode_record is called
    // Then it returns RecordKindFamilyMismatch
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
    // Patch the kind to 1 (WorkflowSource), which is outside 10..=23
    // Kind is at offset 6..8, little-endian
    let kind_bytes = 1u16.to_le_bytes();
    encoded[6] = kind_bytes[0];
    encoded[7] = kind_bytes[1];
    // Recompute CRC32C
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(
        result,
        Err(JournalError::RecordKindFamilyMismatch {
            magic: MAGIC_JOURNAL_EVENT,
            kind: 1
        })
    ));
}
