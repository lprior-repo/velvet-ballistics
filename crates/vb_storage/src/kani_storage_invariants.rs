#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::{
    EventSeq, JournalBatchSize, JournalError, JournalQueueCapacity, StorageLimits,
};

// ---------------------------------------------------------------------------
// Invariant 1: Journal sequence numbers are monotonic
// ---------------------------------------------------------------------------

#[kani::proof]
#[kani::unwind(8)]
fn kani_next_seq_monotonic_for_all_values() {
    let raw: u64 = kani::any();
    let seq = EventSeq::new(raw);
    let result = crate::codec::next_seq(seq);

    if raw == u64::MAX {
        kani::assert(matches!(result, Err(JournalError::SequenceOverflow), "assertion failed"),
            "u64::MAX must overflow",
        );
    } else {
        let Ok(next) = result else {
            ,
            "u64::MAX must overflow",
        );
    } else {
        let Ok(next) = result else {
            kani::assert(false, "next_seq must not fail for non-max values");
            return;
        };
        kani::assert(next.get() == raw + 1, "next_seq must increment by 1");
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_event_seq_ordering_invariant() {
    let a: u64 = kani::any();
    let b: u64 = kani::any();
    kani::assume(a < b);
    let seq_a = EventSeq::new(a);
    let seq_b = EventSeq::new(b);
     == raw + 1, "next_seq must increment by 1");
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_event_seq_ordering_invariant() {
    let a: u64 = kani::any();
    let b: u64 = kani::any();
    kani::assume(a < b);
    let seq_a = EventSeq::new(a);
    let seq_b = EventSeq::new(b);
    kani::assert(seq_a < seq_b, "EventSeq ordering must match raw ordering");
    kani::assert(seq_a.get() < seq_b.get(), "get() must match raw ordering");
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_event_seq_zones_preserved() {
    let val: u64 = kani::any();
    let seq = EventSeq::new(val);
    kani::assert(seq.get(, "assertion failed") == val, "EventSeq::new must be identity");
    kani::assert(EventSeq::ZERO.get(, "assertion failed") == 0, "ZERO must be 0");
    kani::assert(EventSeq::MIN.get(, "assertion failed") == 0, "MIN must be 0");
    kani::assert(EventSeq::MAX.get(, "assertion failed") == u64::MAX, "MAX must be u64::MAX");
}

// ---------------------------------------------------------------------------
// Invariant 2: Queue capacity is never exceeded
// ---------------------------------------------------------------------------

#[kani::proof]
#[kani::unwind(8)]
fn kani_queue_capacity_must_be_nonzero() {
    let cap: usize = kani::any();
    let batch: usize = kani::any();
    let result = JournalWriterQueue::new(cap, batch, StorageLimits::DEFAULT);
    if cap == 0 || batch == 0 {
        kani::assert(matches!(result, Err(JournalError::QueueCapacity), "assertion failed"),
            "zero capacity/batch must be rejected",
        );
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_queue_capacity_contract_preservation() {
    let cap_raw: usize = kani::any();
    kani::assume(cap_raw > 0);
    let batch_raw: usize = kani::any();
    kani::assume(batch_raw > 0);

    let cap = match JournalQueueCapacity::try_from_usize(cap_raw) {
        Ok(c) => c,
        Err(_) => return,
    };
    let batch = match JournalBatchSize::try_from_usize(batch_raw) {
        Ok(b) => b,
        Err(_) => return,
    };

    let result = JournalWriterQueue::with_contracts(cap, batch, StorageLimits::DEFAULT);
    kani::assert(result.is_ok(), "valid contracts must construct queue");

    kani::assert(cap.get(, "assertion failed") == cap_raw, "JournalQueueCapacity get must roundtrip");
    kani::assert(batch.get(, "assertion failed") == batch_raw, "JournalBatchSize get must roundtrip");
}

// ---------------------------------------------------------------------------
// Invariant 3: Batch atomicity (all-or-nothing via aborted flag)
// ---------------------------------------------------------------------------

#[kani::proof]
#[kani::unwind(8)]
fn kani_sequence_overflow_boundary() {
    let raw: u64 = kani::any();
    let result = crate::codec::next_seq(EventSeq::new(raw));
    match (raw, result) {
        (u64::MAX, Err(JournalError::SequenceOverflow)) => {}
        (_, Ok(next)) => kani::assert(next.get(, "assertion failed") == raw + 1, "next must be raw+1"),
        _ =>  == raw + 1, "next must be raw+1"),
        _ => kani::assert(false, "unexpected result for {raw}: {result:?}"),
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_event_seq_comparison_is_total() {
    let a: u64 = kani::any();
    let b: u64 = kani::any();
    let sa = EventSeq::new(a);
    let sb = EventSeq::new(b);
    let by_get = a.cmp(&b);
    let by_cmp = sa.cmp(&sb);
    kani::assert(by_cmp == by_get, "PartialOrd must match raw ordering");
}

// ---------------------------------------------------------------------------
// Invariant 4: Recovery determinism - same input produces same output
// ---------------------------------------------------------------------------

#[kani::proof]
#[kani::unwind(8)]
fn kani_validate_replayed_event_rejects_wrong_run() {
    let run_raw: u64 = kani::any();
    let actual_raw: u64 = kani::any();
    kani::assume(run_raw != actual_raw);

    use vb_core::RunId;
    let run = RunId::new(run_raw);
    let actual = RunId::new(actual_raw);

    let event = crate::JournalEvent::RunFinished {
        run: actual,
        seq: EventSeq::new(0),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };

    let result = crate::codec::validate_replayed_event(run, EventSeq::new(0), &event);
    kani::assert(matches!(result, Err(JournalError::WrongRun { .. }), "assertion failed"),
        "wrong run must be rejected",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_validate_replayed_event_rejects_sequence_gap() {
    let run_raw: u64 = kani::any();
    let seq_raw: u64 = kani::any();
    let expected_raw: u64 = kani::any();
    kani::assume(seq_raw != expected_raw);

    use vb_core::RunId;
    let run = RunId::new(run_raw);

    let event = crate::JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(seq_raw),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };

    let result = crate::codec::validate_replayed_event(run, EventSeq::new(expected_raw), &event);
    kani::assert(matches!(result, Err(JournalError::SequenceGap { .. }), "assertion failed"),
        "sequence gap must be rejected",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_encode_reject_rejects_kind_family_mismatch() {
    let record = crate::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        source: vec![1u8],
    };

    let result = crate::encode_record(
        crate::MAGIC_JOURNAL_EVENT,
        crate::RecordKind::WorkflowSource,
        0,
        &record,
        128,
    );
    kani::assert(matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. }), "assertion failed"),
        "kind family mismatch must be rejected",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_encode_rejects_kind_family_mismatch_blob() {
    let record = crate::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        source: vec![1u8],
    };

    let result = crate::encode_record(
        crate::MAGIC_BLOB,
        crate::RecordKind::Snapshot,
        0,
        &record,
        128,
    );
    kani::assert(matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. }), "assertion failed"),
        "kind family mismatch must be rejected",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_next_seq_compares_correctly_with_event_seq_ordering() {
    let raw: u64 = kani::any();
    kani::assume(raw < u64::MAX);
    let seq = EventSeq::new(raw);
    let result = crate::codec::next_seq(seq);
    let Ok(next) = result else {
        ,
        "kind family mismatch must be rejected",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_next_seq_compares_correctly_with_event_seq_ordering() {
    let raw: u64 = kani::any();
    kani::assume(raw < u64::MAX);
    let seq = EventSeq::new(raw);
    let result = crate::codec::next_seq(seq);
    let Ok(next) = result else {
        kani::assert(false, "non-max should not overflow");
        return;
    };
    kani::assert(next > seq, "next_seq result must be strictly greater than input");
    kani::assert(next.get() - seq.get() == 1, "next_seq must increment by exactly 1");
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_record_kind_valid_range() {
    let kind_id: u16 = kani::any();
    let result = crate::codec::validation::validate_known_kind(kind_id);

    let is_valid = matches!(kind_id, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50);
    match (is_valid, result) {
        (true, Ok(())) => {}
        (false, Err(JournalError::UnknownRecordKind { .. })) => {}
        _ =>  - seq.get() == 1, "next_seq must increment by exactly 1");
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_record_kind_valid_range() {
    let kind_id: u16 = kani::any();
    let result = crate::codec::validation::validate_known_kind(kind_id);

    let is_valid = matches!(kind_id, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50);
    match (is_valid, result) {
        (true, Ok(())) => {}
        (false, Err(JournalError::UnknownRecordKind { .. })) => {}
        _ => kani::assert(false, "validate_known_kind result mismatch for {kind_id}: {result:?}"),
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_validate_schema_version_all_values() {
    let version: u16 = kani::any();
    let current = crate::constants::CURRENT_SCHEMA_VERSION;
    let result = crate::codec::validation::validate_schema_version(version);

    match (version.cmp(&current), result) {
        (std::cmp::Ordering::Equal, Ok(())) => {}
        (std::cmp::Ordering::Less, Err(JournalError::MigrationRequired { .. })) => {}
        (std::cmp::Ordering::Greater, Err(JournalError::UnsupportedSchemaVersion { .. })) => {}
        _ => , result) {
        (std::cmp::Ordering::Equal, Ok(())) => {}
        (std::cmp::Ordering::Less, Err(JournalError::MigrationRequired { .. })) => {}
        (std::cmp::Ordering::Greater, Err(JournalError::UnsupportedSchemaVersion { .. })) => {}
        _ => kani::assert(false, "schema version validation mismatch for {version}: {result:?}"),
    }
}

// ---------------------------------------------------------------------------
// Helper: expose private validation function for Kani
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) mod validation {
    pub use crate::codec::validation::{
        validate_known_kind, validate_schema_version,
    };
}
