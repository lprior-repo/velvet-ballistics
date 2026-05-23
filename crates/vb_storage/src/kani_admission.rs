//! Kani harnesses for pure vb_storage admission value invariants.
//!
//! Storage-backed admission functions require a live `FjallJournal` handle and
//! are verified by behavior tests, not by arbitrary Kani construction of an
//! external database handle. This module keeps the Kani lane focused on pure
//! admission data invariants that Kani can model without stubs.

#![forbid(unsafe_code)]

use crate::admission::verification_proof_core;
use vb_core::WorkflowDigest;

#[kani::proof]
#[kani::unwind(40)]
fn verification_proof_digest_binding() {
    let digest: WorkflowDigest = kani::any();
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = verification_proof_core(digest, gate_count, durable);

    kani::assert(
        proof.digest == digest,
        "VerificationProof::new stores the exact input digest without transformation",
    );
}

#[kani::proof]
#[kani::unwind(40)]
fn verification_proof_digest_binding_relaxed() {
    let digest: WorkflowDigest = kani::any();
    let proof = verification_proof_core(digest, 0, false);

    kani::assert(
        proof.digest == digest,
        "VerificationProof::new with gate_count=0 still preserves digest binding",
    );
}

#[kani::proof]
#[kani::unwind(40)]
fn verification_proof_all_claim_flags_unconditional() {
    let digest: WorkflowDigest = kani::any();
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = verification_proof_core(digest, gate_count, durable);

    kani::assert(
        proof.bounded_claimed
            && proof.taint_safe_claimed
            && proof.retry_safe_claimed
            && proof.replayable_claimed
            && proof.idempotency_verified_claimed,
        "VerificationProof::new initializes every explicit _claimed flag",
    );
    kani::assert(
        proof.digest == digest,
        "Digest binding holds regardless of claim flag values",
    );
}
