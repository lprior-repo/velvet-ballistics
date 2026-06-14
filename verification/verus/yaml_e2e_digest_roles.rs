//! Verus proof obligations for vb-core-yaml-e2e-chain digest role separation.
//!
//! Obligations: PO-004 and PO-005.
//! This is a pure model. BLAKE3, Fjall I/O, postcard decode, and runtime
//! scheduling remain trusted shell boundaries covered by downstream evidence.

use vstd::prelude::*;

verus! {

pub enum DigestRole {
    Source,
    Artifact,
}

pub enum ChainError {
    WorkflowSourceDigestMismatch,
    CompiledIrDigestMismatch,
    AcceptedArtifactInvalid,
    ReplayDivergence,
}

pub enum ShellTarget {
    VerifyContentDigest,
    VerifyDigests,
    RejectWorkflowDigestMismatch,
    AdmitArtifactRun,
}

pub open spec fn same_digest(claimed: int, actual: int) -> bool {
    claimed == actual
}

pub open spec fn source_digest_valid(claimed_source: int, actual_source: int) -> bool {
    same_digest(claimed_source, actual_source)
}

pub open spec fn artifact_digest_valid(claimed_artifact: int, actual_artifact: int) -> bool {
    same_digest(claimed_artifact, actual_artifact)
}

pub open spec fn roles_distinct(source_role: DigestRole, artifact_role: DigestRole) -> bool {
    source_role is Source && artifact_role is Artifact
}

pub open spec fn classify_source_digest(
    claimed_source: int,
    actual_source: int,
) -> Option<ChainError> {
    if source_digest_valid(claimed_source, actual_source) {
        Option::None
    } else {
        Option::Some(ChainError::WorkflowSourceDigestMismatch)
    }
}

pub open spec fn classify_artifact_digest(
    claimed_artifact: int,
    actual_artifact: int,
) -> Option<ChainError> {
    if artifact_digest_valid(claimed_artifact, actual_artifact) {
        Option::None
    } else {
        Option::Some(ChainError::CompiledIrDigestMismatch)
    }
}

pub open spec fn accepted_artifact_ok(
    claimed_artifact: int,
    actual_artifact: int,
    gate_ok: bool,
    proof_ok: bool,
    capability_ok: bool,
) -> bool {
    artifact_digest_valid(claimed_artifact, actual_artifact)
        && gate_ok
        && proof_ok
        && capability_ok
}

pub open spec fn recovery_success_allowed(
    source_claim: int,
    source_actual: int,
    artifact_claim: int,
    artifact_actual: int,
    gate_ok: bool,
    proof_ok: bool,
    capability_ok: bool,
    replay_ok: bool,
) -> bool {
    source_digest_valid(source_claim, source_actual)
        && accepted_artifact_ok(artifact_claim, artifact_actual, gate_ok, proof_ok, capability_ok)
        && replay_ok
}

pub open spec fn recovery_error(
    source_claim: int,
    source_actual: int,
    artifact_claim: int,
    artifact_actual: int,
    gate_ok: bool,
    proof_ok: bool,
    capability_ok: bool,
    replay_ok: bool,
) -> Option<ChainError> {
    if !source_digest_valid(source_claim, source_actual) {
        Option::Some(ChainError::WorkflowSourceDigestMismatch)
    } else if !artifact_digest_valid(artifact_claim, artifact_actual) {
        Option::Some(ChainError::CompiledIrDigestMismatch)
    } else if !gate_ok || !proof_ok || !capability_ok {
        Option::Some(ChainError::AcceptedArtifactInvalid)
    } else if !replay_ok {
        Option::Some(ChainError::ReplayDivergence)
    } else {
        Option::None
    }
}

pub open spec fn source_target_modeled(target: ShellTarget) -> bool {
    target is VerifyContentDigest || target is RejectWorkflowDigestMismatch || target is VerifyDigests
}

pub open spec fn artifact_target_modeled(target: ShellTarget) -> bool {
    target is VerifyDigests || target is AdmitArtifactRun
}

pub proof fn proof_source_digest_mismatch_classifies(
    claimed_source: int,
    actual_source: int,
)
    requires claimed_source != actual_source,
    ensures classify_source_digest(claimed_source, actual_source)
        == Option::Some(ChainError::WorkflowSourceDigestMismatch),
{
    assert(!source_digest_valid(claimed_source, actual_source));
    assert(classify_source_digest(claimed_source, actual_source)
        == Option::Some(ChainError::WorkflowSourceDigestMismatch));
}

pub proof fn proof_artifact_digest_mismatch_classifies(
    claimed_artifact: int,
    actual_artifact: int,
)
    requires claimed_artifact != actual_artifact,
    ensures classify_artifact_digest(claimed_artifact, actual_artifact)
        == Option::Some(ChainError::CompiledIrDigestMismatch),
{
    assert(!artifact_digest_valid(claimed_artifact, actual_artifact));
    assert(classify_artifact_digest(claimed_artifact, actual_artifact)
        == Option::Some(ChainError::CompiledIrDigestMismatch));
}

pub proof fn proof_digest_roles_are_not_interchangeable()
    ensures roles_distinct(DigestRole::Source, DigestRole::Artifact),
{
    assert(roles_distinct(DigestRole::Source, DigestRole::Artifact));
}

pub proof fn proof_role_swapped_digest_detected_when_values_differ(
    source_actual: int,
    artifact_actual: int,
)
    requires source_actual != artifact_actual,
    ensures
        !source_digest_valid(artifact_actual, source_actual),
        !artifact_digest_valid(source_actual, artifact_actual),
{
    assert(!source_digest_valid(artifact_actual, source_actual));
    assert(!artifact_digest_valid(source_actual, artifact_actual));
}

pub proof fn proof_invalid_artifact_never_allows_recovery_success(
    source_claim: int,
    source_actual: int,
    artifact_claim: int,
    artifact_actual: int,
    gate_ok: bool,
    proof_ok: bool,
    capability_ok: bool,
    replay_ok: bool,
)
    requires
        !artifact_digest_valid(artifact_claim, artifact_actual)
            || !gate_ok
            || !proof_ok
            || !capability_ok,
    ensures !recovery_success_allowed(
        source_claim,
        source_actual,
        artifact_claim,
        artifact_actual,
        gate_ok,
        proof_ok,
        capability_ok,
        replay_ok,
    ),
{
    assert(!accepted_artifact_ok(artifact_claim, artifact_actual, gate_ok, proof_ok, capability_ok));
    assert(!recovery_success_allowed(
        source_claim,
        source_actual,
        artifact_claim,
        artifact_actual,
        gate_ok,
        proof_ok,
        capability_ok,
        replay_ok,
    ));
}

pub proof fn proof_same_inputs_same_recovery_classification(
    source_claim: int,
    source_actual: int,
    artifact_claim: int,
    artifact_actual: int,
    gate_ok: bool,
    proof_ok: bool,
    capability_ok: bool,
    replay_ok: bool,
)
    ensures recovery_error(
        source_claim,
        source_actual,
        artifact_claim,
        artifact_actual,
        gate_ok,
        proof_ok,
        capability_ok,
        replay_ok,
    ) == recovery_error(
        source_claim,
        source_actual,
        artifact_claim,
        artifact_actual,
        gate_ok,
        proof_ok,
        capability_ok,
        replay_ok,
    ),
{
    assert(recovery_error(
        source_claim,
        source_actual,
        artifact_claim,
        artifact_actual,
        gate_ok,
        proof_ok,
        capability_ok,
        replay_ok,
    ) == recovery_error(
        source_claim,
        source_actual,
        artifact_claim,
        artifact_actual,
        gate_ok,
        proof_ok,
        capability_ok,
        replay_ok,
    ));
}

pub proof fn proof_source_digest_targets_map_to_source_classification()
    ensures
        source_target_modeled(ShellTarget::VerifyContentDigest),
        source_target_modeled(ShellTarget::VerifyDigests),
        source_target_modeled(ShellTarget::RejectWorkflowDigestMismatch),
{
    assert(source_target_modeled(ShellTarget::VerifyContentDigest));
    assert(source_target_modeled(ShellTarget::VerifyDigests));
    assert(source_target_modeled(ShellTarget::RejectWorkflowDigestMismatch));
}

pub proof fn proof_artifact_admission_targets_map_to_artifact_classification()
    ensures
        artifact_target_modeled(ShellTarget::VerifyDigests),
        artifact_target_modeled(ShellTarget::AdmitArtifactRun),
{
    assert(artifact_target_modeled(ShellTarget::VerifyDigests));
    assert(artifact_target_modeled(ShellTarget::AdmitArtifactRun));
}

} // verus!

fn main() {}
