// PO-VB-DYBJ-007
// Verus artifact for selected RecordKind envelope-id mappings and explicit
// separation from serde/Postcard enum bytes.
// Production binding: mirrors `vb_storage::records::RecordKind::id` at
// `crates/vb_storage/src/records.rs:135-220` for selected variants.

use vstd::prelude::*;

verus! {

pub enum RecordKindModel {
    RunAccepted,
    RunHeader,
    CompiledIr,
}

pub open spec fn envelope_id(kind: RecordKindModel) -> int {
    match kind {
        RecordKindModel::RunAccepted => 10,
        RecordKindModel::RunHeader => 3,
        RecordKindModel::CompiledIr => 2,
    }
}

pub open spec fn postcard_enum_byte(kind: RecordKindModel) -> int {
    match kind {
        RecordKindModel::RunAccepted => 3,
        RecordKindModel::RunHeader => 2,
        RecordKindModel::CompiledIr => 1,
    }
}

pub open spec fn envelope_id_u16_le(kind: RecordKindModel) -> (int, int) {
    (envelope_id(kind), 0)
}

pub proof fn proof_selected_record_kind_envelope_ids()
    ensures
        envelope_id(RecordKindModel::RunAccepted) == 10,
        envelope_id(RecordKindModel::RunHeader) == 3,
        envelope_id(RecordKindModel::CompiledIr) == 2,
{
}

pub proof fn proof_postcard_and_envelope_surfaces_are_named_distinct(kind: RecordKindModel)
    ensures
        postcard_enum_byte(kind) != envelope_id(kind),
        envelope_id_u16_le(kind).1 == 0,
{
}

pub proof fn proof_swapped_surface_rejected_for_selected_variants()
    ensures
        postcard_enum_byte(RecordKindModel::RunAccepted) != envelope_id(RecordKindModel::RunAccepted),
        postcard_enum_byte(RecordKindModel::RunHeader) != envelope_id(RecordKindModel::RunHeader),
{
}

} // verus!

fn main() {}
