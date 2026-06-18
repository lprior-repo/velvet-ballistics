// Kani proof harness for PS-008: gate count and proof flags (Gates 6-7).

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::admission::{VerificationProof, is_accepted_gate_count, missing_proof_flag};

fn proof_with_flags(
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    idempotency_verified: bool,
    replayable: bool,
) -> VerificationProof {
    VerificationProof {
        digest: vb_core::WorkflowDigest::from_bytes([0_u8; 32]),
        gate_count: 15,
        durable: true,
        bounded_claimed: bounded,
        taint_safe_claimed: taint_safe,
        retry_safe_claimed: retry_safe,
        idempotency_verified_claimed: idempotency_verified,
        replayable_claimed: replayable,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: Vec::new(),
    }
}

/// PS-008a: all u8 gate counts satisfy the accepted-count predicate exactly.
#[kani::proof]
fn ps_008_gate_count_exhaustive() {
    let gate_count: u8 = kani::any();
    let accepted = is_accepted_gate_count(gate_count);

    if gate_count == 0 || gate_count == 15 {
        kani::assert(accepted, "zero and full gate counts are accepted");
    } else {
        kani::assert(!accepted, "other gate counts are rejected");
    }
}

/// PS-008b: proof flag validation reports the first missing proof flag.
#[kani::proof]
fn ps_008_proof_flags_exhaustive() {
    let bounded: bool = kani::any();
    let taint_safe: bool = kani::any();
    let retry_safe: bool = kani::any();
    let idempotency_verified: bool = kani::any();
    let replayable: bool = kani::any();

    let proof = proof_with_flags(
        bounded,
        taint_safe,
        retry_safe,
        idempotency_verified,
        replayable,
    );
    let missing = missing_proof_flag(&proof);

    if bounded && taint_safe && retry_safe && idempotency_verified && replayable {
        kani::assert(missing.is_none(), "all proof flags set returns None");
    } else {
        match missing {
            Some("bounded") => kani::assert(!bounded, "bounded is first missing flag"),
            Some("taint_safe") => {
                kani::assert(bounded && !taint_safe, "taint_safe is first missing flag");
            }
            Some("retry_safe") => {
                kani::assert(
                    bounded && taint_safe && !retry_safe,
                    "retry_safe is first missing flag",
                );
            }
            Some("idempotency_verified") => {
                kani::assert(
                    bounded && taint_safe && retry_safe && !idempotency_verified,
                    "idempotency_verified is first missing flag",
                );
            }
            Some("replayable") => {
                kani::assert(
                    bounded && taint_safe && retry_safe && idempotency_verified && !replayable,
                    "replayable is first missing flag",
                );
            }
            Some(_) => kani::assert(false, "unknown proof flag name"),
            None => kani::assert(false, "missing flag must be reported"),
        }
    }
}
