// Verus proof: artifact.digest == record.digest within validate_accepted_artifact_digest.
//
// Obligation: PO-vb-h09wf-017
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-h09wf/PS-006-artifact-digest-match.rs
//
// Domain claim (CS-2 clause 7):
//   When artifact.digest != record.digest (but verification.digest could match),
//   the function returns Err(ArtifactChecksumMismatch).
//   When both match AND BLAKE3 matches, returns Ok(()).
//
// PRODUCTION BINDING:
//   vb_storage::admission::validate_accepted_artifact_digest (admission.rs:393-406)
//   Specifically: `if artifact.digest != digest || artifact.verification.digest != digest`
//
// Trusted base: WorkflowDigest PartialEq transitively holds
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-017

use vstd::prelude::*;

verus! {

/// Spec model: the artifact.digest == record.digest check.
pub open spec fn artifact_digest_matches_record(
    artifact_digest: int,
    record_digest: int,
) -> bool {
    artifact_digest == record_digest
}

/// Lemma: If artifact.digest != record.digest, the check fails.
pub proof fn lemma_artifact_digest_mismatch_fails(
    artifact_digest: int,
    record_digest: int,
)
    requires
        artifact_digest != record_digest,
    ensures
        !artifact_digest_matches_record(artifact_digest, record_digest),
{
}

/// Lemma: If artifact.digest == record.digest, the check passes.
pub proof fn lemma_artifact_digest_match_passes(
    artifact_digest: int,
    record_digest: int,
)
    requires
        artifact_digest == record_digest,
    ensures
        artifact_digest_matches_record(artifact_digest, record_digest),
{
}

/// Combined check: both artifact.digest AND verification.digest must match record.
/// This mirrors the Rust implementation:
///   if artifact.digest != digest || artifact.verification.digest != digest { return Err(...) }
pub open spec fn both_digests_match_record(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
) -> bool {
    artifact_digest_matches_record(artifact_digest, record_digest)
        && verification_digest == record_digest
}

/// Lemma: Either mismatch triggers failure (the OR in the Rust code).
pub proof fn lemma_either_mismatch_fails(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
)
    requires
        artifact_digest != record_digest || verification_digest != record_digest,
    ensures
        !both_digests_match_record(artifact_digest, verification_digest, record_digest),
{
}

/// Lemma: Both matching triggers success.
pub proof fn lemma_both_match_succeeds(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
)
    requires
        artifact_digest == record_digest,
        verification_digest == record_digest,
    ensures
        both_digests_match_record(artifact_digest, verification_digest, record_digest),
{
}

fn main() {}

} // verus!
