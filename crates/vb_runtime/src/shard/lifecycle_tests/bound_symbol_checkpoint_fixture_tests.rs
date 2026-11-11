use crate::RuntimeError;
use crate::journal::VolatileRuntimeJournal;
use crate::run_checkpoint::symbol_prefix_fixture::{
    TestResult, fixture_contract, populated_store, prefix_workflow, retry_checkpoint,
};
use crate::run_checkpoint::{RunCheckpointV1, RunCheckpointV1Error};
use crate::shard::types::RecoverRunCommand;
use crate::shard::{Shard, ShardConfig};
use std::sync::Arc;
use vb_core::{CompiledWorkflow, RunId, WorkflowDigest};
use vb_storage::admission::AcceptedArtifact;

struct PrefixArtifactStore(AcceptedArtifact);

impl crate::admission::AcceptedArtifactStore for PrefixArtifactStore {
    fn load_accepted_artifact(
        &self,
        digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, crate::admission::ArtifactEnvelopeError> {
        if digest != self.0.digest {
            return Err(crate::admission::ArtifactEnvelopeError::ArtifactNotFound { digest });
        }
        Ok(self.0.clone())
    }
}

fn prefix_verification(digest: WorkflowDigest) -> vb_storage::admission::VerificationProof {
    vb_storage::admission::VerificationProof {
        evidence: vb_core::artifact::ArtifactEvidence::Unverified,
        digest,
        gate_count: 8,
        durable: true,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: Vec::new(),
    }
}

fn prefix_artifact(workflow: &CompiledWorkflow) -> TestResult<AcceptedArtifact> {
    let digest = WorkflowDigest::from_bytes([43; 32]);
    Ok(AcceptedArtifact {
        digest,
        source_digest: workflow.digest(),
        policy_digest: vb_storage::admission::compute_policy_digest(workflow)?,
        ir: vb_storage::encode_compiled_ir(workflow)?.into_vec(),
        verification: prefix_verification(digest),
        accepted_at_seq: vb_storage::EventSeq::new(0),
        required_capabilities: Box::new([]),
        action_contracts: Box::from([fixture_contract()?]),
        artifact_version: vb_storage::compat::SchemaVersion::new(2, 0, 0),
    })
}

struct PrefixActorFixture {
    shard: Shard,
    journal: Arc<VolatileRuntimeJournal>,
    workflow: CompiledWorkflow,
    checkpoint: RunCheckpointV1,
}

fn prefix_actor_fixture() -> TestResult<PrefixActorFixture> {
    let workflow = prefix_workflow(vec!["key".into()])?;
    let checkpoint = retry_checkpoint(&workflow)?;
    let store = Arc::new(PrefixArtifactStore(prefix_artifact(&workflow)?));
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let config = ShardConfig {
        policy: vb_core::RuntimePolicy::Relaxed,
        ..ShardConfig::default()
    };
    let shard = Shard::new_with_journal_and_artifact_store(config, journal.clone(), store);
    Ok(PrefixActorFixture {
        shard,
        journal,
        workflow,
        checkpoint,
    })
}

fn prefix_raw_command(fixture: &PrefixActorFixture) -> TestResult<RecoverRunCommand> {
    Ok(RecoverRunCommand::new(
        RunId::new(71),
        vb_core::RunFrame::from_checkpoint_v1(fixture.checkpoint.frame.clone())
            .map_err(|error| format!("retry frame fixture failed: {error:?}"))?,
        fixture.checkpoint.artifact_digest,
        fixture.workflow.digest(),
        vb_storage::EventSeq::new(10),
        crate::primitives::collect::CollectStates::new(),
        crate::recovery::RecoveredRunBoundary::none(),
    ))
}

fn prefix_retry_command(fixture: &PrefixActorFixture) -> TestResult<RecoverRunCommand> {
    Ok(prefix_raw_command(fixture)?.with_retry_checkpoint(fixture.checkpoint.clone()))
}

fn corrupt_prefix(fixture: &mut PrefixActorFixture) -> TestResult {
    *fixture
        .checkpoint
        .value_store
        .symbols
        .get_mut(0)
        .ok_or("missing prefix symbol")? = "wrong".into();
    assert_eq!(
        fixture.checkpoint.validate_structure_against(
            &fixture.workflow,
            fixture.checkpoint.artifact_digest,
            &fixture.checkpoint.action_contracts
        ),
        Ok(())
    );
    Ok(())
}

fn corrupt_historical_pin(fixture: &mut PrefixActorFixture) -> TestResult {
    let admission = fixture
        .checkpoint
        .admission
        .take()
        .ok_or("missing fixture admission")?;
    fixture.checkpoint.admission = Some(admission.with_policy_pin(
        vb_storage::RunPolicyPin::Pinned(WorkflowDigest::from_bytes([99; 32])),
    ));
    Ok(())
}

fn prefix_retry_error(message: &str) -> RuntimeError {
    RuntimeError::Recovery {
        error: format!("retry checkpoint validation failed: {message}"),
    }
}
