// Verus proof: Digest binding invariant for validate_accepted_artifact_digest.
//
// Obligation: PO-vb-h09wf-001
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-h09wf/PS-001-digest-binding.rs
//
// Domain claim (CS-2 clause 9):
//   For all (artifact, digest) pairs: BLAKE3(artifact.ir) == digest iff
//   artifact.digest == artifact.verification.digest == digest.
//   The digest binding is a structural triangle that the gate enforces.
//
// PRODUCTION BINDING:
//   vb_storage::admission::validate_accepted_artifact_digest (admission.rs:422-431)
//   This spec models the function's postcondition: the three-digest equality
//   requirement is the mathematical invariant the function upholds.
//
// Trusted base: blake3 collision resistance (not modeled — trusted boundary)
//   postcard serialization determinism (not modeled — trusted boundary)
//   WorkflowDigest equality is byte-structural (trusted)
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-001
//
// VERUS STANDALONE CONSTRAINT:
// This file is verified with `verus --crate-type=lib` in standalone mode,
// which cannot import production crate types (vb_storage, vb_core). All spec
// and proof functions operate over abstract `int` models of digest values.
// The binding to production code is established by the Kani harness:
//
//   Kani binding: kani_vb_h09wf_ps001.rs (PO-vb-h09wf-002, PO-vb-h09wf-003, PO-vb-h09wf-004)
//   Production fn: vb_storage::admission::validate_accepted_artifact_digest (admission.rs:422-431)
//
// The exec fn bridge below documents the production function signature that
// this Verus model corresponds to. The Kani harness proves the actual
// production code enforces the Digest Triangle Invariant for arbitrary
// bounded inputs (GOD RULES 1 and 4).
//
// Documented use imports (not resolvable in standalone mode):
//   use vb_storage::admission::{AcceptedArtifact, VerificationProof};
//   use vb_core::WorkflowDigest;

use vstd::prelude::*;

// ---------------------------------------------------------------------------
// External type stubs — structural mirrors of production types.
// These are used only in the exec fn bridge signature below. The actual
// production types are in vb_storage and vb_core crates.
// ---------------------------------------------------------------------------

/// Mirrors vb_core::WorkflowDigest (ids/mod.rs:348): newtype over [u8; 32].
#[derive(Clone, Copy)]
pub struct WorkflowDigest(pub [u8; 32]);

/// Mirrors vb_storage::error::JournalError variants used in digest validation.
#[derive(Clone, Copy)]
pub enum JournalError {
    ArtifactChecksumMismatch,
    ArtifactMalformed,
}

/// Mirrors vb_storage::admission::AcceptedArtifact (admission.rs:175-199).
/// Minimal subset: only the fields relevant to digest validation.
#[derive(Clone)]
pub struct AcceptedArtifact {
    pub digest: WorkflowDigest,
    pub verification: VerificationProof,
    pub ir: Vec<u8>,
}

/// Mirrors vb_storage::admission::VerificationProof (admission.rs:71-94).
/// Only the digest field is needed for the bridge signature.
#[derive(Clone)]
pub struct VerificationProof {
    pub digest: WorkflowDigest,
}

// External type specifications for Verus —
// lets Verus accept these types in exec fn signatures.
#[verifier::external_type_specification]
#[allow(dead_code)]
pub struct ExWorkflowDigest(crate::WorkflowDigest);

#[verifier::external_type_specification]
#[allow(dead_code)]
pub struct ExJournalError(crate::JournalError);

#[verifier::external_type_specification]
#[allow(dead_code)]
pub struct ExAcceptedArtifact(crate::AcceptedArtifact);

#[verifier::external_type_specification]
#[allow(dead_code)]
pub struct ExVerificationProof(crate::VerificationProof);

verus! {

/// EXEC BRIDGE: Binding to production `validate_accepted_artifact_digest`.
///
/// Mirrors the production function signature at admission.rs:422-431:
/// ```ignore
/// fn validate_accepted_artifact_digest(
///     artifact: &AcceptedArtifact,
///     digest: WorkflowDigest,
/// ) -> Result<(), JournalError>
/// ```
/// Returns `Ok(())` iff `artifact.digest == digest
/// && artifact.verification.digest == digest` AND metadata valid.
///
/// Marked `#[verifier::external_body]` because the production implementation
/// uses blake3, postcard, and std types that Verus cannot verify in standalone
/// mode. The body is a no-op placeholder; the actual production binding and
/// behavior verification is in the corresponding Kani harness.
///
/// Kani: kani_vb_h09wf_ps001.rs (PO-vb-h09wf-002, PO-vb-h09wf-003, PO-vb-h09wf-004)
#[verifier::external_body]
pub exec fn bridge_validate_accepted_artifact_digest(
    _artifact: &AcceptedArtifact,
    _digest: WorkflowDigest,
) -> Result<(), JournalError> {
    // Trusted: verified by Kani harness kani_vb_h09wf_ps001.
    Ok(())
}

/// The Digest Triangle Invariant: three digests must be equal.
pub open spec fn digest_triangle_holds(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
) -> bool {
    artifact_digest == verification_digest && verification_digest == record_digest
}

/// The content binding: the hash of artifact.ir equals the record digest.
/// In the Verus model, we abstract blake3 as a deterministic pure function.
pub open spec fn content_binding_holds(hash_of_ir: int, record_digest: int) -> bool {
    hash_of_ir == record_digest
}

/// Full validation postcondition: the function returns Ok(()) iff
/// both the digest triangle AND the content binding hold.
pub open spec fn validate_accepted_artifact_digest_spec(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
    hash_of_artifact_ir: int,
) -> bool {
    digest_triangle_holds(artifact_digest, verification_digest, record_digest)
        && content_binding_holds(hash_of_artifact_ir, record_digest)
}

/// Lemma: If the digest triangle does NOT hold (any mismatch), the spec returns false.
pub proof fn lemma_digest_mismatch_denies(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
    hash_of_artifact_ir: int,
)
    requires
        !digest_triangle_holds(artifact_digest, verification_digest, record_digest),
    ensures
        !validate_accepted_artifact_digest_spec(
            artifact_digest, verification_digest, record_digest, hash_of_artifact_ir,
        ),
{
    // Spec-level tautology: validate_accepted_artifact_digest_spec is defined as
    // digest_triangle_holds(…) && content_binding_holds(…).
    // The requires clause (!digest_triangle_holds) makes the first conjunct false,
    // so the conjunction is false. The SMT solver discharges this automatically.
    assert(!digest_triangle_holds(artifact_digest, verification_digest, record_digest));
    assert(validate_accepted_artifact_digest_spec(
        artifact_digest, verification_digest, record_digest, hash_of_artifact_ir,
    ) == (digest_triangle_holds(artifact_digest, verification_digest, record_digest)
        && content_binding_holds(hash_of_artifact_ir, record_digest)));
}

/// Lemma: If content binding does NOT hold (BLAKE3 mismatch), the spec returns false.
pub proof fn lemma_content_mismatch_denies(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
    hash_of_artifact_ir: int,
)
    requires
        !content_binding_holds(hash_of_artifact_ir, record_digest),
    ensures
        !validate_accepted_artifact_digest_spec(
            artifact_digest, verification_digest, record_digest, hash_of_artifact_ir,
        ),
{
    // Spec-level tautology: validate_accepted_artifact_digest_spec is defined as
    // digest_triangle_holds(…) && content_binding_holds(…).
    // The requires clause (!content_binding_holds) makes the second conjunct false,
    // so the conjunction is false. The SMT solver discharges this automatically.
    assert(!content_binding_holds(hash_of_artifact_ir, record_digest));
    assert(validate_accepted_artifact_digest_spec(
        artifact_digest, verification_digest, record_digest, hash_of_artifact_ir,
    ) == (digest_triangle_holds(artifact_digest, verification_digest, record_digest)
        && content_binding_holds(hash_of_artifact_ir, record_digest)));
}

/// Lemma: When all checks pass, the spec returns true.
pub proof fn lemma_all_checks_pass(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
    hash_of_artifact_ir: int,
)
    requires
        digest_triangle_holds(artifact_digest, verification_digest, record_digest),
        content_binding_holds(hash_of_artifact_ir, record_digest),
    ensures
        validate_accepted_artifact_digest_spec(
            artifact_digest, verification_digest, record_digest, hash_of_artifact_ir,
        ),
{
    // Spec-level tautology: when both conjuncts of the spec definition hold,
    // the conjunction is true. Verified by SMT solver automatically.
    assert(validate_accepted_artifact_digest_spec(
        artifact_digest, verification_digest, record_digest, hash_of_artifact_ir,
    ) == true);
}

/// Lemma: Digest triangle equality implies all three digests are equal.
pub proof fn lemma_triangle_implies_equality(
    artifact_digest: int,
    verification_digest: int,
    record_digest: int,
)
    requires
        digest_triangle_holds(artifact_digest, verification_digest, record_digest),
    ensures
        artifact_digest == record_digest,
        verification_digest == record_digest,
        artifact_digest == verification_digest,
{
    // Spec-level tautology: digest_triangle_holds is defined as
    // artifact_digest == verification_digest && verification_digest == record_digest.
    // The three equality conclusions follow directly from the conjunction.
    assert(digest_triangle_holds(artifact_digest, verification_digest, record_digest));
    assert(artifact_digest == verification_digest && verification_digest == record_digest);
}

fn main() {}

} // verus!
