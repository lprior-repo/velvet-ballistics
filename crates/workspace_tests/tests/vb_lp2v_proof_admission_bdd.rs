#![forbid(unsafe_code)]

use std::sync::Arc;

use vb_core::capability::CapabilitySet;
use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::admission::{
    AdmissionError, REQUIRED_GATE_COUNT, StorageArtifactStore, admit_artifact_run,
};
use vb_storage::admission::{AcceptedArtifact, VerificationProof};
use vb_storage::{CompiledIrRecord, EventSeq, FjallJournal};

fn minimal_workflow() -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("vb_lp2v_proof_admission_bdd"),
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
        step_names: Box::new([Box::<str>::from("set_answer"), Box::<str>::from("finish")]),
    };
    let hash_bytes = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash_bytes).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn accepted_artifact(
    workflow: &CompiledWorkflow,
    artifact_digest: WorkflowDigest,
    proof_digest: WorkflowDigest,
) -> Result<AcceptedArtifact, String> {
    let ir = postcard::to_allocvec(&workflow.to_parts()).map_err(|error| error.to_string())?;
    Ok(AcceptedArtifact {
        digest: artifact_digest,
        ir,
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
        accepted_at_seq: EventSeq::new(0),
        required_capabilities: Box::new([]),
    })
}

fn persist_artifact(journal: &FjallJournal, artifact: &AcceptedArtifact) -> Result<(), String> {
    let payload = postcard::to_allocvec(artifact).map_err(|error| error.to_string())?;
    let record = CompiledIrRecord {
        digest: artifact.digest,
        ir: payload,
    };
    journal
        .put_compiled_ir(&record)
        .map_err(|error| error.to_string())
}

#[test]
fn given_storage_artifact_with_matching_proof_digest_when_strict_admission_runs_then_admitted()
-> Result<(), String> {
    // Given: real local Fjall storage contains a postcard AcceptedArtifact whose artifact and proof digests match.
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal =
        Arc::new(FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?);
    let workflow = minimal_workflow()?;
    let artifact_digest = workflow.digest();
    let artifact = accepted_artifact(&workflow, artifact_digest, artifact_digest)?;
    persist_artifact(&journal, &artifact)?;
    let store = StorageArtifactStore::new(Arc::clone(&journal));
    let run_id = RunId::new(7_001);

    // When: strict runtime admission loads the accepted artifact from storage.
    let admitted = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        run_id,
        artifact_digest,
        CapabilitySet::empty(),
    )
    .map_err(|error| error.to_string())?;

    // Then: the run is admitted with the exact requested digest and strict policy.
    assert_eq!(admitted.artifact_digest(), artifact_digest);
    assert_eq!(admitted.policy(), RuntimePolicy::Strict);
    assert_eq!(admitted.run_id(), run_id);
    assert_eq!(admitted.granted_capabilities(), &CapabilitySet::empty());
    Ok(())
}

#[test]
fn given_storage_artifact_with_mismatched_proof_digest_when_strict_admission_runs_then_digest_mismatch_denies()
-> Result<(), String> {
    // Given: real local Fjall storage contains a postcard AcceptedArtifact whose proof digest differs from the requested artifact digest.
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal =
        Arc::new(FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?);
    let workflow = minimal_workflow()?;
    let requested_artifact_digest = workflow.digest();
    let found_proof_digest = WorkflowDigest::from_bytes([0xA5; 32]);
    let artifact = accepted_artifact(&workflow, requested_artifact_digest, found_proof_digest)?;
    persist_artifact(&journal, &artifact)?;
    let store = StorageArtifactStore::new(Arc::clone(&journal));

    // When: strict runtime admission loads the accepted artifact from storage.
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(7_002),
        requested_artifact_digest,
        CapabilitySet::empty(),
    );

    // Then: admission is denied with the exact requested artifact digest and found proof digest.
    match result {
        Err(AdmissionError::ArtifactDigestMismatch { requested, found }) => {
            assert_eq!(requested, requested_artifact_digest);
            assert_eq!(found, found_proof_digest);
        }
        other => {
            return Err(format!(
                "expected ArtifactDigestMismatch {{ requested: {requested_artifact_digest:?}, found: {found_proof_digest:?} }}, got {other:?}"
            ));
        }
    }
    Ok(())
}
