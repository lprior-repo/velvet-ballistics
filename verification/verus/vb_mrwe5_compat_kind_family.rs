// Verus artifact for obl-vb-mrwe-5-ps004-verus-016.
// Strict r11 binding: source-includes crates/vb_storage/src/mrwe5_contract.rs,
// the dependency-free production kernel used for family and compatibility policy.

use vstd::prelude::*;

#[path = "../../crates/vb_storage/src/mrwe5_contract.rs"]
mod production_mrwe5_contract;

const fn bool_len(value: bool) -> usize {
    if value { 1 } else { 0 }
}

// ---------------------------------------------------------------------------
// Compile-time verification of production kernel family/compatibility behavior
// ---------------------------------------------------------------------------

const _: [(); 1] = [(); bool_len(
    production_mrwe5_contract::mrwe5_is_journal_record_kind(29),
)];
const _: [(); 1] = [(); bool_len(
    production_mrwe5_contract::mrwe5_is_journal_record_kind(12),
)];
const _: [(); 1] = [(); bool_len(
    !production_mrwe5_contract::mrwe5_is_journal_record_kind(9),
)];
const _: [(); 1] = [(); bool_len(
    !production_mrwe5_contract::mrwe5_is_journal_record_kind(30),
)];
const _: [(); 1] = [(); bool_len(matches!(
    production_mrwe5_contract::mrwe5_classify_record_kind_family(0x5642_4A45, 29),
    production_mrwe5_contract::Mrwe5RecordKindFamilyDecision::Accepted,
))];
const _: [(); 1] = [(); bool_len(matches!(
    production_mrwe5_contract::mrwe5_classify_record_kind_family(0x5642_4A45, 9),
    production_mrwe5_contract::Mrwe5RecordKindFamilyDecision::Rejected,
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
// Verus spec/exec/proof layer: family and compatibility policy proof
// ---------------------------------------------------------------------------

verus! {

// Local enum mirroring production Mrwe5KindCompatibility
pub enum LocalKindCompatibility {
    ExactMatch,
    RejectedMismatch,
}

// Local enum mirroring production Mrwe5RecordKindFamilyDecision
pub enum LocalRecordKindFamilyDecision {
    Accepted,
    Rejected,
}

// Journal magic constant (from production kernel)
pub spec const JOURNAL_MAGIC: int = 0x5642_4A45int;

// Spec: journal record kind membership
pub open spec fn is_journal_record_kind_spec(kind: int) -> bool
{
    10int <= kind && kind <= 29int
}

// Spec: record kind family classification
pub open spec fn classify_record_kind_family_spec(
    magic: int,
    kind: int,
) -> LocalRecordKindFamilyDecision
{
    if magic == JOURNAL_MAGIC && 10int <= kind && kind <= 29int {
        LocalRecordKindFamilyDecision::Accepted
    } else {
        LocalRecordKindFamilyDecision::Rejected
    }
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

// Exec: returns true for kinds in journal family range
pub fn exec_is_journal_record_kind(kind: u16) -> (accepted: bool)
    ensures accepted == (10u16 <= kind && kind <= 29u16)
{
    10u16 <= kind && kind <= 29u16
}

// Exec: family classification - Accepted case
pub fn exec_record_family_accepted(magic: u32, kind: u16) -> (decision: LocalRecordKindFamilyDecision)
    requires magic == 0x5642_4A45u32 && 10u16 <= kind && kind <= 29u16,
    ensures decision == LocalRecordKindFamilyDecision::Accepted,
{
    LocalRecordKindFamilyDecision::Accepted
}

// Exec: family classification - Rejected case
pub fn exec_record_family_rejected(magic: u32, kind: u16) -> (decision: LocalRecordKindFamilyDecision)
    requires !(magic == 0x5642_4A45u32 && 10u16 <= kind && kind <= 29u16),
    ensures decision == LocalRecordKindFamilyDecision::Rejected,
{
    LocalRecordKindFamilyDecision::Rejected
}

// Exec: exact match compatibility
pub fn exec_kind_compatibility_exact(envelope_kind: u16, payload_kind: u16) -> (compat: LocalKindCompatibility)
    requires envelope_kind == payload_kind,
    ensures compat == LocalKindCompatibility::ExactMatch,
{
    LocalKindCompatibility::ExactMatch
}

// Exec: mismatch compatibility
pub fn exec_kind_compatibility_mismatch(envelope_kind: u16, payload_kind: u16) -> (compat: LocalKindCompatibility)
    requires envelope_kind != payload_kind,
    ensures compat == LocalKindCompatibility::RejectedMismatch,
{
    LocalKindCompatibility::RejectedMismatch
}

// Proof: StepSucceeded (29) is in accepted journal family
proof fn lemma_step_kind_is_accepted_journal_family()
    ensures
        is_journal_record_kind_spec(29int) == true,
        classify_record_kind_family_spec(JOURNAL_MAGIC, 29int)
            == LocalRecordKindFamilyDecision::Accepted,
{
}

// Proof: kind 9 is below journal family minimum
proof fn lemma_below_journal_min_is_rejected()
    ensures
        is_journal_record_kind_spec(9int) == false,
        classify_record_kind_family_spec(JOURNAL_MAGIC, 9int)
            == LocalRecordKindFamilyDecision::Rejected,
{
}

// Proof: kind 30 is above journal family maximum
proof fn lemma_above_journal_max_is_rejected()
    ensures
        is_journal_record_kind_spec(30int) == false,
        classify_record_kind_family_spec(JOURNAL_MAGIC, 30int)
            == LocalRecordKindFamilyDecision::Rejected,
{
}

// Proof: compatibility policy is fail-closed for mismatches
proof fn lemma_compatibility_policy_fail_closed(envelope_kind: u16, payload_kind: u16)
    requires envelope_kind != payload_kind,
    ensures
        classify_kind_compatibility_spec(envelope_kind as int, payload_kind as int)
            == LocalKindCompatibility::RejectedMismatch,
{
}

// Proof: journal family bounds are stable (10..=29)
proof fn lemma_journal_family_bounds_stable()
    ensures
        is_journal_record_kind_spec(10int) == true,
        is_journal_record_kind_spec(29int) == true,
        is_journal_record_kind_spec(9int) == false,
        is_journal_record_kind_spec(30int) == false,
{
}

} // verus!
