#![forbid(unsafe_code)]

use std::sync::Arc;

use vb_core::{ActionId, Capability, CapabilitySet, RunId, RuntimePolicy, WorkflowDigest};
use vb_runtime::admission::{
    AdmissionError, REQUIRED_GATE_COUNT, StorageArtifactStore, admit_artifact_run,
};
use vb_storage::admission::{AcceptedArtifact, VerificationProof};
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

fn accepted_artifact(
    artifact_digest: WorkflowDigest,
    proof_digest: WorkflowDigest,
) -> AcceptedArtifact {
    AcceptedArtifact {
        digest: artifact_digest,
        ir: Vec::new(),
        verification: VerificationProof {
            digest: proof_digest,
            gate_count: REQUIRED_GATE_COUNT,
            durable: true,
            bounded: true,
            taint_safe: true,
            retry_safe: true,
            idempotency_verified: true,
            replayable: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: EventSeq::new(42),
        required_capabilities: Box::new([required_capability()]),
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
    let requested = digest(0xA1);
    let artifact = accepted_artifact(requested, requested);
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
    let journal = FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?;
    let requested = digest(0xB1);
    let found = digest(0xB2);
    let artifact = accepted_artifact(requested, found);
    persist_artifact(&journal, &artifact)?;
    let store = StorageArtifactStore::new(Arc::new(journal));

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
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?;
    let requested = digest(0xC1);
    let found = digest(0xC2);
    let artifact = accepted_artifact(found, found);
    persist_artifact_as(&journal, requested, &artifact)?;
    let store = StorageArtifactStore::new(Arc::new(journal));

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
