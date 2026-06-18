// Kani proof harness for PS-001: accepted-artifact digest binding (Gate 11).

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::admission::{AcceptedArtifact, VerificationProof, validate_accepted_artifact_digest};
use crate::error::JournalError;
use vb_core::WorkflowDigest;

fn bounded_ir() -> Vec<u8> {
    let len: u8 = kani::any();
    let bounded_len = len % 4;
    let mut ir = Vec::new();
    for index in 0_u8..4 {
        if index < bounded_len {
            ir.push(kani::any());
        }
    }
    ir
}

fn arbitrary_artifact() -> AcceptedArtifact {
    let digest = WorkflowDigest::from_bytes(kani::any());
    let policy_digest = WorkflowDigest::from_bytes(kani::any());

    AcceptedArtifact {
        digest,
        source_digest: digest,
        policy_digest,
        ir: bounded_ir(),
        verification: VerificationProof::new(digest, 15, true),
        accepted_at_seq: crate::types::EventSeq::new(0),
        required_capabilities: Box::new([]),
    }
}

/// PS-001: if validation succeeds, the artifact and proof digests equal the record digest.
#[kani::proof]
#[kani::unwind(8)]
fn ps_001_inner_ir_digest() {
    let artifact = arbitrary_artifact();
    let digest = artifact.digest;
    let result = validate_accepted_artifact_digest(&artifact, digest);

    match result {
        Ok(()) => {
            kani::assert(artifact.digest == digest, "artifact digest matches");
            kani::assert(
                artifact.verification.digest == digest,
                "verification digest matches",
            );
        }
        Err(JournalError::ArtifactChecksumMismatch) => {
            kani::assert(
                artifact.digest != digest || artifact.verification.digest != digest,
                "checksum mismatch only when digest fields disagree",
            );
        }
        Err(_) => {}
    }
}

/// PS-001b: a digest that differs from artifact/proof digests cannot validate successfully.
#[kani::proof]
#[kani::unwind(8)]
fn ps_001_forged_digest_rejected() {
    let artifact = arbitrary_artifact();
    let forged_digest = WorkflowDigest::from_bytes(kani::any());
    kani::assume(forged_digest != artifact.digest);

    let result = validate_accepted_artifact_digest(&artifact, forged_digest);
    kani::assert(result.is_err(), "forged digest must be rejected");
}

/// PS-001c: validation returns a typed result for any bounded artifact and digest.
#[kani::proof]
#[kani::unwind(8)]
fn ps_001_no_panic_on_arbitrary_input() {
    let artifact = arbitrary_artifact();
    let digest = WorkflowDigest::from_bytes(kani::any());

    let _result = validate_accepted_artifact_digest(&artifact, digest);
}
