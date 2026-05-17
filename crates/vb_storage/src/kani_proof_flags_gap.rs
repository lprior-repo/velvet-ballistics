//! Kani harness proving VerificationProof::new() always sets proof flags to true.
//!
//! Gap: `vb_storage::admission::VerificationProof::new()` unconditionally sets
//! `bounded=true`, `taint_safe=true`, `retry_safe=true`, `replayable=true`
//! regardless of actual workflow verification.
//!
//! This harness demonstrates that any CompiledWorkflow (valid or invalid)
//! produces a VerificationProof with all flags=true.

#![forbid(unsafe_code)]

use crate::admission::{VerificationProof, submit_artifact};
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, StepIdx, WorkflowDigest,
    value::ConstValue,
};

/// Creates a minimal workflow for testing.
fn minimal_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("test_admission"),
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

    let hash_bytes = postcard::to_allocvec(&parts).unwrap();
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());

    CompiledWorkflow::try_from_parts(parts).unwrap()
}

/// VB-STORAGE-GAP-001: VerificationProof::new always sets bounded=true
///
/// Proof: For any digest, gate_count, and durable flag, bounded is always true.
#[kani::proof]
fn verification_proof_new_bounded_always_true() {
    let digest = vb_core::WorkflowDigest::from_bytes(kani::any());
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = VerificationProof::new(digest, gate_count, durable);

    kani::assert(
        proof.bounded == true,
        "VerificationProof::new always sets bounded=true regardless of workflow validity",
    );
}

/// VB-STORAGE-GAP-002: VerificationProof::new always sets taint_safe=true
///
/// Proof: For any digest, gate_count, and durable flag, taint_safe is always true.
#[kani::proof]
fn verification_proof_new_taint_safe_always_true() {
    let digest = vb_core::WorkflowDigest::from_bytes(kani::any());
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = VerificationProof::new(digest, gate_count, durable);

    kani::assert(
        proof.taint_safe == true,
        "VerificationProof::new always sets taint_safe=true regardless of workflow validity",
    );
}

/// VB-STORAGE-GAP-003: VerificationProof::new always sets retry_safe=true
///
/// Proof: For any digest, gate_count, and durable flag, retry_safe is always true.
#[kani::proof]
fn verification_proof_new_retry_safe_always_true() {
    let digest = vb_core::WorkflowDigest::from_bytes(kani::any());
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = VerificationProof::new(digest, gate_count, durable);

    kani::assert(
        proof.retry_safe == true,
        "VerificationProof::new always sets retry_safe=true regardless of workflow validity",
    );
}

/// VB-STORAGE-GAP-004: VerificationProof::new always sets replayable=true
///
/// Proof: For any digest, gate_count, and durable flag, replayable is always true.
#[kani::proof]
fn verification_proof_new_replayable_always_true() {
    let digest = vb_core::WorkflowDigest::from_bytes(kani::any());
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = VerificationProof::new(digest, gate_count, durable);

    kani::assert(
        proof.replayable == true,
        "VerificationProof::new always sets replayable=true regardless of workflow validity",
    );
}

/// VB-STORAGE-GAP-005: All proof flags are always true simultaneously
///
/// Proof: VerificationProof::new sets ALL proof flags to true, not just one.
#[kani::proof]
fn verification_proof_new_all_flags_always_true() {
    let digest = vb_core::WorkflowDigest::from_bytes(kani::any());
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = VerificationProof::new(digest, gate_count, durable);

    kani::assert(
        proof.bounded && proof.taint_safe && proof.retry_safe && proof.replayable,
        "VerificationProof::new sets ALL proof flags to true simultaneously",
    );
}

/// VB-STORAGE-GAP-006: Gate count does not determine proof flag values
///
/// Proof: Even with gate_count=0 (Relaxed), proof flags are still true.
#[kani::proof]
fn verification_proof_new_gate_count_zero_still_flags_true() {
    let digest = vb_core::WorkflowDigest::from_bytes(kani::any());
    let durable = false;

    let proof = VerificationProof::new(digest, 0, durable);

    kani::assert(
        proof.bounded && proof.taint_safe && proof.retry_safe && proof.replayable,
        "VerificationProof::new with gate_count=0 still sets all flags=true",
    );
}
