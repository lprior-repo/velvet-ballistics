#![forbid(unsafe_code)]
//! Runtime admission requires checked artifact evidence bound to the actual payload.
//! Historical proof flags and gate counts do not establish that authority.

use proptest::{prop_assert_eq, proptest};
use vb_core::artifact::ArtifactEvidence;
use vb_core::{CapabilitySet, RunId, RuntimePolicy, WorkflowDigest};
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, MissingAcceptedArtifactStore,
    admit_artifact_run,
};
use vb_storage::admission::{AcceptedArtifact, prepare_compiled_artifact};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: version_barrier\nwhen: { manual: {} }\nsteps:\n  - id: make\n    set: { output: answer, value: \"42\" }\n  - id: done\n    finish: { result: answer }\n";

struct FixedStore(AcceptedArtifact);

impl AcceptedArtifactStore for FixedStore {
    fn load_accepted_artifact(
        &self,
        _digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.0.clone())
    }
}

fn valid_artifact() -> Result<AcceptedArtifact, Box<dyn std::error::Error>> {
    let workflow = vb_compile::compile_workflow(SOURCE)?;
    Ok(prepare_compiled_artifact(
        &workflow,
        RuntimePolicy::Strict,
        &[],
    )?)
}

fn admit(artifact: AcceptedArtifact) -> Result<(), AdmissionError> {
    let digest = artifact.digest;
    admit_artifact_run(
        &FixedStore(artifact),
        RuntimePolicy::Strict,
        RunId::new(1),
        digest,
        CapabilitySet::empty(),
    )
    .map(|_| ())
}

fn evidence_error() -> Result<(), AdmissionError> {
    Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
}

#[test]
fn verified_artifact_admitted_without_fabricated_claims() -> TestResult {
    let artifact = valid_artifact()?;
    assert_eq!(artifact.verification.gate_count, 0);
    assert!(!artifact.verification.durable);
    assert!(!artifact.verification.replayable_claimed);
    assert!(!artifact.verification.retry_safe_claimed);
    assert!(!artifact.verification.taint_safe_claimed);
    assert_eq!(admit(artifact), Ok(()));
    Ok(())
}

#[test]
fn all_legacy_claims_true_do_not_establish_evidence() -> TestResult {
    let mut artifact = valid_artifact()?;
    let proof = &mut artifact.verification;
    proof.evidence = ArtifactEvidence::Unverified;
    proof.gate_count = 15;
    proof.durable = true;
    proof.bounded_claimed = true;
    proof.taint_safe_claimed = true;
    proof.retry_safe_claimed = true;
    proof.idempotency_verified_claimed = true;
    proof.replayable_claimed = true;
    assert_eq!(admit(artifact), evidence_error());
    Ok(())
}

#[test]
fn changed_proof_claims_rejected_even_after_checksum_rebinding() -> TestResult {
    let base = valid_artifact()?;
    for field in 0..7 {
        let mut artifact = base.clone();
        let proof = &mut artifact.verification;
        match field {
            0 => proof.bounded_claimed = !proof.bounded_claimed,
            1 => proof.taint_safe_claimed = !proof.taint_safe_claimed,
            2 => proof.retry_safe_claimed = !proof.retry_safe_claimed,
            3 => proof.durable = !proof.durable,
            4 => proof.replayable_claimed = !proof.replayable_claimed,
            5 => proof.idempotency_verified_claimed = !proof.idempotency_verified_claimed,
            _ => proof.gate_count = 15,
        }
        artifact.digest = vb_storage::admission::accepted_artifact_digest(&artifact)?;
        artifact.verification.digest = artifact.digest;
        assert_eq!(admit(artifact), evidence_error(), "changed field {field}");
    }
    Ok(())
}

#[test]
fn unsupported_evidence_version_rejected() -> TestResult {
    let mut artifact = valid_artifact()?;
    let ArtifactEvidence::Verified(evidence) = &mut artifact.verification.evidence else {
        return Err("fixture lacks verified evidence".into());
    };
    evidence.schema_version = u16::MAX;
    assert_eq!(admit(artifact), evidence_error());
    Ok(())
}

#[test]
fn matching_digest_does_not_authorize_changed_ir() -> TestResult {
    let mut artifact = valid_artifact()?;
    artifact.ir.clear();
    assert_eq!(admit(artifact), evidence_error());
    Ok(())
}

#[test]
fn requested_digest_mismatch_has_exact_error() -> TestResult {
    let artifact = valid_artifact()?;
    let found = artifact.digest;
    let requested = WorkflowDigest::from_bytes([1; 32]);
    assert_ne!(found, requested);
    assert_eq!(
        admit_artifact_run(
            &FixedStore(artifact),
            RuntimePolicy::Strict,
            RunId::new(2),
            requested,
            CapabilitySet::empty(),
        ),
        Err(AdmissionError::ArtifactDigestMismatch { requested, found }),
    );
    Ok(())
}

#[test]
fn relaxed_policy_skips_evidence_validation() -> TestResult {
    let mut artifact = valid_artifact()?;
    artifact.verification.evidence = ArtifactEvidence::Unverified;
    let digest = artifact.digest;
    let run_id = RunId::new(3);
    let result = admit_artifact_run(
        &FixedStore(artifact),
        RuntimePolicy::Relaxed,
        run_id,
        digest,
        CapabilitySet::empty(),
    );
    assert!(
        result.is_ok(),
        "relaxed policy should accept artifact with any evidence"
    );
    let admission = result?;
    assert_eq!(
        admission.artifact_digest(),
        digest,
        "admission must carry exact admitted artifact digest"
    );
    assert_eq!(
        admission.run_id(),
        run_id,
        "admission must carry exact run identity"
    );
    Ok(())
}

#[test]
fn missing_artifact_rejected_under_strict_policy() {
    let digest = WorkflowDigest::from_bytes([0; 32]);
    assert_eq!(
        admit_artifact_run(
            MissingAcceptedArtifactStore::shared().as_ref(),
            RuntimePolicy::Strict,
            RunId::new(4),
            digest,
            CapabilitySet::empty(),
        ),
        Err(AdmissionError::ArtifactNotFound { digest }),
    );
}

#[test]
fn unsupported_step_primitive_rejected_at_compile() {
    let yaml = b"version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: collect_pages\n    collect:\n      variable: page\n      source: \"0\"\n      steps:\n        - id: inner\n          collect:\n            variable: inner_page\n            source: \"1\"\n  - id: done\n    finish:\n      result: 0\n";
    let result = vb_compile::YamlCompiler::default().compile(yaml);
    assert!(matches!(result, Err(errors) if matches!(
        errors.first(), Some(vb_compile::CompileError::UnsupportedStepPrimitive { .. })
    )));
}

proptest! {
    #[test]
    fn any_claimed_gate_count_without_evidence_is_rejected(count in proptest::num::u8::ANY) {
        let artifact = valid_artifact().map(|mut artifact| {
            artifact.verification.evidence = ArtifactEvidence::Unverified;
            artifact.verification.gate_count = count;
            artifact
        });
        let result = artifact.map(admit).map_err(|error| error.to_string());
        prop_assert_eq!(result, Ok(evidence_error()));
    }
}
