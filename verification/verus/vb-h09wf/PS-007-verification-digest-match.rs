// Verus proof: verification.digest == record.digest three-digest triangle.
//
// Obligation: PO-vb-h09wf-020
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-h09wf/PS-007-verification-digest-match.rs
//
// Domain claim (CS-2 clause 8):
//   The three-digest triangle: artifact.digest == verification.digest == record.digest
//   for all valid AcceptedArtifact instances.
//   When verification.digest != record.digest, Err(ArtifactChecksumMismatch).
//
// PRODUCTION BINDING:
//   vb_storage::admission::validate_accepted_artifact_digest (admission.rs:393-406)
//   Specifically: `if artifact.digest != digest || artifact.verification.digest != digest`
//
// Trusted base: VerificationProof.digest is set from workflow.digest() at construction
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-020

use vstd::prelude::*;

verus! {

/// The full Digest Triangle Invariant: artifact.digest == verification.digest == record.digest.
pub open spec fn digest_triangle_invariant(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
) -> bool {
    artifact_digest == verification_digest && verification_digest == record_digest
}

/// Lemma: If verification.digest != record.digest, the triangle fails.
pub proof fn lemma_verification_digest_mismatch_breaks_triangle(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
)
    requires
        verification_digest != record_digest,
    ensures
        !digest_triangle_invariant(artifact_digest, verification_digest, record_digest),
{
    // If the second equality fails, the conjunction fails.
    // The first equality (artifact == verification) is not sufficient alone.
}

/// Lemma: If artifact.digest != record.digest, the triangle fails regardless
/// of whether verification.digest matches artifact.digest.
pub proof fn lemma_artifact_digest_mismatch_breaks_triangle(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
)
    requires
        artifact_digest != record_digest,
    ensures
        !digest_triangle_invariant(artifact_digest, verification_digest, record_digest),
{
    // Even if verification.digest == artifact.digest, if artifact.digest != record.digest
    // then verification.digest != record.digest by transitivity.
}

/// Lemma: When all three digests are equal, the triangle holds.
pub proof fn lemma_all_equal_implies_triangle(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
)
    requires
        artifact_digest == record_digest,
        verification_digest == record_digest,
    ensures
        digest_triangle_invariant(artifact_digest, verification_digest, record_digest),
{
}

/// Lemma: Triangle transitivity — if artifact == record and verification == record,
/// then artifact == verification.
pub proof fn lemma_triangle_transitivity(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
)
    requires
        artifact_digest == record_digest,
        verification_digest == record_digest,
    ensures
        artifact_digest == verification_digest,
{
}

fn main() {}

} // verus!
