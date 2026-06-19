// HVR-PO-STORAGE-001: shared-source Verus bridge for the production MRWE5
// scalar kernel. This file binds directly to crates/vb_storage/src/mrwe5_contract.rs
// with #[path]; it does not define replacement constants or local mirror enums.

use vstd::prelude::*;

verus! {

#[path = "../../crates/vb_storage/src/mrwe5_contract.rs"]
mod production_mrwe5_contract;

use production_mrwe5_contract::{
    MRWE5_JOURNAL_MAX_KIND_ID, MRWE5_JOURNAL_MIN_KIND_ID, MRWE5_MAGIC_JOURNAL_EVENT,
    MRWE5_SLOT_WRITTEN_KIND_ID, MRWE5_STEP_SUCCEEDED_KIND_ID, Mrwe5KindCompatibility,
    Mrwe5PayloadClass, Mrwe5RecordKindFamilyDecision, Mrwe5SemanticDecodeDecision,
    mrwe5_canonical_kind_id, mrwe5_classify_kind_compatibility,
    mrwe5_classify_record_kind_family, mrwe5_classify_semantic_decode,
    mrwe5_is_journal_record_kind, mrwe5_kinds_are_exact_match,
};

pub fn hvr_po_storage_001_journal_kind_bridge(kind: u16) -> (accepted: bool)
    ensures
        accepted ==> MRWE5_JOURNAL_MIN_KIND_ID <= kind && kind <= MRWE5_JOURNAL_MAX_KIND_ID,
        MRWE5_JOURNAL_MIN_KIND_ID <= kind && kind <= MRWE5_JOURNAL_MAX_KIND_ID ==> accepted,
{
    mrwe5_is_journal_record_kind(kind)
}

pub fn hvr_po_storage_001_record_family_bridge(
    magic: u32,
    kind: u16,
) -> (decision: Mrwe5RecordKindFamilyDecision)
    ensures
        decision == Mrwe5RecordKindFamilyDecision::Accepted ==> magic == MRWE5_MAGIC_JOURNAL_EVENT
            && MRWE5_JOURNAL_MIN_KIND_ID <= kind
            && kind <= MRWE5_JOURNAL_MAX_KIND_ID,
        magic == MRWE5_MAGIC_JOURNAL_EVENT
            && MRWE5_JOURNAL_MIN_KIND_ID <= kind
            && kind <= MRWE5_JOURNAL_MAX_KIND_ID ==> decision == Mrwe5RecordKindFamilyDecision::Accepted,
        decision == Mrwe5RecordKindFamilyDecision::Rejected ==> !(magic == MRWE5_MAGIC_JOURNAL_EVENT
            && MRWE5_JOURNAL_MIN_KIND_ID <= kind
            && kind <= MRWE5_JOURNAL_MAX_KIND_ID),
{
    mrwe5_classify_record_kind_family(magic, kind)
}

pub fn hvr_po_storage_001_exact_match_bridge(envelope_kind: u16, payload_kind: u16) -> (matches: bool)
    ensures
        matches ==> envelope_kind == payload_kind,
        envelope_kind == payload_kind ==> matches,
{
    mrwe5_kinds_are_exact_match(envelope_kind, payload_kind)
}

pub fn hvr_po_storage_001_kind_compatibility_bridge(
    envelope_kind: u16,
    payload_kind: u16,
) -> (decision: Mrwe5KindCompatibility)
    ensures
        envelope_kind == payload_kind ==> decision == Mrwe5KindCompatibility::ExactMatch,
        envelope_kind != payload_kind ==> decision == Mrwe5KindCompatibility::RejectedMismatch,
{
    mrwe5_classify_kind_compatibility(envelope_kind, payload_kind)
}

pub fn hvr_po_storage_001_semantic_decode_bridge(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) -> (decision: Mrwe5SemanticDecodeDecision)
    ensures
        envelope_kind != payload_kind ==> decision == Mrwe5SemanticDecodeDecision::KindPayloadMismatch,
        envelope_kind == payload_kind && event_valid ==> decision == Mrwe5SemanticDecodeDecision::SemanticSuccess,
        envelope_kind == payload_kind && !event_valid ==> decision == Mrwe5SemanticDecodeDecision::InvalidEvent,
{
    mrwe5_classify_semantic_decode(envelope_kind, payload_kind, event_valid)
}

pub fn hvr_po_storage_001_canonical_kind_bridge(class: Mrwe5PayloadClass) -> (kind: Option<u16>)
    ensures
        class == Mrwe5PayloadClass::StepSucceeded ==> kind == Some(MRWE5_STEP_SUCCEEDED_KIND_ID),
        class == Mrwe5PayloadClass::SlotWrittenEvent ==> kind == Some(MRWE5_SLOT_WRITTEN_KIND_ID),
        class == Mrwe5PayloadClass::Other ==> kind == None::<u16>,
{
    mrwe5_canonical_kind_id(class)
}

}
