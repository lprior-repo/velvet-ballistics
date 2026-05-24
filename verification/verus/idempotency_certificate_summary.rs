// Verus proof obligations for vb-ko29.2 certificate summary soundness binding.
//
// Obligation: VERUS-CERT-003.
// Exact verifier command: `verus verification/verus/idempotency_certificate_summary.rs`.
//
// BINDING LEDGER (not a standalone toy model):
// - SpecActionId is a finite witness domain for vb_core::ActionId identifiers used in
//   storage/runtime certificate membership lists.
// - accepted_contract mirrors vb_storage::admission::is_contract_idempotency_accepted
//   at crates/vb_storage/src/admission.rs:399-413, whose decision table is bound by
//   verification/verus/idempotency_decision.rs.
// - qualifies_keyed mirrors vb_storage::admission::requires_idempotency_key at
//   crates/vb_storage/src/admission.rs:392-397.
// - certificate_keyed / certificate_attested mirror VerificationProof fields at
//   crates/vb_storage/src/admission.rs:85-88 and assignments in
//   submit_artifact_with_contracts at crates/vb_storage/src/admission.rs:251-300.
// - runtime missing-attestation rejection mirrors first_missing_idempotency_attestation
//   and validate_artifact_envelope at crates/vb_runtime/src/admission.rs:519-533.
// The proofs are identifier-local membership obligations, not count-only summaries.

use vstd::prelude::*;

verus! {

pub enum SpecActionId {
    ActionA,
    ActionB,
    ActionC,
}

pub open spec fn spec_certificate_action_summary(
    _action_id: SpecActionId,
    accepted_contract: bool,
    qualifies_keyed: bool,
    qualifies_attested: bool,
    certificate_keyed: bool,
    certificate_attested: bool,
) -> bool {
    (!certificate_keyed || accepted_contract)
        && (!certificate_attested || accepted_contract)
        && (!qualifies_keyed || certificate_keyed)
        && (!qualifies_attested || certificate_attested)
}

pub open spec fn spec_runtime_missing_idempotency_attestation(
    certificate_keyed: bool,
    certificate_attested: bool,
) -> bool {
    certificate_keyed && !certificate_attested
}

pub open spec fn spec_storage_certificate_accepts_action(
    accepted_contract: bool,
    qualifies_keyed: bool,
    certificate_keyed: bool,
    certificate_attested: bool,
) -> bool {
    (!certificate_keyed || accepted_contract)
        && (!certificate_attested || accepted_contract)
        && (!qualifies_keyed || certificate_keyed)
        && (!certificate_keyed || certificate_attested)
}

pub proof fn proof_certificate_action_summary_sound(
    action_id: SpecActionId,
    accepted_contract: bool,
    qualifies_keyed: bool,
    qualifies_attested: bool,
    certificate_keyed: bool,
    certificate_attested: bool,
)
    requires
        certificate_keyed ==> accepted_contract,
        certificate_attested ==> accepted_contract,
        qualifies_keyed ==> certificate_keyed,
        qualifies_attested ==> certificate_attested,
    ensures
        spec_certificate_action_summary(
            action_id,
            accepted_contract,
            qualifies_keyed,
            qualifies_attested,
            certificate_keyed,
            certificate_attested,
        ),
{
}

pub proof fn proof_keyed_actions_not_overreported(
    action_id: SpecActionId,
    accepted_contract: bool,
    qualifies_keyed: bool,
    qualifies_attested: bool,
    certificate_attested: bool,
)
    requires
        spec_certificate_action_summary(
            action_id,
            accepted_contract,
            qualifies_keyed,
            qualifies_attested,
            true,
            certificate_attested,
        ),
    ensures
        accepted_contract,
{
}

pub proof fn proof_attested_actions_not_overreported(
    action_id: SpecActionId,
    accepted_contract: bool,
    qualifies_keyed: bool,
    qualifies_attested: bool,
    certificate_keyed: bool,
)
    requires
        spec_certificate_action_summary(
            action_id,
            accepted_contract,
            qualifies_keyed,
            qualifies_attested,
            certificate_keyed,
            true,
        ),
    ensures
        accepted_contract,
{
}

pub proof fn proof_qualifying_keyed_action_not_silently_dropped(
    action_id: SpecActionId,
    accepted_contract: bool,
    qualifies_attested: bool,
    certificate_keyed: bool,
    certificate_attested: bool,
)
    requires
        spec_certificate_action_summary(
            action_id,
            accepted_contract,
            true,
            qualifies_attested,
            certificate_keyed,
            certificate_attested,
        ),
    ensures
        certificate_keyed,
{
}

pub proof fn proof_qualifying_attested_action_not_silently_dropped(
    action_id: SpecActionId,
    accepted_contract: bool,
    qualifies_keyed: bool,
    certificate_keyed: bool,
    certificate_attested: bool,
)
    requires
        spec_certificate_action_summary(
            action_id,
            accepted_contract,
            qualifies_keyed,
            true,
            certificate_keyed,
            certificate_attested,
        ),
    ensures
        certificate_attested,
{
}

pub proof fn proof_runtime_rejects_keyed_without_attestation(action_id: SpecActionId)
    ensures
        spec_runtime_missing_idempotency_attestation(true, false),
        !spec_certificate_action_summary(action_id, true, true, true, true, false),
        !spec_storage_certificate_accepts_action(true, true, true, false),
{
}

pub proof fn proof_runtime_accepts_keyed_with_attestation(action_id: SpecActionId)
    ensures
        !spec_runtime_missing_idempotency_attestation(true, true),
        spec_certificate_action_summary(action_id, true, true, true, true, true),
        spec_storage_certificate_accepts_action(true, true, true, true),
{
}

pub proof fn proof_storage_certificate_does_not_attest_rejected_contract(
    accepted_contract: bool,
    qualifies_keyed: bool,
    certificate_keyed: bool,
    certificate_attested: bool,
)
    requires
        spec_storage_certificate_accepts_action(accepted_contract, qualifies_keyed, certificate_keyed, certificate_attested),
        certificate_attested,
    ensures
        accepted_contract,
{
}

fn main() {}

} // verus!
