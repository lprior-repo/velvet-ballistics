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
fn journal_error_postcard_decode_failed_display() {
    // Given a JournalError::PostcardDecodeFailed
    // When displayed
    // Then the message mentions postcard
    let err = JournalError::PostcardDecodeFailed;
    let msg = format!("{}", err);
    assert!(msg.contains("postcard"));
}


#[test]
fn journal_error_unexpected_eof_display() {
    // Given a JournalError::UnexpectedEof
    // When displayed
    // Then the message mentions end of record
    let err = JournalError::UnexpectedEof;
    let msg = format!("{}", err);
    assert!(msg.contains("end"));
}


#[test]
fn journal_error_payload_digest_mismatch_display() {
    // Given a JournalError::PayloadDigestMismatch
    // When displayed
    // Then the message mentions digest
    let err = JournalError::PayloadDigestMismatch;
    let msg = format!("{}", err);
    assert!(msg.contains("digest"));
}


#[test]
fn record_envelope_fields_match_encoded_values() {
    // Given an encoded event
    // When decoded
    // Then the envelope contains magic, schema_version, record_kind, and sequence
    let event = JournalEvent::RunAccepted {
        run: RunId::new(77),
        seq: EventSeq::new(3),
        workflow: test_digest(5),
    };
    let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 3, &event, 128)
        .expect("encoding should succeed");
    let (envelope, _) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.schema_version, 1);
    assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
    assert_eq!(envelope.sequence, 3);
}


// --- Section 7: RunHeaderRecord Integration Tests ---

#[test]
fn run_header_overwrite_replaces_existing_header() {
    // Given a journal with a stored run header
    // When a new header with the same run id is stored
    // Then the new header replaces the old one
    let (_guard, journal) = open_journal();
    let original = RunHeaderRecord {
        run: RunId::new(1),
        workflow_id: WorkflowId::new(10),
        compiled_digest: test_digest(1),
        status: 0,
        accepted_at_ms: 100,
    };
    let updated = RunHeaderRecord {
        run: RunId::new(1),
        workflow_id: WorkflowId::new(20),
        compiled_digest: test_digest(2),
        status: 1,
        accepted_at_ms: 200,
    };
    journal
        .put_run_header(&original)
        .expect("journal.put_run_header must succeed");
    journal
        .put_run_header(&updated)
        .expect("journal.put_run_header must succeed");

    let retrieved = journal
        .run_header(RunId::new(1))
        .expect("lookup should succeed");
    assert_eq!(retrieved, Some(updated));
}


#[test]
fn multiple_runs_have_independent_events() {
    // Given a journal with 2 events for run 1 and 3 events for run 2
    // When events_for_run is called for each
    // Then each run returns only its own events
    let (_guard, journal) = open_journal();
    let run1 = RunId::new(1);
    let run2 = RunId::new(2);

    let r1_e0 = JournalEvent::RunAccepted {
        run: run1,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    let r1_e1 = JournalEvent::StepStarted {
        run: run1,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    let r2_e0 = JournalEvent::RunAccepted {
        run: run2,
        seq: EventSeq::new(0),
        workflow: test_digest(2),
    };
    let r2_e1 = JournalEvent::StepStarted {
        run: run2,
        seq: EventSeq::new(1),
        step: StepIdx::new(1),
        attempt: 1,
    };
    let r2_e2 = JournalEvent::RunFinished {
        run: run2,
        seq: EventSeq::new(2),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };

    journal
        .append_journaled(&r1_e0)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&r1_e1)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&r2_e0)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&r2_e1)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&r2_e2)
        .expect("journal.append_journaled must succeed");

    let events1 = journal
        .events_for_run(run1)
        .expect("events_for_run run1 should succeed");
    assert_eq!(events1.len(), 2);
    let events2 = journal
        .events_for_run(run2)
        .expect("events_for_run run2 should succeed");
    assert_eq!(events2.len(), 3);
}


#[test]
fn event_seq_ordering_is_correct() {
    // Given two EventSeq values
    // When compared
    // Then ordering follows the inner u64
    assert!(EventSeq::new(0) < EventSeq::new(1));
    assert!(EventSeq::new(100) < EventSeq::new(200));
    assert_eq!(EventSeq::new(5), EventSeq::new(5));
}


#[test]
fn record_kind_all_variants_have_distinct_ids() {
    // Given all RecordKind variants
    // When their ids are collected
    // Then no two variants share an id
    let ids = [
        RecordKind::WorkflowSource.id(),
        RecordKind::CompiledIr.id(),
        RecordKind::RunHeader.id(),
        RecordKind::RunAccepted.id(),
        RecordKind::StepStarted.id(),
        RecordKind::SlotWritten.id(),
        RecordKind::ActionScheduled.id(),
        RecordKind::ActionCompleted.id(),
        RecordKind::ActionFailed.id(),
        RecordKind::WaitScheduled.id(),
        RecordKind::AskScheduled.id(),
        RecordKind::AskAnswered.id(),
        RecordKind::RetryScheduled.id(),
        RecordKind::StepFailed.id(),
        RecordKind::RunCancelled.id(),
        RecordKind::RunFinished.id(),
        RecordKind::RunFailed.id(),
        RecordKind::Snapshot.id(),
        RecordKind::Blob.id(),
        RecordKind::IndexUpdate.id(),
    ];
    let mut sorted = ids.to_vec();
    sorted.sort();
    let mut deduped = sorted.clone();
    deduped.dedup();
    assert_eq!(
        ids.len(),
        deduped.len(),
        "all RecordKind ids must be distinct"
    );
}


#[test]
fn constants_have_expected_values() {
    // Given the module constants
    // When inspected
    // Then they match the contract values
    assert_eq!(RECORD_HEADER_LEN, 60);
    assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    assert_eq!(MAGIC_COMPILED_ARTIFACT, 0x5642_4952);
    assert_eq!(MAGIC_JOURNAL_EVENT, 0x5642_4A45);
    assert_eq!(MAGIC_SNAPSHOT, 0x5642_534E);
    assert_eq!(MAGIC_BLOB, 0x5642_424C);
    assert_eq!(MAGIC_IPC_FRAME, 0x5642_4C54);
    assert_eq!(MAGIC_WORKFLOW_SOURCE, 0x5642_5352);
    assert_eq!(MAGIC_INDEX_RECORD, 0x5642_4958);
}
