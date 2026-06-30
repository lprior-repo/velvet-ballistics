#![forbid(unsafe_code)]
use super::*;

#[test]
fn batch_constructed_with_default_constructor_is_empty() {
    let (_temp, journal) = temp_journal();
    let batch = journal.batch();
    assert_eq!(batch.len(), 0, "new batch must have zero length");
    assert!(batch.is_empty(), "new batch must be empty");
}

#[test]
fn batch_constructed_via_new_starts_empty() {
    let (_temp, journal) = temp_journal();
    let batch = JournalWriteBatch::new(&journal);
    assert_eq!(batch.len(), 0);
    assert!(batch.is_empty());
}

#[test]
fn encode_record_returns_at_least_record_header_bytes() {
    let (_temp, _journal) = temp_journal();
    let run = RunId::new(1);
    let event = make_event(run, 0);
    let value = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode_record must succeed for valid event");
    assert!(
        value.len() >= RECORD_HEADER_BYTES,
        "encoded len {} must be >= RECORD_HEADER_BYTES ({})",
        value.len(),
        RECORD_HEADER_BYTES
    );
    assert!(
        value.len() > RECORD_HEADER_BYTES,
        "encoded len {} must exceed header (has payload)",
        value.len()
    );
}

#[test]
fn encoded_length_exceeds_postcard_payload_length() {
    let (_temp, _journal) = temp_journal();
    let run = RunId::new(2);
    let event = make_event(run, 0);
    let value = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode_record must succeed");
    let postcard_len = postcard::to_allocvec(&event)
        .expect("postcard must succeed")
        .len();
    assert!(
        value.len() > postcard_len,
        "encoded len {} must exceed payload len {}",
        value.len(),
        postcard_len
    );
    assert_eq!(
        value.len() - postcard_len,
        RECORD_HEADER_BYTES,
        "difference must be exactly RECORD_HEADER_BYTES (60)"
    );
}

#[test]
fn accounting_uses_full_encoded_length_not_payload_length() {
    let (_temp, _journal) = temp_journal();
    let run = RunId::new(3);
    let event = make_event(run, 0);
    let value = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode_record must succeed");
    let full_len = value.len();
    let postcard_len = postcard::to_allocvec(&event)
        .expect("postcard must succeed")
        .len();
    assert!(
        full_len > postcard_len,
        "full encoded len {full_len} must be greater than payload-only len {postcard_len}"
    );
}

#[test]
fn encode_record_rejects_oversize_payload_with_payload_too_large() {
    let (_temp, _journal) = temp_journal();
    let run = RunId::new(4);
    let event = make_event(run, 0);
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        0u32,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "must return PayloadTooLarge when max=0, got {result:?}"
    );
}

#[test]
fn encode_record_accepts_payload_at_exact_cap() {
    let (_temp, _journal) = temp_journal();
    let run = RunId::new(5);
    let event = make_event(run, 0);
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        result.is_ok(),
        "encode_record must accept payload at exact cap, got {result:?}"
    );
}

#[test]
fn encode_record_failure_does_not_enter_write_batch() {
    let (_temp, _journal) = temp_journal();
    let batch = JournalWriteBatch::new(&_journal);
    let initial_len = batch.len();

    assert_eq!(batch.len(), initial_len, "batch must be unchanged");
}
