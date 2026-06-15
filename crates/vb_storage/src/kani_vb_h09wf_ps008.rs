// Kani proof harness for PS-008: Gate count and proof flags (Gates 6-7).
//
// Obligation: PO-vb-h09wf-023
// Verifier: kani
// Command: cargo kani -p vb_storage --harness ps_008_gate_count_flags --features kani-vb-h09wf
//
// Domain claim: (a) of all 256 u8 gate_count values, only 0 and 15 pass is_accepted_gate_count;
// (b) of all 32 proof flag combinations (2^5), only the all-true combination causes
// missing_proof_flag to return None; all others return the name of the first missing flag.
//
// PRODUCTION BINDING:
//   vb_storage::admission::is_accepted_gate_count (admission.rs:475-477)
//   vb_storage::admission::missing_proof_flag (admission.rs:459-473)
//
// Trusted base: u8 exhaustive enumeration, bool exhaustive enumeration
// Model bounds: exhaustive — 256 gate_count values, 32 flag combos
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-023

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::admission::{VerificationProof, is_accepted_gate_count, missing_proof_flag};

/// PS-008a: Exhaustively verify is_accepted_gate_count for all 256 u8 values.
#[kani::proof]
#[kani::unwind(8)]
fn ps_008_gate_count_exhaustive() {
    let gate_count: u8 = kani::any();

    let accepted = is_accepted_gate_count(gate_count);

    if gate_count == 0 || gate_count == 15 {
        kani::assert(accepted, "gate_count={gate_count} must be accepted");
    } else {
        kani::assert(!accepted, "gate_count={gate_count} must be rejected");
    }

    kani::cover!(accepted, "valid gate_count accepted");
    kani::cover!(!accepted, "invalid gate_count rejected");
}

/// PS-008b: Exhaustively verify missing_proof_flag for all 32 flag combinations.
#[kani::proof]
fn ps_008_proof_flags_exhaustive() {
    let bounded: bool = kani::any();
    let taint_safe: bool = kani::any();
    let retry_safe: bool = kani::any();
    let idempotency_verified: bool = kani::any();
    let replayable: bool = kani::any();

    let proof = VerificationProof {
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
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
    };

    let missing = missing_proof_flag(&proof);

    let all_true = bounded && taint_safe && retry_safe && idempotency_verified && replayable;

    if all_true {
        kani::assert(
            missing.is_none(),
            "all flags true: missing_proof_flag must return None",
        );
    } else {
        kani::assert(
            missing.is_some(),
            "missing flag must be detected: {bounded} {taint_safe} {retry_safe} {idempotency_verified} {replayable}",
        );
        // Verify the returned flag name matches the first missing one
        let flag = match missing {
            Some(v) => v,
            None => {
                kani::assume(false);
                return;
            }
        };
        match flag {
            "bounded" => kani::assert(!bounded, "bounded was false"),
            "taint_safe" => kani::assert(
                !taint_safe && bounded,
                "taint_safe was false and bounded was true",
            ),
            "retry_safe" => {
                kani::assert(!retry_safe && bounded && taint_safe, "retry_safe was false")
            }
            "idempotency_verified" => {
                kani::assert(
                    !idempotency_verified && bounded && taint_safe && retry_safe,
                    "idempotency_verified flag check",
                );
            }
            "replayable" => {
                kani::assert(
                    !replayable && bounded && taint_safe && retry_safe && idempotency_verified,
                    "replayable flag check",
                );
            }
            _ => kani::assert(false, "unknown proof flag name"),
        }
    }

    kani::cover!(missing.is_none(), "all flags present passes");
    kani::cover!(missing.is_some(), "missing flag detected");
}
