#![forbid(unsafe_code)]

use std::sync::Arc;

use vb_core::{ActionId, Capability, CapabilitySet, RunId, RuntimePolicy, WorkflowDigest};
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, REQUIRED_GATE_COUNT,
    StorageArtifactStore, admit_artifact_run,
};
use vb_storage::admission::{AcceptedArtifact, VerificationProof, accepted_artifact_digest};
use vb_storage::{CompiledIrRecord, EventSeq, FjallJournal, put_compiled_ir};

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn required_capability() -> Capability {
    Capability::new("net.fetch".into(), ActionId::new(7))
}

fn granted_capabilities(required: Capability) -> CapabilitySet {
    CapabilitySet::from_grants(Box::new([required]))
}

fn accepted_artifact(source_digest: WorkflowDigest) -> Result<AcceptedArtifact, String> {
    let zero = digest(0);
    let mut artifact = AcceptedArtifact {
        digest: zero,
        source_digest,
        policy_digest: source_digest,
        ir: Vec::new(),
        verification: VerificationProof {
            digest: zero,
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
        action_contracts: Box::new([]),
    };
    let artifact_digest = accepted_artifact_digest(&artifact).map_err(|error| error.to_string())?;
    artifact.digest = artifact_digest;
    artifact.verification.digest = artifact_digest;
    Ok(artifact)
}

struct FixedAcceptedStore {
    artifact: AcceptedArtifact,
}

impl AcceptedArtifactStore for FixedAcceptedStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
    }
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
    put_compiled_ir(
        journal,
        &CompiledIrRecord {
            digest: record_digest,
            ir,
        },
    )
    .map_err(|error| error.to_string())
}

#[test]
fn given_matching_proof_digest_when_strict_admission_runs_then_artifact_is_admitted()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?;
    let artifact = accepted_artifact(digest(0xA1))?;
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
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let _journal = FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?;
    let found = digest(0xB2);
    let mut artifact = accepted_artifact(digest(0xB1))?;
    let requested = artifact.digest;
    artifact.verification.digest = found;
    let store = FixedAcceptedStore { artifact };

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
fn given_storage_record_requested_by_source_digest_when_strict_admission_runs_then_artifact_is_admitted()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?;
    let requested = digest(0xC1);
    let artifact = accepted_artifact(requested)?;
    let found = artifact.digest;
    persist_artifact(&journal, &artifact)?;
    let store = StorageArtifactStore::new(Arc::new(journal));

    let admission = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(9003),
        requested,
        granted_capabilities(required_capability()),
    )
    .map_err(|error| error.to_string())?;

    assert_eq!(admission.artifact_digest(), found);
    Ok(())
}
