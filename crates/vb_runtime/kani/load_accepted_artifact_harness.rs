//! Kani harness for KANI-POST-05: load_accepted_artifact idempotency evidence propagation.
//!
//! Obligation ID: KANI-POST-05
//! Contract clause: POST-01, POST-02
//! Risk: high
//! Verifier: kani
//!
//! This harness verifies that when load_accepted_artifact constructs a RunAdmission
//! from a VerificationProof, the idempotency_keyed and idempotency_attested fields
//! are correctly propagated with preserved lengths.
//!
//! Expected evidence: Kani completes bounded model check with no failures.
//! Command (after vb_runtime compiles): cargo kani --harness load_accepted_artifact_harness -p vb_runtime
//!
//! # Blocking
//!
//! BLOCKED - vb_runtime fails to compile due to missing chunk_001.rs (DEFERRED_GLOBAL).
//! This harness will be executable once DEFERRED_GLOBAL is resolved.
//!
//! # Findings
//!
//! - POST-01 requires that RunAdmission.idempotency_keyed.len() == VerificationProof.idempotency_keyed.len()
//! - POST-02 requires that RunAdmission.idempotency_attested.len() == VerificationProof.idempotency_attested.len()
//! - The 32 VerificationProof flag combinations determine whether idempotency semantics apply
//! - Kani exhaustively checks all flag combinations to verify correct field propagation

#![forbid(unsafe_code)]

// NOTE: This harness is BLOCKED by DEFERRED_GLOBAL (vb_runtime missing chunk_001.rs).
// Once vb_runtime compiles, uncomment and adapt the following:

/*
use vb_runtime::admission::{load_accepted_artifact, RunAdmission};
use vb_core::ids::{ActionId, WorkflowDigest};
use vb_core::storage::VerificationProof;

#[kani::proof]
fn load_accepted_artifact_harness() {
    // Non-deterministic flag values from Kani
    let durable: bool = kani::any();
    let bounded: bool = kani::any();
    let taint_safe: bool = kani::any();
    let retry_safe: bool = kani::any();
    let replayable: bool = kani::any();

    // Construct VerificationProof with non-deterministic idempotency fields
    let keyed_actions: [ActionId; 4] = [
        ActionId::new(1),
        ActionId::new(2),
        ActionId::new(3),
        ActionId::new(4),
    ];
    let attested_actions: [ActionId; 2] = [
        ActionId::new(5),
        ActionId::new(6),
    ];

    let proof = VerificationProof {
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        gate_count: 15,
        durable,
        bounded,
        taint_safe,
        retry_safe,
        replayable,
        idempotency_keyed: Box::new(keyed_actions),
        idempotency_attested: Box::new(attested_actions),
        warnings: Vec::new(),
    };

    // Load artifact (this internally constructs RunAdmission with idempotency fields)
    let result = load_accepted_artifact(proof);

    // POST-01: If load succeeds, verify idempotency_keyed length is preserved
    if let Ok(admission) = result {
        // The key invariant: idempotency_keyed.len() is preserved from VerificationProof
        // to RunAdmission. This is verified by the fact that the same Box<[ActionId]>
        // is copied (not cloned element-by-element) during construction.
        kani::assert(
            admission.idempotency_keyed.len() == 4,
            "idempotency_keyed length preserved from proof to admission",
        );
        kani::assert(
            admission.idempotency_attested.len() == 2,
            "idempotency_attested length preserved from proof to admission",
        );
    }

    // POST-02: Flag combinations don't affect field propagation - only safety semantics
    // The idempotency fields are ALWAYS propagated regardless of flag values.
    // Flags control whether replay is safe, not whether fields exist.
}
*/

/// Placeholder证明 - KANI-POST-05 blocked by DEFERRED_GLOBAL
///
/// This file documents the intended harness. The actual proof is blocked
/// because vb_runtime cannot compile (missing chunk_001.rs).
///
/// Once DEFERRED_GLOBAL is resolved:
/// 1. Uncomment the harness code above
/// 2. Run: cargo kani --harness load_accepted_artifact_harness -p vb_runtime
/// 3. Verify all 32 flag combinations pass with correct field propagation
pub fn stub_load_accepted_artifact_harness() {
    unimplemented!("Blocked by DEFERRED_GLOBAL: vb_runtime missing chunk_001.rs")
}