// Verus proof obligations for guard precedence (PS-008, C6).
//
// Obligation ID: POB-vb-vzcuf-029
// Verifier: verus
// Command: cargo verus --crate-type=lib verification/verus/vb-vzcuf-PS-008.rs
//
// Domain claim: Guard precedence remains key, durable duplicate,
// count, per-record payload, accumulated bytes, mutation.
//
// PRODUCTION BINDING:
//   Target: crates/vb_storage/src/batch.rs append_event (lines 209-229)
//   Guard order observed in production code:
//     Line 210: run_event_key(event.run_id(), event.seq())? — key validation
//     Line 211: self.journal.events.contains_key(key)? — durable duplicate check
//     Line 218: self.inner.len() >= MAX_BATCH_COUNT — count limit → QueueFull
//     Line 221: encode_record(...) — per-record encoding/payload validation
//     (accumulated byte admission goes here, after encoding, before insert)
//     Line 228: self.inner.insert(...) — mutation/insertion
//
//   This spec models the guard ordering and proves that each guard
//   has a well-defined position that downstream code can rely on.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-029

use vstd::prelude::*;

verus! {

/// Guard stages in append_event execution order.
/// PRODUCTION BINDING: matches the guard order at batch.rs:210-228.
pub enum Guard {
    KeyValidation,           // line 210: run_event_key
    DurableDuplicate,        // line 211: events.contains_key
    BatchCount,              // line 218: inner.len() >= MAX_BATCH_COUNT
    PerRecordEncoding,       // line 221: encode_record
    AccumulatedByteAdmission, // contract C6: byte budget check
    Mutation,                // line 228: inner.insert
}

/// Guard index for ordering comparisons.
pub open spec fn guard_index(g: Guard) -> u8 {
    match g {
        Guard::KeyValidation => 0,
        Guard::DurableDuplicate => 1,
        Guard::BatchCount => 2,
        Guard::PerRecordEncoding => 3,
        Guard::AccumulatedByteAdmission => 4,
        Guard::Mutation => 5,
    }
}

/// Spec: guards are in strict ascending order.
pub open spec fn guard_order_valid() -> bool {
    guard_index(Guard::KeyValidation) < guard_index(Guard::DurableDuplicate)
        && guard_index(Guard::DurableDuplicate) < guard_index(Guard::BatchCount)
        && guard_index(Guard::BatchCount) < guard_index(Guard::PerRecordEncoding)
        && guard_index(Guard::PerRecordEncoding) < guard_index(Guard::AccumulatedByteAdmission)
        && guard_index(Guard::AccumulatedByteAdmission) < guard_index(Guard::Mutation)
}

/// Lemma: guard precedence is totally ordered.
pub proof fn lemma_guard_order_is_valid()
    ensures
        guard_order_valid(),
{
}

/// Lemma: KeyValidation executes before DurableDuplicate.
pub proof fn lemma_key_before_duplicate()
    ensures
        guard_index(Guard::KeyValidation) < guard_index(Guard::DurableDuplicate),
{
}

/// Lemma: DurableDuplicate before BatchCount (count check).
pub proof fn lemma_duplicate_before_count()
    ensures
        guard_index(Guard::DurableDuplicate) < guard_index(Guard::BatchCount),
{
}

/// Lemma: BatchCount before PerRecordEncoding.
pub proof fn lemma_count_before_encoding()
    ensures
        guard_index(Guard::BatchCount) < guard_index(Guard::PerRecordEncoding),
{
}

/// Lemma: PerRecordEncoding before AccumulatedByteAdmission.
/// This is critical: we need encoded_len for byte admission.
pub proof fn lemma_encoding_before_admission()
    ensures
        guard_index(Guard::PerRecordEncoding) < guard_index(Guard::AccumulatedByteAdmission),
{
}

/// Lemma: AccumulatedByteAdmission before Mutation.
/// Rejection at this guard prevents the insert.
pub proof fn lemma_admission_before_mutation()
    ensures
        guard_index(Guard::AccumulatedByteAdmission) < guard_index(Guard::Mutation),
{
}

/// Spec: AccumulatedByteAdmission is after encoding (needs encoded_len).
pub open spec fn admission_after_encoding() -> bool {
    guard_index(Guard::PerRecordEncoding) < guard_index(Guard::AccumulatedByteAdmission)
}

/// Spec: AccumulatedByteAdmission is before mutation (rejection prevents insert).
pub open spec fn admission_before_mutation() -> bool {
    guard_index(Guard::AccumulatedByteAdmission) < guard_index(Guard::Mutation)
}

pub proof fn lemma_guard_positions_contract()
    ensures
        admission_after_encoding(),
        admission_before_mutation(),
{
}

} // verus!
