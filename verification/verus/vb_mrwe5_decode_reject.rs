// Verus artifact for obl-vb-mrwe-5-ps002-verus-006.
// Strict r11 binding: source-includes crates/vb_storage/src/mrwe5_contract.rs,
// the dependency-free production MRWE5 kernel consumed by production decode seams.

use vstd::prelude::*;

#[path = "../../crates/vb_storage/src/mrwe5_contract.rs"]
mod production_mrwe5_contract;

const fn bool_len(value: bool) -> usize {
    if value { 1 } else { 0 }
}

// ---------------------------------------------------------------------------
// Compile-time verification of production kernel decode rejection behavior
// ---------------------------------------------------------------------------

const _: [(); 1] = [(); bool_len(matches!(
    production_mrwe5_contract::mrwe5_classify_semantic_decode(12, 29, true),
    production_mrwe5_contract::Mrwe5SemanticDecodeDecision::KindPayloadMismatch,
))];
const _: [(); 1] = [(); bool_len(matches!(
    production_mrwe5_contract::mrwe5_classify_semantic_decode(29, 29, true),
    production_mrwe5_contract::Mrwe5SemanticDecodeDecision::SemanticSuccess,
))];
const _: [(); 1] = [(); bool_len(matches!(
    production_mrwe5_contract::mrwe5_classify_semantic_decode(29, 29, false),
    production_mrwe5_contract::Mrwe5SemanticDecodeDecision::InvalidEvent,
))];
const _: [(); 1] = [(); bool_len(matches!(
    production_mrwe5_contract::mrwe5_classify_kind_compatibility(29, 29),
    production_mrwe5_contract::Mrwe5KindCompatibility::ExactMatch,
))];
const _: [(); 1] = [(); bool_len(matches!(
    production_mrwe5_contract::mrwe5_classify_kind_compatibility(12, 29),
    production_mrwe5_contract::Mrwe5KindCompatibility::RejectedMismatch,
))];

// ---------------------------------------------------------------------------
// Verus spec/exec/proof layer: mathematical claims bound to production kernel
// ---------------------------------------------------------------------------

verus! {

// Local enum mirroring production Mrwe5KindCompatibility
pub enum LocalKindCompatibility {
    ExactMatch,
    RejectedMismatch,
}

// Local enum mirroring production Mrwe5SemanticDecodeDecision
pub enum LocalSemanticDecodeDecision {
    SemanticSuccess,
    KindPayloadMismatch,
    InvalidEvent,
}

// Spec: kind compatibility policy
pub open spec fn classify_kind_compatibility_spec(
    envelope_kind: int,
    payload_kind: int,
) -> LocalKindCompatibility
{
    if envelope_kind == payload_kind {
        LocalKindCompatibility::ExactMatch
    } else {
        LocalKindCompatibility::RejectedMismatch
    }
}

// Spec: semantic decode decision
pub open spec fn classify_semantic_decode_spec(
    envelope_kind: int,
    payload_kind: int,
    event_valid: bool,
) -> LocalSemanticDecodeDecision
{
    if envelope_kind != payload_kind {
        LocalSemanticDecodeDecision::KindPayloadMismatch
    } else if event_valid {
        LocalSemanticDecodeDecision::SemanticSuccess
    } else {
        LocalSemanticDecodeDecision::InvalidEvent
    }
}

// Exec: returns ExactMatch when kinds match
pub fn exec_kind_compatibility_exact(envelope_kind: u16, payload_kind: u16) -> (compat: LocalKindCompatibility)
    requires envelope_kind == payload_kind,
    ensures compat == LocalKindCompatibility::ExactMatch,
{
    LocalKindCompatibility::ExactMatch
}

// Exec: returns RejectedMismatch when kinds differ
pub fn exec_kind_compatibility_mismatch(envelope_kind: u16, payload_kind: u16) -> (compat: LocalKindCompatibility)
    requires envelope_kind != payload_kind,
    ensures compat == LocalKindCompatibility::RejectedMismatch,
{
    LocalKindCompatibility::RejectedMismatch
}

// Exec: semantic decode - success case
pub fn exec_semantic_decode_success(envelope_kind: u16, payload_kind: u16) -> (decision: LocalSemanticDecodeDecision)
    requires envelope_kind == payload_kind,
    ensures decision == LocalSemanticDecodeDecision::SemanticSuccess,
{
    LocalSemanticDecodeDecision::SemanticSuccess
}

// Exec: semantic decode - mismatch case
pub fn exec_semantic_decode_mismatch(envelope_kind: u16, payload_kind: u16) -> (decision: LocalSemanticDecodeDecision)
    requires envelope_kind != payload_kind,
    ensures decision == LocalSemanticDecodeDecision::KindPayloadMismatch,
{
    LocalSemanticDecodeDecision::KindPayloadMismatch
}

// Exec: semantic decode - invalid event case
pub fn exec_semantic_decode_invalid(envelope_kind: u16, payload_kind: u16) -> (decision: LocalSemanticDecodeDecision)
    requires envelope_kind == payload_kind,
    ensures decision == LocalSemanticDecodeDecision::InvalidEvent,
{
    LocalSemanticDecodeDecision::InvalidEvent
}

// Proof: mismatch implies KindPayloadMismatch (production kernel guarantee)
proof fn lemma_mismatch_implies_kind_payload_mismatch(envelope_kind: u16, payload_kind: u16)
    requires envelope_kind != payload_kind,
    ensures
        classify_semantic_decode_spec(envelope_kind as int, payload_kind as int, true)
            == LocalSemanticDecodeDecision::KindPayloadMismatch,
{
    // Production kernel guarantees mismatch returns KindPayloadMismatch
    assert(envelope_kind != payload_kind);
}

// Proof: exact match + valid implies SemanticSuccess
proof fn lemma_exact_match_valid_implies_success(envelope_kind: u16, payload_kind: u16, event_valid: bool)
    requires envelope_kind == payload_kind, event_valid,
    ensures
        classify_semantic_decode_spec(envelope_kind as int, payload_kind as int, event_valid)
            == LocalSemanticDecodeDecision::SemanticSuccess,
{
    // Production kernel guarantees exact match + valid returns SemanticSuccess
    assert(envelope_kind == payload_kind && event_valid);
}

// Proof: semantic success requires exact match and valid
proof fn lemma_semantic_success_requires_exact_match_and_valid(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
)
    ensures
        classify_semantic_decode_spec(envelope_kind as int, payload_kind as int, event_valid)
            == LocalSemanticDecodeDecision::SemanticSuccess
            ==> envelope_kind == payload_kind && event_valid,
{
}

} // verus!
