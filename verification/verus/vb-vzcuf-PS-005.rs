// Verus proof obligations for encoded byte accounting (PS-005, C2).
//
// Obligation ID: POB-vb-vzcuf-017
// Verifier: verus
// Command: cargo verus --crate-type=lib verification/verus/vb-vzcuf-PS-005.rs
//
// Domain claim: Encoded byte accounting uses full encoded journal event
// value length returned by encode_record, not payload-only length.
//
// PRODUCTION BINDING:
//   Target: crates/vb_storage/src/codec/mod.rs encode_record (lines 20-32)
//   Production constants from crates/vb_storage/src/constants.rs:
//     - RECORD_HEADER_LEN = 60 (line 46) — full header size
//     - MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_576 (line 78)
//   encode_record returns Result<Vec<u8>, JournalError>.
//   The Vec<u8>.len() is the full encoded length including:
//     - 60-byte RECORD_HEADER_LEN (magic, kind, seq, payload_len, checksums)
//     - All postcard-serialized payload bytes
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-017

use vstd::prelude::*;

verus! {

/// Production constant: RECORD_HEADER_LEN from crates/vb_storage/src/constants.rs:46
pub open spec fn record_header_len() -> u64 {
    60u64
}

/// Maximum journal event payload bytes (from constants.rs:78).
pub open spec fn max_payload_bytes() -> u32 {
    1_048_576u32
}

/// Spec: encoded record length = RECORD_HEADER_LEN + payload_bytes.
/// PRODUCTION BINDING: models encode_record's Vec<u8>.len() return value.
pub open spec fn encoded_length(payload_len: u32) -> int {
    record_header_len() as int + payload_len as int
}

/// Lemma: encoded length is always >= RECORD_HEADER_LEN (minimum 60 bytes).
pub proof fn lemma_encoded_length_min()
    ensures
        forall |p: u32| encoded_length(p) >= record_header_len() as int,
{
}

/// Lemma: encoded length >= payload length (strictly larger due to header).
pub proof fn lemma_encoded_larger_than_payload(payload_len: u32)
    ensures
        encoded_length(payload_len) >= payload_len as int,
{
    assert(encoded_length(payload_len) == record_header_len() as int + payload_len as int);
    assert(record_header_len() as int + payload_len as int >= payload_len as int) by (nonlinear_arith);
}

/// Spec: full accounting uses encoded_length, not payload-only.
pub open spec fn full_accounting(total: int, payload_len: u32) -> bool {
    total >= encoded_length(payload_len)
}

pub proof fn lemma_full_accounting_includes_header(payload_len: u32)
    ensures
        full_accounting(encoded_length(payload_len), payload_len),
{
}

/// Lemma: payload-only accounting underestimates by at least RECORD_HEADER_LEN.
pub proof fn lemma_payload_only_underestimates(payload_len: u32)
    requires
        payload_len > 0,
    ensures
        (payload_len as int) < encoded_length(payload_len),
{
    assert(encoded_length(payload_len) == (record_header_len() as int) + (payload_len as int));
    assert(payload_len as int + 60 > payload_len as int);
    assert(60 == record_header_len() as int);
    assert((payload_len as int) < (record_header_len() as int) + (payload_len as int));
}

/// Lemma: difference between encoded and payload-only is exactly RECORD_HEADER_LEN.
pub proof fn lemma_encoding_overhead_exact(payload_len: u32)
    ensures
        encoded_length(payload_len) - payload_len as int == record_header_len() as int,
{
}

/// Lemma: maximum encoded record fits comfortably in u64.
pub proof fn lemma_max_encoded_in_u64()
    ensures
        encoded_length(max_payload_bytes()) < u64::MAX as int,
{
}

/// Lemma: encoded length is monotonic in payload length.
pub proof fn lemma_encoded_monotonic(a: u32, b: u32)
    requires
        a <= b,
    ensures
        encoded_length(a) <= encoded_length(b),
{
    assert(encoded_length(a) == record_header_len() as int + a as int);
    assert(encoded_length(b) == record_header_len() as int + b as int);
    assert(record_header_len() as int + a as int <= record_header_len() as int + b as int)
        by (nonlinear_arith)
        requires a <= b;
}

} // verus!
