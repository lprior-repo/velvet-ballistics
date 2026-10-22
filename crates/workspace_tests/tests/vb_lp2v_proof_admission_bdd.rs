#![forbid(unsafe_code)]

use std::sync::Arc;

use vb_core::{ActionId, Capability, CapabilitySet, RunId, RuntimePolicy, WorkflowDigest};
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, StorageArtifactStore,
    admit_artifact_run,
};
use vb_storage::admission::{AcceptedArtifact, prepare_compiled_artifact};
use vb_storage::{CompiledIrRecord, FjallJournal, put_compiled_ir};

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn required_capability() -> Capability {
    Capability::new("net.fetch".into(), ActionId::new(0))
}

fn granted_capabilities(required: Capability) -> CapabilitySet {
    CapabilitySet::from_grants(Box::new([required]))
}

fn compile_fixture(_source_digest: WorkflowDigest) -> vb_core::CompiledWorkflow {
    let source = "version: velvet-ballistics/v1\nname: lp2v_fixture\nwhen: { manual: {} }\nsteps:\n  - id: make\n    set: { output: answer, value: \"42\" }\n  - id: done\n    finish: { result: answer }\n";
    vb_compile::compile_workflow(source.as_bytes()).expect("compile fixture")
}

fn accepted_artifact(source_digest: WorkflowDigest) -> Result<AcceptedArtifact, String> {
    let workflow = compile_fixture(source_digest);
    let artifact = prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])
        .map_err(|error| error.to_string())?;
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
    let artifact = accepted_artifact(digest(0xB1))?;
    let requested = artifact.digest;
    let mut tampered = artifact.clone();
    tampered.verification.digest = found;
    let store = FixedAcceptedStore { artifact: tampered };

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
    let artifact = accepted_artifact(digest(0xC1))?;
    let requested = artifact.source_digest;
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
