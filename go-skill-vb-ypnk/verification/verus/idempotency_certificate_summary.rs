// Verus proof obligations for vb-qi37.5 certificate summary soundness.
//
// Obligation: VERUS-CERT-003.
// Standalone model: each proof is identifier-local over a finite action-id
// abstraction so the certificate summary cannot pass as a count-only proof.
// Exact verifier command: `verus verification/verus/idempotency_certificate_summary.rs`.

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

fn main() {}

} // verus!
