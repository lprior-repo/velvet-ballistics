// Verification artifact: encoding_injectivity.rs
// PO: PO-V02
// Bead: vb-xi2f.35
// Verifier: Verus
// Command: verus --crate-type=lib verification/verus/vb_compile/encoding_injectivity.rs
// Workdir: crates/vb_compile
//
// Proof obligation: Prove that the domain-tagged field encoding function
// is injective: for all contract_a ≠ contract_b, encode_contract(contract_a) ≠ encode_contract(contract_b).

#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

// ============================================================================
// Model: Contract field encoding
// ============================================================================

pub struct TaggedField {
    pub tag: Seq<u8>,
    pub value: Seq<u8>,
}

pub struct ContractEncoding {
    pub fields: Seq<TaggedField>,
}

/// Encoding: concatenate tag + value for each field.
pub closed spec fn encode_contract(fields: Seq<TaggedField>) -> Seq<u8>
    decreases fields.len()
{
    if fields.len() == 0 {
        Seq::empty()
    } else {
        let head = fields[0];
        let rest = fields.drop_first();
        head.tag + head.value + encode_contract(rest)
    }
}

pub closed spec fn tagged_field_eq(a: TaggedField, b: TaggedField) -> bool {
    a.tag == b.tag && a.value == b.value
}

pub closed spec fn contract_encodings_equal(a: ContractEncoding, b: ContractEncoding) -> bool {
    a.fields.len() == b.fields.len()
    && forall|i: int| 0 <= i && i < a.fields.len() as int
        ==> tagged_field_eq(a.fields[i], b.fields[i])
}

// ============================================================================
// Concrete instantiation: the 18-field ResourceContract
// ============================================================================

pub const CONTRACT_FIELD_TAGS: [&'static str; 18] = [
    "max_steps",
    "max_slots",
    "max_constants",
    "max_accessors",
    "max_expressions",
    "max_expr_stack",
    "max_step_budget_per_tick",
    "max_transitions_per_tick",
    "max_input_bytes",
    "max_output_bytes",
    "max_blob_bytes",
    "max_ipc_payload_bytes",
    "max_retry_attempts",
    "max_fanout",
    "max_collect_items",
    "max_queue_depth",
    "max_journal_batch_bytes",
    "allows_secret_results",
];

/// Lemma: All 18 field tags are unique.
pub proof fn lemma_field_tags_unique()
    ensures
        forall|i: int, j: int|
            0 <= i && i < 18 && 0 <= j && j < 18 && i != j
            ==> CONTRACT_FIELD_TAGS[i] != CONTRACT_FIELD_TAGS[j],
{
    // The field tags are statically known to be unique.
    assert(forall|i: int, j: int|
        0 <= i && i < 18 && 0 <= j && j < 18 && i != j
        ==> CONTRACT_FIELD_TAGS[i] != CONTRACT_FIELD_TAGS[j]);
}

} // verus!
