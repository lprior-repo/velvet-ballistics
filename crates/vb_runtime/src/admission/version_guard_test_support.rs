use super::*;
use vb_core::workflow::{CompiledWorkflow, WorkflowIrV2, WorkflowParts};
use vb_storage::AcceptedArtifact;

pub(super) fn rebind(mut artifact: AcceptedArtifact) -> FixtureResult<AcceptedArtifact> {
    let digest = vb_storage::admission::accepted_artifact_digest(&artifact)?;
    artifact.digest = digest;
    artifact.verification.digest = digest;
    Ok(artifact)
}

pub(super) fn legacy(policy: RuntimePolicy) -> FixtureResult<AcceptedArtifact> {
    let original = accepted_artifact_with_caps(Box::new([]))?;
    let parts: WorkflowParts = postcard::from_bytes(&original.ir)?;
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    Ok(vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        policy,
        &[],
    )?)
}

fn wire(ir: &WorkflowIrV2) -> FixtureResult<Vec<u8>> {
    let mut bytes = vec![0x01, 0xff, 0x56, 0x42];
    bytes.extend_from_slice(&postcard::to_allocvec(ir)?);
    Ok(bytes)
}

fn bound_workflow(payloads: &[&str]) -> FixtureResult<CompiledWorkflow> {
    let original = accepted_artifact_with_caps(Box::new([]))?;
    let mut parts: WorkflowParts = postcard::from_bytes(&original.ir)?;
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.symbols_count = u32::try_from(payloads.len())?;
    let mut ir = WorkflowIrV2 {
        wire_version: 2,
        parts,
        symbol_payloads: payloads
            .iter()
            .map(|value| Box::<str>::from(*value))
            .collect(),
    };
    ir.parts.digest = WorkflowDigest::from_bytes(blake3::hash(&wire(&ir)?).into());
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        ir.parts,
        ir.symbol_payloads,
    )?)
}

pub(super) fn bound(
    policy: RuntimePolicy,
    held: SchemaVersion,
    payloads: &[&str],
) -> FixtureResult<AcceptedArtifact> {
    let workflow = bound_workflow(payloads)?;
    let mut artifact = vb_storage::admission::prepare_compiled_artifact(&workflow, policy, &[])?;
    assert_eq!(artifact.artifact_version, SchemaVersion::new(2, 0, 0));
    artifact.artifact_version = held;
    rebind(artifact)
}

pub(super) fn future(policy: RuntimePolicy) -> FixtureResult<AcceptedArtifact> {
    let mut artifact = legacy(policy)?;
    artifact.artifact_version = SchemaVersion::new(3, 0, 0);
    rebind(artifact)
}

pub(super) fn future_error() -> AdmissionError {
    AdmissionError::ArtifactVersionIncompatible {
        held: SchemaVersion::new(3, 0, 0),
        required: SchemaVersion::new(2, 0, 0),
        code: 300,
    }
}

pub(super) fn simple(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
) -> Result<RunAdmission, AdmissionError> {
    admit_run(store, policy, digest, RunId::new(1), CapabilitySet::empty())
}

pub(super) fn full(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
) -> Result<RunAdmission, AdmissionError> {
    admit_artifact_run(store, policy, RunId::new(1), digest, CapabilitySet::empty())
}

pub(super) fn rejected(
    artifact: AcceptedArtifact,
    policy: RuntimePolicy,
    expected: AdmissionError,
) {
    let store = FixedAcceptedStore {
        artifact: artifact.clone(),
    };
    assert_eq!(
        simple(&store, policy, artifact.digest),
        Err(expected.clone())
    );
    assert_eq!(full(&store, policy, artifact.digest), Err(expected));
    assert_eq!(store.artifact, artifact);
}

pub(super) fn exact_decision(
    held: SchemaVersion,
    class: CompatibilityClass,
    code: u16,
) -> FixtureResult<CompatDecision> {
    let artifact = bound(RuntimePolicy::Strict, held, &["key"])?;
    let decision = check_artifact_ir_version(&artifact)?;
    assert_eq!(
        decision.boundary,
        vb_storage::compat::CompatBoundary::IrVersion
    );
    assert_eq!(decision.class, class);
    assert_eq!(decision.decision_code.code(), code);
    assert_eq!(
        decision.old_schema_digest,
        vb_storage::compat::schema_version_digest(held)
    );
    assert_eq!(
        decision.new_schema_digest,
        vb_storage::compat::schema_version_digest(SchemaVersion::new(2, 0, 0))
    );
    Ok(decision)
}

fn expected_full(
    artifact: &AcceptedArtifact,
    policy: RuntimePolicy,
    decision: CompatDecision,
) -> RunAdmission {
    RunAdmission::with_idempotency_evidence(
        artifact.digest,
        RunId::new(1),
        CapabilitySet::empty(),
        policy,
        artifact.verification.idempotency_attested.clone(),
    )
    .with_policy_pin(RunPolicyPin::Pinned(artifact.policy_digest))
    .with_ir_compat_decision(decision)
}

pub(super) fn admitted(
    store: &dyn AcceptedArtifactStore,
    artifact: &AcceptedArtifact,
    policy: RuntimePolicy,
    decision: CompatDecision,
) {
    let expected_simple = RunAdmission::new(
        artifact.digest,
        RunId::new(1),
        CapabilitySet::empty(),
        policy,
    )
    .with_ir_compat_decision(decision);
    assert_eq!(simple(store, policy, artifact.digest), Ok(expected_simple));
    assert_eq!(
        full(store, policy, artifact.digest),
        Ok(expected_full(artifact, policy, decision))
    );
}

pub(super) fn persisted(
    artifact: &AcceptedArtifact,
) -> FixtureResult<(tempfile::TempDir, std::sync::Arc<vb_storage::FjallJournal>)> {
    let directory = tempfile::tempdir()?;
    let journal = std::sync::Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    journal.put_compiled_ir(&vb_storage::CompiledIrRecord {
        digest: artifact.digest,
        ir: postcard::to_allocvec(artifact)?,
    })?;
    Ok((directory, journal))
}

pub(super) fn public_version(
    held: SchemaVersion,
    class: CompatibilityClass,
    code: u16,
) -> FixtureResult {
    let decision = exact_decision(held, class, code)?;
    [RuntimePolicy::Strict, RuntimePolicy::Journaled]
        .into_iter()
        .try_for_each(|policy| {
            let artifact = bound(policy, held, &["key"])?;
            let store = FixedAcceptedStore {
                artifact: artifact.clone(),
            };
            admitted(&store, &artifact, policy, decision);
            assert_eq!(store.artifact, artifact);
            Ok(())
        })
}
