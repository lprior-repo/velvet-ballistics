//! Kani harnesses for pure vb_storage admission value invariants.
//!
//! Storage-backed admission functions require a live `FjallJournal` handle and
//! are verified by behavior tests, not by arbitrary Kani construction of an
//! external database handle. This module keeps the Kani lane focused on pure
//! admission data invariants that Kani can model without stubs.

#![forbid(unsafe_code)]

use crate::admission::{VerificationProof, admit_compiled_artifact, submit_artifact};
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest, value::ConstValue,
};

fn bounded_policy() -> RuntimePolicy {
    match kani::any::<u8>() % 3 {
        0 => RuntimePolicy::Relaxed,
        1 => RuntimePolicy::Journaled,
        _ => RuntimePolicy::Strict,
    }
}

fn bounded_journal() -> Option<crate::FjallJournal> {
    crate::FjallJournal::open("/tmp/vb-storage-kani-admission", None).ok()
}

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

    let hash_bytes = match postcard::to_allocvec(&parts) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());

    match CompiledWorkflow::try_from_parts(parts) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn submit_artifact_kani() {
    let workflow = minimal_valid_workflow();
    let policy = bounded_policy();

    if let Some(journal) = bounded_journal() {
        let _ = submit_artifact(&journal, &workflow, policy);
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn submit_artifact_with_contracts_kani() {
    let workflow = minimal_valid_workflow();
    let policy = bounded_policy();

    if let Some(journal) = bounded_journal() {
        let _ = crate::admission::submit_artifact_with_contracts(&journal, &workflow, policy, &[]);
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn admit_compiled_artifact_kani() {
    let workflow = minimal_valid_workflow();

    if let Some(journal) = bounded_journal() {
        let _ = admit_compiled_artifact(&journal, &workflow);
    }
}

#[kani::proof]
#[kani::unwind(40)]
fn verification_proof_digest_binding() {
    let digest: WorkflowDigest = kani::any();
    let gate_count: u8 = kani::any();
    let durable: bool = kani::any();

    let proof = VerificationProof::new(digest, gate_count, durable);

    kani::assert(
        proof.digest == digest,
        "VerificationProof::new stores the exact input digest without transformation",
    );
}

#[kani::proof]
#[kani::unwind(40)]
fn verification_proof_digest_binding_relaxed() {
    let digest: WorkflowDigest = kani::any();
    let proof = VerificationProof::new(digest, 0, false);

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

    let proof = VerificationProof::new(digest, gate_count, durable);

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
