#![forbid(unsafe_code)]
//! Metadata hashing for accepted artifacts.

use super::types::AcceptedArtifact;

/// Computes a BLAKE3 hash of the artifact metadata fields that must remain
/// immutable after admission.
///
/// This includes: `source_digest`, `policy_digest`, the inner `ir` bytes,
/// `verification` fields (excluding the nested `digest` which equals the outer
/// digest), `accepted_at_seq`, and `required_capabilities`.
///
/// The `digest` field itself is excluded because it is the primary binding
/// already verified by `validate_accepted_artifact_digest`.
pub(crate) fn compute_artifact_metadata_hash(artifact: &AcceptedArtifact) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&artifact.source_digest.as_bytes());
    hasher.update(&artifact.policy_digest.as_bytes());
    hasher.update(&artifact.ir);
    // Hash verification fields (excluding artifact.verification.digest which
    // equals artifact.digest, already verified separately; durable and gate_count
    // which are runtime policy decisions, not intrinsic artifact metadata)
    // NOTE: durable is NOT included because it reflects RuntimePolicy (Strict vs Journaled)
    // at admission time, not an immutable artifact property
    // NOTE: gate_count is NOT included because Relaxed=0 vs Journaled/Strict=15,
    // so the same artifact legitimately has different gate_count under different policies
    hasher.update(&[u8::from(artifact.verification.bounded_claimed)]);
    hasher.update(&[u8::from(artifact.verification.taint_safe_claimed)]);
    hasher.update(&[u8::from(artifact.verification.retry_safe_claimed)]);
    hasher.update(&[u8::from(artifact.verification.idempotency_verified_claimed)]);
    hasher.update(&[u8::from(artifact.verification.replayable_claimed)]);
    // Hash idempotency data
    for id in artifact.verification.idempotency_keyed.as_ref() {
        hasher.update(&id.get().to_le_bytes());
    }
    for id in artifact.verification.idempotency_attested.as_ref() {
        hasher.update(&id.get().to_le_bytes());
    }
    // Hash warnings
    for w in &artifact.verification.warnings {
        hasher.update(&w.code.to_le_bytes());
        hasher.update(w.message.as_bytes());
        hasher.update(&[w.gate]);
    }
    hasher.update(&artifact.accepted_at_seq.get().to_le_bytes());
    for cap in artifact.required_capabilities.as_ref() {
        hasher.update(cap.name().as_bytes());
        hasher.update(&cap.action_id().get().to_le_bytes());
    }
    *hasher.finalize().as_bytes()
}
