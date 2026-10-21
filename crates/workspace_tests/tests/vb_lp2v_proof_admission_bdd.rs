#![forbid(unsafe_code)]

use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_core::{
    ActionId, Capability, CapabilitySet, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest,
};
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

fn accepted_artifact(_requested_source_digest: WorkflowDigest) -> Result<AcceptedArtifact, String> {
    let mut parts = WorkflowParts {
        name: Box::from("lp2v_proof_admission"),
        digest: digest(0),
        semantic_source_identity: None,
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::new(0),
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
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    };
    parts.digest = digest(0);
    parts.digest = WorkflowDigest::from_bytes(
        blake3::hash(&postcard::to_allocvec(&parts).map_err(|error| error.to_string())?).into(),
    );
    let workflow =
        vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let contract = ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("lp2v_admission_action").map_err(|error| error.to_string())?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([required_capability()]),
    };
    prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[contract])
        .map_err(|error| error.to_string())
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
