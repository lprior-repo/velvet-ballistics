#![forbid(unsafe_code)]

use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, REQUIRED_GATE_COUNT,
    admit_artifact_run,
};
use vb_storage::EventSeq;
use vb_storage::admission::{AcceptedArtifact, VerificationProof};

struct ScenarioArtifactStore {
    artifact: AcceptedArtifact,
}

impl AcceptedArtifactStore for ScenarioArtifactStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
    }
}

fn scenario_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn verification_proof(
    digest: WorkflowDigest,
    idempotency_actions: Box<[ActionId]>,
) -> VerificationProof {
    VerificationProof {
        digest,
        gate_count: REQUIRED_GATE_COUNT,
        durable: true,
        bounded: true,
        taint_safe: true,
        retry_safe: true,
        idempotency_verified: true,
        replayable: true,
        idempotency_keyed: idempotency_actions.clone(),
        idempotency_attested: idempotency_actions,
        warnings: Vec::new(),
    }
}

fn accepted_artifact(
    digest: WorkflowDigest,
    required_capabilities: Box<[Capability]>,
    idempotency_actions: Box<[ActionId]>,
) -> AcceptedArtifact {
    AcceptedArtifact {
        digest,
        ir: Vec::new(),
        verification: verification_proof(digest, idempotency_actions),
        accepted_at_seq: EventSeq::new(1),
        required_capabilities,
    }
}

#[test]
fn test_admission_accepts_when_all_verification_gates_pass() {
    // Given: a v1 accepted artifact whose public admission proof has all 15 gates.
    let digest = scenario_digest(0x15);
    let store = ScenarioArtifactStore {
        artifact: accepted_artifact(digest, Box::new([]), Box::new([])),
    };

    // When: strict admission runs through the public runtime admission API.
    let observed = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(101),
        digest,
        CapabilitySet::empty(),
    );

    // Then: the run is admitted and the certificate records exact digest and policy evidence.
    assert!(matches!(
        observed,
        Ok(admission)
            if admission.artifact_digest() == digest
                && admission.run_id() == RunId::new(101)
                && admission.policy() == RuntimePolicy::Strict
    ));
}

#[test]
fn test_strict_verify_emits_certificate_when_workflow_is_safe() {
    // Given: a strict artifact with capability and idempotency evidence for a safe action.
    let digest = scenario_digest(0x42);
    let action = ActionId::new(7);
    let capability = Capability::new("network.github".into(), action);
    let grants = CapabilitySet::from_grants(Box::new([capability.clone()]));
    let store = ScenarioArtifactStore {
        artifact: accepted_artifact(digest, Box::new([capability]), Box::new([action])),
    };

    // When: strict admission validates the artifact through the public runtime surface.
    let observed = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(102),
        digest,
        grants.clone(),
    );

    // Then: the admission certificate carries granted capability and idempotency evidence.
    assert!(matches!(
        observed,
        Ok(admission)
            if admission.granted_capabilities() == &grants
                && admission.idempotency_attested() == [action]
                && admission.policy() == RuntimePolicy::Strict
    ));
}

#[test]
fn test_admission_rejects_when_capability_missing() {
    // Given: an accepted artifact that requires an explicit public capability grant.
    let digest = scenario_digest(0x77);
    let action = ActionId::new(9);
    let required = Capability::new("filesystem.read".into(), action);
    let granted = CapabilitySet::empty();
    let store = ScenarioArtifactStore {
        artifact: accepted_artifact(digest, Box::new([required.clone()]), Box::new([])),
    };

    // When: strict admission runs without the required grant.
    let observed = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(103),
        digest,
        granted.clone(),
    );

    // Then: admission fails closed with the exact missing-capability diagnostic.
    assert_eq!(
        observed,
        Err(AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        })
    );
}

#[test]
fn test_admission_rejects_when_ir_digest_mismatches_artifact() {
    // Given: a stored accepted artifact whose verification proof names a different digest.
    let requested = scenario_digest(0xAA);
    let found = scenario_digest(0xBB);
    let mut artifact = accepted_artifact(requested, Box::new([]), Box::new([]));
    artifact.verification.digest = found;
    let store = ScenarioArtifactStore { artifact };

    // When: strict admission validates the artifact envelope.
    let observed = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(104),
        requested,
        CapabilitySet::empty(),
    );

    // Then: admission rejects the digest mismatch before admitting the run.
    assert_eq!(
        observed,
        Err(AdmissionError::ArtifactDigestMismatch { requested, found })
    );
}
