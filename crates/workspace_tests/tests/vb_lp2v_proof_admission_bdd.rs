#![forbid(unsafe_code)]

use std::sync::Arc;

use vb_core::{ActionId, Capability, CapabilitySet, RunId, RuntimePolicy, WorkflowDigest};
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, REQUIRED_GATE_COUNT,
    StorageArtifactStore, admit_artifact_run,
};
use vb_storage::admission::{AcceptedArtifact, VerificationProof};
use vb_storage::{CompiledIrRecord, EventSeq, FjallJournal};

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

struct ReturningAcceptedArtifactStore {
    artifact: AcceptedArtifact,
}

impl AcceptedArtifactStore for ReturningAcceptedArtifactStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
    }
}

fn required_capability() -> Capability {
    Capability::new("net.fetch".into(), ActionId::new(7))
}

fn granted_capabilities(required: Capability) -> CapabilitySet {
    CapabilitySet::from_grants(Box::new([required]))
}

fn accepted_artifact(proof_digest: WorkflowDigest) -> Result<AcceptedArtifact, String> {
    let workflow = compile_storage_workflow()?;
    let mut parts = workflow.to_parts();
    parts.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let ir = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    let artifact_digest = workflow.digest();
    let policy_digest = vb_storage::admission::compute_policy_digest(&workflow)
        .map_err(|error| error.to_string())?;
    Ok(AcceptedArtifact {
        digest: artifact_digest,
        source_digest: artifact_digest,
        policy_digest,
        ir,
        verification: VerificationProof {
            digest: proof_digest,
            gate_count: REQUIRED_GATE_COUNT,
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
        accepted_at_seq: EventSeq::new(42),
        required_capabilities: Box::new([required_capability()]),
    })
}

fn compile_storage_workflow() -> Result<vb_core::CompiledWorkflow, String> {
    let yaml = br#"version: velvet-ballistics/v1
name: proof_admission_bdd
when:
  manual: {}
steps:
  - id: make
    set:
      output: answer
      value: "42"
  - id: done
    finish:
      result: answer
"#;
    let workflow = vb_compile::compile_workflow(yaml).map_err(|errors| errors.to_string())?;
    let mut parts = workflow.to_parts();
    parts.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let ir = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn persist_artifact(journal: &FjallJournal, artifact: &AcceptedArtifact) -> Result<(), String> {
    persist_artifact_as(journal, artifact.digest, artifact)
}

fn persist_artifact_as(
    journal: &FjallJournal,
    record_digest: WorkflowDigest,
    artifact: &AcceptedArtifact,
) -> Result<(), String> {
    let ir = postcard::to_allocvec(artifact).map_err(|error| error.to_string())?;
    journal
        .put_compiled_ir(&CompiledIrRecord {
            digest: record_digest,
            ir,
        })
        .map_err(|error| error.to_string())
}

#[test]
fn given_matching_proof_digest_when_strict_admission_runs_then_artifact_is_admitted()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?;
    let mut artifact = accepted_artifact(digest(0xA1))?;
    artifact.verification.digest = artifact.digest;
    let requested = artifact.digest;
    persist_artifact(&journal, &artifact)?;
    let store = StorageArtifactStore::new(Arc::new(journal));
    let run = RunId::new(9001);
    let caps = granted_capabilities(required_capability());

    let admission = admit_artifact_run(&store, RuntimePolicy::Strict, run, requested, caps.clone())
        .map_err(|error| error.to_string())?;

    assert_eq!(admission.artifact_digest(), requested);
    assert_eq!(admission.policy(), RuntimePolicy::Strict);
    assert_eq!(admission.run_id(), run);
    assert_eq!(admission.granted_capabilities(), &caps);
    Ok(())
}

#[test]
fn given_mismatched_proof_digest_when_strict_admission_runs_then_digest_mismatch_denies()
-> Result<(), String> {
    let found = digest(0xB2);
    let artifact = accepted_artifact(found)?;
    let requested = artifact.digest;
    let store = ReturningAcceptedArtifactStore { artifact };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(9002),
        requested,
        granted_capabilities(required_capability()),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactDigestMismatch { requested, found })
    );
    Ok(())
}

#[test]
fn given_storage_record_with_mismatched_artifact_digest_when_strict_admission_runs_then_digest_mismatch_denies()
-> Result<(), String> {
    let mut artifact = accepted_artifact(digest(0xC2))?;
    artifact.verification.digest = artifact.digest;
    let requested = digest(0xC1);
    let found = artifact.digest;
    let store = ReturningAcceptedArtifactStore { artifact };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(9003),
        requested,
        granted_capabilities(required_capability()),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactDigestMismatch { requested, found })
    );
    Ok(())
}
