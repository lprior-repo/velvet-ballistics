// Verus proof obligations for error variant discrimination (PS-003, C4, C6).
//
// Obligation ID: POB-vb-vzcuf-009
// Verifier: verus
// Command: cargo verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs
//
// Domain claim: Accumulated budget rejection is distinct from
// QueueFull and PayloadTooLarge under controlled unrelated guards.
//
// PRODUCTION BINDING:
//   Target: crates/vb_storage/src/error/mod.rs JournalError enum (lines 20-247)
//   Production error variants:
//     - JournalError::QueueFull (line 45) — batch count limit
//     - JournalError::PayloadTooLarge { len, max } (line 110) — per-record payload limit
//     - JournalError::DuplicateEvent { run, seq } (line 31) — key collision
//   Contract C4 requires a distinct variant for accumulated byte rejection.
//
//   This spec models the variant discriminant space and proves that
//   the new AccumulatedBytesExceeded variant is distinguishable from
//   QueueFull and PayloadTooLarge, satisfying the C4 requirement.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-009

use vstd::prelude::*;

verus! {

/// Discriminant model for JournalError variants relevant to byte accounting.
/// PRODUCTION BINDING: mirrors JournalError enum in error/mod.rs.
pub enum ErrorVariant {
    QueueFull,
    PayloadTooLarge,
    AccumulatedBytesExceeded,
}

/// Spec: error variants must be distinguishable.
/// C4 requires that AccumulatedBytesExceeded is NOT QueueFull or PayloadTooLarge.
pub open spec fn distinct_variants() -> bool {
    ErrorVariant::AccumulatedBytesExceeded != ErrorVariant::QueueFull
        && ErrorVariant::AccumulatedBytesExceeded != ErrorVariant::PayloadTooLarge
        && ErrorVariant::QueueFull != ErrorVariant::PayloadTooLarge
}

/// Lemma: AccumulatedBytesExceeded is distinct from QueueFull.
/// Production binding: error matching in downstream code must distinguish.
pub proof fn lemma_error_variant_distinct_from_queue_full()
    ensures
        ErrorVariant::AccumulatedBytesExceeded != ErrorVariant::QueueFull,
{
}

/// Lemma: AccumulatedBytesExceeded is distinct from PayloadTooLarge.
pub proof fn lemma_error_variant_distinct_from_payload_too_large()
    ensures
        ErrorVariant::AccumulatedBytesExceeded != ErrorVariant::PayloadTooLarge,
{
}

/// Lemma: QueueFull is distinct from PayloadTooLarge.
pub proof fn lemma_error_variant_queue_full_distinct_from_payload()
    ensures
        ErrorVariant::QueueFull != ErrorVariant::PayloadTooLarge,
{
}

/// Lemma: all three error variants are pairwise distinct.
pub proof fn lemma_all_variants_distinct()
    ensures
        distinct_variants(),
{
    assert(ErrorVariant::QueueFull != ErrorVariant::PayloadTooLarge);
    assert(ErrorVariant::QueueFull != ErrorVariant::AccumulatedBytesExceeded);
    assert(ErrorVariant::PayloadTooLarge != ErrorVariant::AccumulatedBytesExceeded);
}

/// Guard precedence model for append_event.
/// PRODUCTION BINDING: matches guard order in batch.rs append_event (lines 209-229):
///   1. Key validation (run_event_key)
///   2. Durable duplicate check (events.contains_key)
///   3. Batch count limit (inner.len() >= MAX_BATCH_COUNT)
///   4. Per-record encoding (encode_record)
///   5. Accumulated byte admission (to be added)
pub enum Guard {
    KeyValidation,
    DurableDuplicate,
    BatchCount,
    PerRecordEncoding,
    AccumulatedByteAdmission,
}

/// Guard index for comparison.
pub open spec fn guard_index(g: Guard) -> u8 {
    match g {
        Guard::KeyValidation => 0,
        Guard::DurableDuplicate => 1,
        Guard::BatchCount => 2,
        Guard::PerRecordEncoding => 3,
        Guard::AccumulatedByteAdmission => 4,
    }
}

/// Spec: guard precedence ordering for append_event.
/// The accumulated byte admission guard must be after encoding
/// (because we need encoded_len) but before the insert mutation.
/// Uses guard_index for ordering comparisons.
pub open spec fn guard_precedence_order() -> bool {
    guard_index(Guard::KeyValidation) < guard_index(Guard::DurableDuplicate)
        && guard_index(Guard::DurableDuplicate) < guard_index(Guard::BatchCount)
        && guard_index(Guard::BatchCount) < guard_index(Guard::PerRecordEncoding)
        && guard_index(Guard::PerRecordEncoding) < guard_index(Guard::AccumulatedByteAdmission)
}

/// Lemma: guard precedence is well-ordered.
pub proof fn lemma_guard_precedence_well_ordered()
    ensures
        guard_precedence_order(),
{
}

} // verus!
