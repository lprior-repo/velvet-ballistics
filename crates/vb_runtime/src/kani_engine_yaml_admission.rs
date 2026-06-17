//! Kani harnesses for PO-012 engine YAML artifact admission.
//!
//! These are `cfg(kani)` verification artifacts only. They model the runtime
//! admission boundary using deterministic stores and the public admission API.

#![forbid(unsafe_code)]

use crate::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, REQUIRED_GATE_COUNT,
    admit_artifact_run,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;

struct ErrorStore {
    error: ArtifactEnvelopeError,
}

impl AcceptedArtifactStore for ErrorStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Err(self.error.clone())
    }
}

struct FixedStore {
    artifact: vb_storage::admission::AcceptedArtifact,
}

impl AcceptedArtifactStore for FixedStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
    }
}

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn proof(
    digest: WorkflowDigest,
    gate_count: u8,
    bounded: bool,
) -> vb_storage::admission::VerificationProof {
    vb_storage::admission::VerificationProof {
        digest,
        gate_count,
        durable: true,
        bounded,
        taint_safe: true,
        retry_safe: true,
        idempotency_verified: true,
        replayable: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: Vec::new(),
    }
}

fn artifact(
    digest: WorkflowDigest,
    gate_count: u8,
    bounded: bool,
    required_capabilities: Box<[Capability]>,
) -> vb_storage::admission::AcceptedArtifact {
    vb_storage::admission::AcceptedArtifact {
        digest,
        ir: Vec::new(),
        verification: proof(digest, gate_count, bounded),
        accepted_at_seq: vb_storage::EventSeq::new(0),
        required_capabilities,
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn engine_yaml_admission_rejects_raw_ir() {
    let artifact_digest = digest(0x11);
    let store = ErrorStore {
        error: ArtifactEnvelopeError::PostcardDecodeFailed,
    };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        artifact_digest,
        CapabilitySet::empty(),
    );
    kani::assert(matches!(result, Err(AdmissionError::ArtifactEnvelopeDecodeFailed), "assertion failed"),
        "raw IR bytes must not admit as an accepted artifact",
    );
}

#[kani::proof]
#[kani::unwind(64)]
fn engine_yaml_admission_rejects_dummy_proof() {
    let artifact_digest = digest(0x22);
    let store = FixedStore {
        artifact: artifact(artifact_digest, REQUIRED_GATE_COUNT, false, Box::new([])),
    };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(2),
        artifact_digest,
        CapabilitySet::empty(),
    );
    kani::assert(matches!(
            result,
            Err(AdmissionError::ArtifactInvalidProofFlag { flag: "bounded" }), "assertion failed"),
        "dummy proof with missing bounded flag must reject",
    );
}

#[kani::proof]
#[kani::unwind(64)]
fn engine_yaml_admission_rejects_digest_mismatch() {
    let requested_digest = digest(0x33);
    let store = ErrorStore {
        error: ArtifactEnvelopeError::ArtifactNotFound {
            digest: requested_digest,
        },
    };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(3),
        requested_digest,
        CapabilitySet::empty(),
    );
    kani::assert(matches!(result, Err(AdmissionError::ArtifactNotFound { digest }) if digest == requested_digest, "assertion failed"),
        "digest mismatch or missing accepted artifact must reject",
    );
}

#[kani::proof]
#[kani::unwind(64)]
fn engine_yaml_admission_requires_capability_gate() {
    let artifact_digest = digest(0x44);
    let action_id = ActionId::new(7);
    let required = Capability::new("network.egress".into(), action_id);
    let store = FixedStore {
        artifact: artifact(
            artifact_digest,
            REQUIRED_GATE_COUNT,
            true,
            Box::new([required]),
        ),
    };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(4),
        artifact_digest,
        CapabilitySet::empty(),
    );
    kani::assert(matches!(result, Err(AdmissionError::CapabilityDenied { .. }), "assertion failed"),
        "accepted artifact admission must require matching capability grants",
    );
}
