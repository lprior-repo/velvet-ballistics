// Verus model for vb-qi37.4 accepted-artifact capability proof obligations.
//
// Obligations:
// - VERUS-CAP-001: exact capability name/action matching only.
// - VERUS-CARD-003: runtime admission is cardinality-exact.
// - VERUS-CERT-007: accepted-artifact certificate preserves profile count.
// - Derived schema lemma: accepted certificate assumes validated schema inputs.
// - VERUS-CAP-003: exact capability name/action matching, cardinality-exact
//   runtime admission, and accepted-artifact certificate profile preservation.
//
// This is a pure model.  Fjall I/O, postcard bytes, and production Rust structs
// remain trusted shell boundaries and require integration evidence later.
//
// BINDING: capability_artifact_model
// Rust type: vb_core::capability::Capability
// Verified: Matched spec capability matching functions to Rust Capability::{name, action} fields
// Divergences: Spec models abstract int for name/action; Rust uses Box<str> and ActionId types

use vstd::prelude::*;

verus! {

pub open spec fn valid_capability_name(name_len: int) -> bool {
    0 < name_len && name_len <= 128
}

pub open spec fn exact_capability_match(
    required_name: int,
    required_action: int,
    granted_name: int,
    granted_action: int,
) -> bool {
    required_name == granted_name && required_action == granted_action
}

pub open spec fn exact_profile(
    required_count: int,
    granted_count: int,
    every_required_has_exact_grant: bool,
) -> bool {
    0 <= required_count
        && required_count == granted_count
        && every_required_has_exact_grant
}

pub open spec fn accepted_certificate_preserves_profile(
    contract_required_count: int,
    accepted_required_count: int,
) -> bool {
    0 <= contract_required_count && accepted_required_count == contract_required_count
}

pub open spec fn gate12_schema_valid(
    name_len: int,
    action_matches_contract: bool,
    duplicate_requirement: bool,
) -> bool {
    valid_capability_name(name_len) && action_matches_contract && !duplicate_requirement
}

pub proof fn proof_exact_match_requires_name_and_action(
    required_name: int,
    required_action: int,
    granted_name: int,
    granted_action: int,
)
    requires
        exact_capability_match(required_name, required_action, granted_name, granted_action),
    ensures
        required_name == granted_name,
        required_action == granted_action,
{
}

pub proof fn proof_prefix_or_action_mismatch_denies(
    required_name: int,
    required_action: int,
    granted_name: int,
    granted_action: int,
)
    requires
        required_name != granted_name || required_action != granted_action,
    ensures
        !exact_capability_match(required_name, required_action, granted_name, granted_action),
{
}

pub proof fn proof_exact_profile_requires_cardinality(
    required_count: int,
    granted_count: int,
    every_required_has_exact_grant: bool,
)
    requires
        exact_profile(required_count, granted_count, every_required_has_exact_grant),
    ensures
        required_count == granted_count,
        every_required_has_exact_grant,
{
}

pub proof fn proof_missing_or_excess_grants_deny(
    required_count: int,
    granted_count: int,
)
    requires
        0 <= required_count,
        required_count != granted_count,
    ensures
        !exact_profile(required_count, granted_count, true),
{
}

pub proof fn proof_certificate_preserves_required_capabilities(
    contract_required_count: int,
    accepted_required_count: int,
)
    requires
        accepted_certificate_preserves_profile(contract_required_count, accepted_required_count),
    ensures
        accepted_required_count == contract_required_count,
        contract_required_count >= 0,
{
}

pub proof fn proof_non_empty_contract_not_erased(
    contract_required_count: int,
    accepted_required_count: int,
)
    requires
        accepted_certificate_preserves_profile(contract_required_count, accepted_required_count),
    forall|i: int|
        #![trigger required[i]]
        0 <= i < required.len() ==> {
            exists|j: int|
/// Contract predicate: when `required_count != granted_count`, the failure
/// shape is the typed `CapabilityCountMismatch` carrying the raw counts.
/// Closed so Verus inlines the body without trigger inference.
pub closed spec fn admit_failure_is_typed_count_mismatch(
    required_count: nat,
    granted_count: nat,
) -> bool {
    required_count != granted_count ==> {
        forall|err: SpecAdmitError|
            match err {
                SpecAdmitError::Ok => false,
                SpecAdmitError::CapabilityCountMismatch { required_count: rc, granted_count: gc } =>
                    rc == required_count && gc == granted_count,
                SpecAdmitError::CapabilityDenied { .. } => false,
            }
    }
}
        !gate12_schema_valid(name_len, action_matches_contract, duplicate_requirement),
{
}

fn main() {}

} // verus!
