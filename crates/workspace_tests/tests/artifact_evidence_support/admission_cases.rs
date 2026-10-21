use super::*;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::{ActionId, RunId};
use vb_runtime::admission::{AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError};
use vb_storage::AcceptedArtifact;

struct FixtureStore(AcceptedArtifact);
impl AcceptedArtifactStore for FixtureStore {
    fn load_accepted_artifact(
        &self,
        _: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.0.clone())
    }
}

#[test]
fn actual_do_capabilities_are_bound_and_public_admission_requires_exact_grants() -> TestResult {
    let mut parts = action_workflow()?.to_parts();
    for node in parts.nodes.iter_mut() {
        if let vb_core::CompiledNodeKind::Do { action, .. } = &mut node.kind {
            *action = ActionId::new(0);
        }
    }
    let workflow = canonicalize(parts)?;
    let required = Capability::new("network.github".into(), ActionId::new(0));
    let mut action_contract = contract()?;
    action_contract.id = ActionId::new(0);
    action_contract.side_effect = vb_core::SideEffect::None;
    action_contract.idempotency = vb_core::Idempotency::DeterministicPure;
    action_contract.retry_safety = vb_core::RetrySafety::Safe;
    action_contract.required_capabilities = Box::new([required.clone()]);
    let artifact = prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[action_contract])?;
    assert_eq!(artifact.required_capabilities.as_ref(), &[required.clone()]);
    assert_eq!(verify_accepted_artifact(&artifact)?, workflow);
    let exact = CapabilitySet::from_grants(Box::new([required.clone()]));
    let store = FixtureStore(artifact.clone());
    let admitted = vb_runtime::admission::admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(11),
        artifact.digest,
        exact.clone(),
    )?;
    assert_eq!(admitted.artifact_digest(), artifact.digest);
    assert_eq!(admitted.granted_capabilities(), &exact);
    let missing = CapabilitySet::empty();
    assert_eq!(
        vb_runtime::admission::admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(12),
            artifact.digest,
            missing.clone()
        ),
        Err(AdmissionError::CapabilityDenied {
            action: ActionId::new(0),
            required: required.clone(),
            granted: missing
        })
    );
    let over = CapabilitySet::from_grants(Box::new([
        required,
        Capability::new("extra".into(), ActionId::new(1)),
    ]));
    assert_eq!(
        vb_runtime::admission::admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(13),
            artifact.digest,
            over
        ),
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 1,
            granted_count: 2
        })
    );
    let mut forged = artifact;
    forged.required_capabilities = Box::new([]);
    rebind(&mut forged)?;
    assert!(matches!(
        verify_accepted_artifact(&forged),
        Err(JournalError::ArtifactMalformed)
    ));
    let digest = forged.digest;
    assert_eq!(
        vb_runtime::admission::admit_artifact_run(
            &FixtureStore(forged),
            RuntimePolicy::Strict,
            RunId::new(14),
            digest,
            CapabilitySet::empty()
        ),
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn public_runtime_admission_rejects_journaled_artifact_policy_upgrade() -> TestResult {
    let artifact = prepare_compiled_artifact(&workflow()?, RuntimePolicy::Journaled, &[])?;
    let digest = artifact.digest;
    let store = FixtureStore(artifact);
    let admitted = vb_runtime::admission::admit_artifact_run(
        &store,
        RuntimePolicy::Journaled,
        RunId::new(20),
        digest,
        CapabilitySet::empty(),
    )?;
    assert_eq!(admitted.policy(), RuntimePolicy::Journaled);
    assert_eq!(
        vb_runtime::admission::admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(21),
            digest,
            CapabilitySet::empty()
        ),
        Err(AdmissionError::ArtifactInvalidProofFlag {
            flag: "admission_policy"
        })
    );
    assert_eq!(
        vb_runtime::admission::admit_run(
            &store,
            RuntimePolicy::Strict,
            digest,
            RunId::new(22),
            CapabilitySet::empty()
        ),
        Err(AdmissionError::ArtifactInvalidProofFlag {
            flag: "admission_policy"
        })
    );
    Ok(())
}

pub(super) fn expected_verifier_digest() -> WorkflowDigest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(include_bytes!(
        "../../../vb_storage/src/admission/verification.rs"
    ));
    hasher.update(include_bytes!("../../../vb_core/src/workflow/mod.rs"));
    hasher.update(include_bytes!("../../../vb_core/src/budget.rs"));
    hasher.update(include_bytes!("../../../vb_core/src/action/registry.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/shared.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/gates.rs"));
    WorkflowDigest::from_bytes(hasher.finalize().into())
}

pub(super) fn expected_assumption(id: AssumptionId) -> Result<ObligationClosure, postcard::Error> {
    Ok(ObligationClosure::ExternalAssumption {
        assumption_id: id,
        evidence_digest: WorkflowDigest::from_bytes(
            blake3::hash(&postcard::to_allocvec(&(
                "velvet-ballistics/external-assumption/v1",
                id,
            ))?)
            .into(),
        ),
    })
}
