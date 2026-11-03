//! PS-007 core/storage bridge binding tests (vb-mvvrr).
//!
//! Deterministic boundary evidence for the `vb_vzcuf_PS_007` fuzz oracle:
//! the bridge budget the fuzzer exercises is the actual production
//! conversion behavior, not scalar aliases.
//!
//! Behaviors covered:
//! - B1: storage batch default equals core policy max (cross-crate bridge).
//! - B2: record budget decomposes into header + payload exactly.
//! - B3: one max-size record overshoots a fresh default batch by one header.
//! - B4: fresh batch carries the production byte limit; small append admitted.
//! - B5: oversized-payload event rejected with typed `PayloadTooLarge`.
//! - B6: second medium append rejected with typed `JournalBatchBytesExceeded`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::let_underscore_must_use,
    clippy::panic_in_result_fn
)]

use vb_core::{BoundednessPolicy, RunId, StepIdx, WorkflowDigest};
use vb_storage::{
    EventSeq, JournalError, JournalEvent,
    batch::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT,
    constants::{
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_JOURNAL_EVENT_RECORD_BYTES, RECORD_HEADER_LEN,
    },
    journal::FjallJournal,
};

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

fn checkpoint_event(run: RunId, seq: u64, payload_len: usize) -> JournalEvent {
    JournalEvent::RetryCheckpoint {
        run,
        seq: EventSeq::new(seq),
        attempt: 1,
        step: StepIdx::new(0),
        payload_digest: [0u8; 32],
        payload: vec![0xA5u8; payload_len],
    }
}

// ============================================================================
// B1: storage batch default equals core policy max (cross-crate bridge)
// ============================================================================

#[test]
fn bridge_storage_default_equals_core_policy_max() {
    let storage_default: u64 = DEFAULT_JOURNAL_BATCH_BYTE_LIMIT;
    let core_policy: u64 = u64::from(BoundednessPolicy::DEFAULT.absolute_max_journal_batch_bytes);
    assert_eq!(
        storage_default, core_policy,
        "storage batch default must equal core policy max: storage={storage_default}, core={core_policy}"
    );
    assert!(storage_default > 0, "bridge budget must be non-zero");
}

// ============================================================================
// B2: record budget decomposes into header + payload exactly
// ============================================================================

#[test]
fn bridge_record_budget_decomposes_into_header_plus_payload() {
    let header = u64::from(RECORD_HEADER_LEN);
    let payload = u64::from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    let record = u64::from(MAX_JOURNAL_EVENT_RECORD_BYTES);
    assert_eq!(
        header.checked_add(payload),
        Some(record),
        "record budget must equal header + payload exactly"
    );
    assert!(
        record > payload,
        "record budget must exceed the bare payload ceiling"
    );
}

// ============================================================================
// B3: one max-size record overshoots a fresh default batch by one header
// ============================================================================

#[test]
fn bridge_max_record_overshoots_default_batch_by_one_header() {
    let header = u64::from(RECORD_HEADER_LEN);
    let record = u64::from(MAX_JOURNAL_EVENT_RECORD_BYTES);
    assert_eq!(
        record.checked_sub(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT),
        Some(header),
        "max record must overshoot the default batch budget by exactly one header"
    );
}

// ============================================================================
// B4: fresh batch carries the production byte limit; small append admitted
// ============================================================================

#[test]
fn bridge_fresh_batch_limit_matches_production_default() {
    let (_temp, journal) = temp_journal();
    let batch = journal.batch();
    assert_eq!(
        batch.byte_limit(),
        Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT),
        "fresh batch must carry the production byte limit"
    );

    let run = RunId::new(11);
    let mut batch = journal.batch();
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let result = batch.append_event(&event);
    assert!(
        matches!(result, Ok(())),
        "small append must be admitted, got {result:?}"
    );
    let staged = batch.staged_event_bytes();
    assert!(staged > 0, "admitted event must advance staged bytes");
    assert!(
        staged <= DEFAULT_JOURNAL_BATCH_BYTE_LIMIT,
        "staged bytes must fit the production limit: staged={staged}"
    );
}

// ============================================================================
// B5: oversized-payload event rejected with typed PayloadTooLarge
// ============================================================================

#[test]
fn bridge_oversized_payload_rejected_with_typed_error() {
    let (_temp, journal) = temp_journal();
    let mut batch = journal.batch();
    let run = RunId::new(12);
    let overlong =
        usize::try_from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES).expect("payload max must fit usize") + 1;
    let event = checkpoint_event(run, 0, overlong);
    let result = batch.append_event(&event);
    match result {
        Err(JournalError::PayloadTooLarge { len, max }) => {
            assert_eq!(
                max, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                "error max must be the production payload ceiling"
            );
            assert!(
                len > max,
                "error len must exceed the ceiling: len={len}, max={max}"
            );
        }
        other => panic!("expected PayloadTooLarge, got {other:?}"),
    }
    assert!(
        !batch.is_aborted(),
        "payload rejection must leave the batch open"
    );
    assert_eq!(
        batch.staged_event_bytes(),
        0,
        "rejected event must not advance staged bytes"
    );
}

// ============================================================================
// B6: second medium append rejected with typed JournalBatchBytesExceeded
// ============================================================================

#[test]
fn bridge_second_medium_append_exceeds_batch_budget() {
    let (_temp, journal) = temp_journal();
    let mut batch = journal.batch();
    let run = RunId::new(13);
    // ~700 KiB payload: one fits the 1 MiB default budget, two do not.
    let event_a = checkpoint_event(run, 0, 700_000);
    let first = batch.append_event(&event_a);
    assert!(
        matches!(first, Ok(())),
        "first medium append must be admitted, got {first:?}"
    );
    let staged_after_first = batch.staged_event_bytes();
    assert!(
        staged_after_first <= DEFAULT_JOURNAL_BATCH_BYTE_LIMIT,
        "first append must fit the budget: staged={staged_after_first}"
    );

    let event_b = checkpoint_event(run, 1, 700_000);
    let result = batch.append_event(&event_b);
    match result {
        Err(JournalError::JournalBatchBytesExceeded { attempted, limit }) => {
            assert_eq!(
                limit, DEFAULT_JOURNAL_BATCH_BYTE_LIMIT,
                "error limit must be the production batch default"
            );
            assert!(
                attempted > limit,
                "attempted must exceed the limit: attempted={attempted}, limit={limit}"
            );
            assert_eq!(
                attempted,
                staged_after_first + staged_after_first,
                "attempted must equal twice the single encoded length"
            );
        }
        other => panic!("expected JournalBatchBytesExceeded, got {other:?}"),
    }
    assert!(
        !batch.is_aborted(),
        "budget rejection must leave the batch open"
    );
    assert_eq!(
        batch.staged_event_bytes(),
        staged_after_first,
        "rejected append must not advance staged bytes"
    );
}
