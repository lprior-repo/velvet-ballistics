// Verus verifier-only model for vb-core-cli-accepted-path PO-003.
// Trusted shell boundary: final runtime constructor names and storage handle type.

use vstd::prelude::*;

verus! {

pub enum Policy {
    Strict,
    Journaled,
    Relaxed,
}

pub enum Witness {
    StorageAcceptedArtifact,
    RawWorkflowParts,
    RawCompiledWorkflow,
    AlwaysPresentStore,
}

pub open spec fn strict_like(policy: Policy) -> bool {
    match policy {
        Policy::Strict => true,
        Policy::Journaled => true,
        Policy::Relaxed => false,
    }
}

pub open spec fn storage_backed(witness: Witness) -> bool {
    match witness {
        Witness::StorageAcceptedArtifact => true,
        Witness::RawWorkflowParts => false,
        Witness::RawCompiledWorkflow => false,
        Witness::AlwaysPresentStore => false,
    }
}

pub open spec fn valid_admission_witness(policy: Policy, witness: Witness) -> bool {
    strict_like(policy) ==> storage_backed(witness)
}

pub proof fn proof_strict_requires_storage(witness: Witness)
    requires
        valid_admission_witness(Policy::Strict, witness),
    ensures
        storage_backed(witness),
{
}

pub proof fn proof_journaled_requires_storage(witness: Witness)
    requires
        valid_admission_witness(Policy::Journaled, witness),
    ensures
        storage_backed(witness),
{
}

pub proof fn proof_raw_parts_not_strict_witness()
    ensures
        !valid_admission_witness(Policy::Strict, Witness::RawWorkflowParts),
        !valid_admission_witness(Policy::Journaled, Witness::RawWorkflowParts),
{
}

pub proof fn proof_raw_compiled_not_strict_witness()
    ensures
        !valid_admission_witness(Policy::Strict, Witness::RawCompiledWorkflow),
        !valid_admission_witness(Policy::Journaled, Witness::RawCompiledWorkflow),
{
}

pub proof fn proof_always_present_not_strict_witness()
    ensures
        !valid_admission_witness(Policy::Strict, Witness::AlwaysPresentStore),
        !valid_admission_witness(Policy::Journaled, Witness::AlwaysPresentStore),
{
}

pub proof fn proof_storage_artifact_satisfies_strict_witness()
    ensures
        valid_admission_witness(Policy::Strict, Witness::StorageAcceptedArtifact),
        valid_admission_witness(Policy::Journaled, Witness::StorageAcceptedArtifact),
{
}

} // verus!

fn main() {}
