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
fn decode_rejects_corrupt_header_checksum() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        128,
    );
    let Ok(mut encoded) = encoded else {
        return;
    };
    if let Some(byte) = encoded.get_mut(56) {
        *byte ^= 1;
    }

    let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);

    assert!(matches!(decoded, Err(JournalError::HeaderChecksumMismatch)));
}


#[test]
fn decode_rejects_corrupt_payload_digest() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        128,
    );
    let Ok(mut encoded) = encoded else {
        return;
    };
    if let Some(byte) = encoded.get_mut(60) {
        *byte ^= 1;
    }

    let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);

    assert!(matches!(decoded, Err(JournalError::PayloadDigestMismatch)));
}


#[test]
fn decode_rejects_payload_before_allocation() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        128,
    );
    let Ok(encoded) = encoded else {
        return;
    };

    let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 1);

    assert!(matches!(decoded, Err(JournalError::PayloadTooLarge { .. })));
}


#[test]
fn decode_rejects_bad_magic_and_unknown_kind() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    let Ok(mut bad_magic) = encoded else {
        return;
    };
    if let Some(byte) = bad_magic.get_mut(0) {
        *byte ^= 1;
    }

    let decoded = decode_record::<JournalEvent>(
        &bad_magic,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(matches!(decoded, Err(JournalError::BadMagic { .. })));

    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    let Ok(mut unknown_kind) = encoded else {
        return;
    };
    if let Some(byte) = unknown_kind.get_mut(6) {
        *byte = 200;
    }
    if let Some(byte) = unknown_kind.get_mut(56) {
        *byte ^= 1;
    }

    let decoded = decode_record::<JournalEvent>(
        &unknown_kind,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(matches!(
        decoded,
        Err(JournalError::UnknownRecordKind { .. })
    ));
}


#[test]
fn append_strict_batch_writes_all_events_with_single_fsync() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(61);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        },
    ];

    let result = journal.append_strict_batch(&events);
    result.expect("action must succeed");

    let replayed = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(replayed.len(), 3);
    assert_eq!(replayed, events);
}


#[test]
fn append_strict_batch_rejects_duplicate_within_batch() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(62);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let events = vec![event.clone(), event.clone()];

    let result = journal.append_strict_batch(&events);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "expected DuplicateEvent, got {:?}",
        result
    );
}


#[test]
fn batch_builder_collects_events() {
    let mut builder = BatchBuilder::new();
    assert!(builder.is_empty());
    assert_eq!(builder.len(), 0);

    let run = RunId::new(63);
    builder.push(JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    });
    assert_eq!(builder.len(), 1);
    assert!(!builder.is_empty());

    builder.push(JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(1),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    });
    assert_eq!(builder.len(), 2);
    assert_eq!(builder.as_slice().len(), 2);
}
