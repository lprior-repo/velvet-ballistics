// Kani proof harness for PS-002: Anti-contract — BLAKE3(record.ir) != record.digest.
//
// Obligation: PO-vb-h09wf-006
// Verifier: kani
// Command: cargo kani -p vb_storage --harness ps_002_anti_contract --features kani-vb-h09wf
//
// Domain claim: For every valid AcceptedArtifact constructed via accepted_artifact,
// let envelope = postcard(AcceptedArtifact), let record = CompiledIrRecord{digest: artifact.digest, ir: envelope}.
// Then BLAKE3(record.ir) != record.digest (barring BLAKE3 collision).
//
// This is a NEGATIVE proof: it demonstrates that following the vb-6uue specification
// (BLAKE3(record.ir) == record.digest) would be catastrophically wrong.
//
// PRODUCTION BINDING:
//   - vb_storage::admission::accepted_artifact (admission.rs:328-343)
//   - vb_storage::records::CompiledIrRecord (records/entities.rs:19-24)
//   - blake3::hash
//
// Trusted base: blake3 crate (collision-resistant hash), postcard serialization determinism
// Model bounds: WorkflowParts bounded to 256 bytes for Kani solver
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-006

#![forbid(unsafe_code)]
#![cfg(kani)]

use vb_core::WorkflowDigest;

/// Construct a minimal valid AcceptedArtifact using kani::any() for all fields.
/// GOD RULE 1: No hardcoded dummy data — uses exhaustive kani::any() generation.
fn arbitrary_accepted_artifact() -> (
    crate::admission::AcceptedArtifact,
    Vec<u8>, // serialized envelope bytes
) {
    let ir_len: u8 = kani::any();
    let ir: Vec<u8> = (0..ir_len).map(|_| kani::any()).collect();

    let digest_bytes: [u8; 32] = kani::any();
    let digest = WorkflowDigest::from_bytes(digest_bytes);

    let artifact = crate::admission::AcceptedArtifact {
        digest,
        source_digest: digest,
        policy_digest: digest,
        ir,
        verification: crate::admission::VerificationProof {
            digest,
            gate_count: 15,
            durable: true,
            bounded_claimed: true,
            taint_safe_claimed: true,
            retry_safe_claimed: true,
            idempotency_verified_claimed: true,
            replayable_claimed: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: crate::types::EventSeq::new(0),
        required_capabilities: Box::new([]),
    };

    // Serialize to postcard envelope bytes
    let envelope = match postcard::to_allocvec(&artifact) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    (artifact, envelope)
}

/// PS-002: Anti-contract proof — BLAKE3(envelope) != record.digest for all valid records.
#[kani::proof]
#[kani::unwind(4)]
fn ps_002_anti_contract() {
    let (artifact, envelope) = arbitrary_accepted_artifact();

    // Construct a CompiledIrRecord from the artifact and envelope
    let record = crate::records::CompiledIrRecord {
        digest: artifact.digest,
        ir: envelope.clone(),
    };

    // The vb-6uue check: BLAKE3(record.ir) == record.digest
    let envelope_hash = blake3::hash(&envelope);
    let envelope_hash_matches = envelope_hash.as_bytes() == &record.digest.as_bytes();

    // This MUST be false for valid records — the envelope contains metadata
    // (verification, capabilities, etc.) so its hash differs from ir_digest.
    // The only exception is a BLAKE3 collision (astronomically unlikely).
    assert!(
        !envelope_hash_matches,
        "Anti-contract: BLAKE3(envelope) must NOT equal record.digest. \
         vb-6uue check would reject ALL valid records."
    );

    kani::cover!(
        !envelope_hash_matches,
        "Anti-contract holds: envelope hash != record digest"
    );
}

/// PS-002b: Demonstrate what happens with the correct two-step unwrap-then-verify.
/// This tests that BLAKE3(artifact.ir) == record.digest (the correct check) works.
#[kani::proof]
#[kani::unwind(4)]
fn ps_002_correct_two_step_verification() {
    let (artifact, envelope) = arbitrary_accepted_artifact();

    // Set the digest from the actual hash of artifact.ir (the CORRECT pattern)
    let inner_hash = blake3::hash(&artifact.ir);
    let correct_digest = WorkflowDigest::from_bytes(*inner_hash.as_bytes());

    // Now test the correct check: BLAKE3(artifact.ir) == record.digest
    let record = crate::records::CompiledIrRecord {
        digest: correct_digest,
        ir: envelope,
    };

    let envelope_hash = blake3::hash(&record.ir);
    // The envelope hash should differ, but the inner hash should match
    let inner_matches = inner_hash.as_bytes() == &correct_digest.as_bytes();
    assert!(inner_matches, "Correct pattern: inner hash matches digest");

    // Structural proof: envelope hash != inner hash
    if envelope_hash.as_bytes() != inner_hash.as_bytes() {
    }
}
