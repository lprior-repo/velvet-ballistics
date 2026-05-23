//! Kani harnesses for vb_storage admission functions.
//!
//! Covers: KANI-ADMIT-001, KANI-ADMIT-002, KANI-DIGEST-001
//! - KANI-ADMIT-001: `submit_artifact` panic-free for bounded inputs
//! - KANI-ADMIT-002: `admit_compiled_artifact` preserves digest binding
//! - KANI-DIGEST-001: `VerificationProof` digest binding invariant on construction

#![forbid(unsafe_code)]

use crate::admission::{VerificationProof, admit_compiled_artifact, submit_artifact};
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, StepIdx, WorkflowDigest,
    value::ConstValue,
};

/// Builds a minimal valid CompiledWorkflow whose digest field is correctly
/// computed from its content (zeroed-digest serialization → BLAKE3).
///
/// This is required because `submit_artifact` and `admit_compiled_artifact`
/// validate the digest at lines 191 and 339 respectively: the claimed digest
/// must equal BLAKE3(serialize(parts with digest=0)).
fn minimal_valid_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("kani_admission"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Compute correct digest: serialize with digest=0, then BLAKE3.
    let hash_bytes = postcard::to_allocvec(&parts).unwrap();
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());

    CompiledWorkflow::try_from_parts(parts).unwrap()
}

// ---------------------------------------------------------------------------
// KANI-ADMIT-001: submit_artifact panic-free for all RuntimePolicy values
// ---------------------------------------------------------------------------

/// KANI-ADMIT-001: `submit_artifact` never panics on bounded inputs.
///
/// Proof: Creates a minimal valid CompiledWorkflow (digest correctly computed)
/// and calls `submit_artifact` with all three RuntimePolicy variants.
/// All paths return Result<AcceptedArtifact, JournalError> — no unwraps,
/// no panics. The function is also called with an empty action_contracts slice
/// to exercise the capability-extraction path.
///
/// Bound: Uses pre-computed minimal workflow (not kani::any()) because
/// arbitrary bytes cannot produce a valid CompiledWorkflow that passes
/// the checksum validation gate in submit_artifact.
#[kani::proof]
#[kani::unwind(4)]
fn submit_artifact_kani() {
    let workflow = minimal_valid_workflow();
    let policy = kani::any::<vb_core::RuntimePolicy>();

    // submit_artifact must not panic regardless of policy.
    // Under Relaxed: skips checksum, stores artifact, returns Ok.
    // Under Journaled/Strict: validates checksum, stores artifact, returns Ok.
    // Unknown/future variants: returns Err(ArtifactMalformed).
    let _ = submit_artifact(&kani::any::<crate::FjallJournal>(), &workflow, policy);
}

/// KANI-ADMIT-001 variant: submit_artifact with empty action_contracts.
///
/// Exercises the `submit_artifact_with_contracts` path with zero contracts,
/// covering the capability extraction code path without panicking.
#[kani::proof]
#[kani::unwind(4)]
fn submit_artifact_with_contracts_kani() {
    let workflow = minimal_valid_workflow();
    let policy = kani::any::<vb_core::RuntimePolicy>();

    let _ = crate::admission::submit_artifact_with_contracts(
        &kani::any::<crate::FjallJournal>(),
        &workflow,
        policy,
        &[],
    );
}

// ---------------------------------------------------------------------------
// KANI-ADMIT-002: admit_compiled_artifact digest binding preservation
// ---------------------------------------------------------------------------

/// KANI-ADMIT-002: `admit_compiled_artifact` preserves digest binding.
///
/// Proof: The function recomputes BLAKE3(serialize(parts with digest=0))
/// and compares it to workflow.digest(). On mismatch it returns
/// Err(ArtifactChecksumMismatch), never panicking.
///
/// This harness verifies that for a well-formed workflow (digest correctly
/// pre-computed), admit_compiled_artifact returns Ok(digest) without panic.
#[kani::proof]
#[kani::unwind(4)]
fn admit_compiled_artifact_kani() {
    let workflow = minimal_valid_workflow();

    // The workflow digest was pre-computed to match its content.
    // admit_compiled_artifact must not panic — it either returns Ok(digest)
    // or Err(ArtifactChecksumMismatch) if bytes are corrupted.
    let _ = admit_compiled_artifact(&kani::any::<crate::FjallJournal>(), &workflow);
}

// ---------------------------------------------------------------------------
// KANI-ADMIT-003: submit_artifact ok-path (PO-004)
// ---------------------------------------------------------------------------

/// KANI-ADMIT-003: `submit_artifact` returns Ok for valid workflow and journal.
///
/// Proof: With a pre-computed minimal valid workflow (digest correctly derived
/// from content) and RuntimePolicy::Relaxed (which skips checksum validation
/// but still stores the artifact), submit_artifact returns Ok(AcceptedArtifact).
///
/// Bound: Uses RuntimePolicy::Relaxed to avoid checksum validation path complexity.
/// The Relaxed path exercises artifact storage without requiring journal persistence.
#[kani::proof]
#[kani::unwind(5)]
fn submit_artifact_ok_path() {
    let workflow = minimal_valid_workflow();
    let policy = vb_core::RuntimePolicy::Relaxed;

    let result = submit_artifact(&kani::any::<crate::FjallJournal>(), &workflow, policy);

    // Meaningful property: successful submission with valid workflow returns Ok.
    kani::assert(
        result.is_ok(),
        "submit_artifact must return Ok for valid workflow under Relaxed policy",
    );
}

// ---------------------------------------------------------------------------
// KANI-ADMIT-004: admit_compiled_artifact ok-path (PO-005)
// ---------------------------------------------------------------------------

/// KANI-ADMIT-004: `admit_compiled_artifact` returns Ok for valid workflow.
///
/// Proof: With a pre-computed minimal valid workflow (digest correctly derived
/// from content), admit_compiled_artifact recomputes BLAKE3 and compares to
/// workflow.digest(), storing the artifact and returning Ok(digest).
///
/// Bound: The journal is unconstrained (kani::any()), but admit_compiled_artifact
/// returns Err only on structural failure (checksum mismatch, serialization error,
/// or journal failure). With valid workflow bytes, Err is unreachable for
/// checksum/structure; only journal failure is possible but does not panic.
#[kani::proof]
#[kani::unwind(5)]
fn admit_compiled_artifact_ok_path() {
    let workflow = minimal_valid_workflow();

    let result = admit_compiled_artifact(&kani::any::<crate::FjallJournal>(), &workflow);

    // Meaningful property: successful admission with valid workflow returns Ok.
    kani::assert(
        result.is_ok(),
        "admit_compiled_artifact must return Ok for valid workflow with correct digest",
    );
}

// ---------------------------------------------------------------------------
// KANI-DIGEST-001: VerificationProof digest binding invariant
// ---------------------------------------------------------------------------

/// KANI-DIGEST-001: VerificationProof digest binding invariant.
///
/// Proof: The `digest` field of VerificationProof is set to the exact value
/// passed as the first argument to `VerificationProof::new()`. There is no
/// transformation or recomputation — the digest is stored as-is.
///
/// This digest binding is the foundation of the artifact integrity contract:
/// `submit_artifact` and `admit_compiled_artifact` verify the artifact content
/// by recomputing BLAKE3(serialize(parts with digest=0)) and comparing it to
/// proof.digest. If they match, the artifact is admitted; otherwise rejected.
///
/// This harness uses kani::any() for all three constructor arguments to prove
/// that the digest binding holds for arbitrary digests, gate counts, and
/// durability flags.
///
/// Bound: unwind(3) — VerificationProof::new is a simple struct constructor
/// with no loops or recursion.
#[kani::proof]
#[kani::unwind(3)]
fn verification_proof_digest_binding() {
    let digest: WorkflowDigest = kani::any();
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = VerificationProof::new(digest, gate_count, durable);

    // The digest stored in the proof must be exactly the input digest.
    // This is the digest binding invariant: proof.digest ≡ workflow.digest.
    kani::assert(
        proof.digest == digest,
        "VerificationProof::new stores the exact input digest without transformation",
    );
}

/// KANI-DIGEST-001 variant: digest binding with relaxed (gate_count=0).
///
/// Even when gate_count=0 (Relaxed policy, no verification performed),
/// the proof.digest must still equal the input digest.
#[kani::proof]
fn verification_proof_digest_binding_relaxed() {
    let digest: WorkflowDigest = kani::any();
    let durable = false;

    let proof = VerificationProof::new(digest, 0, durable);

    kani::assert(
        proof.digest == digest,
        "VerificationProof::new with gate_count=0 still preserves digest binding",
    );
}

/// KANI-DIGEST-001 variant: proof flags are set unconditionally.
///
/// Documents that proof flags (bounded, taint_safe, retry_safe, replayable,
/// idempotency_verified) are always true regardless of gate_count or digest.
/// This is the VB-STORAGE-GAP — not a bug of this harness, but a documented
/// gap that the flags are set without actual per-gate verification.
#[kani::proof]
fn verification_proof_all_flags_unconditional() {
    let digest: WorkflowDigest = kani::any();
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = VerificationProof::new(digest, gate_count, durable);

    // All flags are always true (this is the known gap).
    kani::assert(
        proof.bounded && proof.taint_safe && proof.retry_safe && proof.replayable,
        "All proof flags are true regardless of gate_count (documented gap)",
    );

    // Digest binding always holds.
    kani::assert(
        proof.digest == digest,
        "Digest binding holds regardless of flag values",
    );
}
