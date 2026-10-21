#![forbid(unsafe_code)]

mod artifact_evidence_support;
use artifact_evidence_support::*;
use vb_compile::{ArtifactVerificationError, CompilationBinding, CompilerCheck, SourceProvenance};
use vb_core::artifact::{ArtifactEvidence, AssumptionId, ObligationClosure, ObligationKind};
use vb_core::{RuntimePolicy, WorkflowDigest};
use vb_storage::admission::{
    prepare_compiled_artifact, verify_accepted_artifact, verify_accepted_artifact_for_policy,
};
use vb_storage::{JournalError, VerificationProof};
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "artifact_evidence_support/compiler_cases.rs"]
mod compiler_cases;
#[path = "artifact_evidence_support/tamper_cases.rs"]
mod tamper_cases;

#[test]
fn storage_rejects_missing_and_orphan_action_contracts() -> TestResult {
    match prepare_compiled_artifact(&action_workflow()?, RuntimePolicy::Journaled, &[]) {
        Err(JournalError::ArtifactValidation(
            vb_validate::ValidationError::ActionContractMissing {
                action_id,
                node_index,
            },
        )) => assert_eq!((action_id, node_index), (7, 1)),
        other => return Err(format!("expected missing contract, got {other:?}").into()),
    }
    match prepare_compiled_artifact(&workflow()?, RuntimePolicy::Journaled, &[contract()?]) {
        Err(JournalError::ArtifactValidation(
            vb_validate::ValidationError::ActionContractOrphan { action_id },
        )) => assert_eq!(action_id, 7),
        other => return Err(format!("expected orphan contract, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn accepted_ledger_distinguishes_executed_checks_and_unproved_runtime_assumptions() -> TestResult {
    let workflow = workflow()?;
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let artifact = prepare_compiled_artifact(&workflow, policy, &[])?;
        assert_eq!(
            (
                artifact.verification.gate_count,
                artifact.verification.durable
            ),
            (0, false)
        );
        assert_eq!(
            verify_accepted_artifact_for_policy(&artifact, policy)?,
            workflow
        );
        let ArtifactEvidence::Verified(evidence) = &artifact.verification.evidence else {
            return Err("missing typed ledger".into());
        };
        assert_eq!(evidence.workflow_digest, workflow.digest());
        assert_eq!(
            evidence.payload_digest,
            WorkflowDigest::from_bytes(blake3::hash(&artifact.ir).into())
        );
        assert_eq!(evidence.policy_digest, artifact.policy_digest);
        assert_eq!(
            evidence.action_contract_digest,
            WorkflowDigest::from_bytes(
                blake3::hash(&postcard::to_allocvec::<[vb_core::ActionContract]>(&[])?).into()
            )
        );
        assert_eq!(
            evidence.ledger.map(|entry| entry.obligation),
            [
                ObligationKind::Determinism,
                ObligationKind::EffectSafety,
                ObligationKind::CapabilityNonMinting,
                ObligationKind::Boundedness,
                ObligationKind::ReplayEffectSeparation,
                ObligationKind::ArtifactValidity,
                ObligationKind::IdempotencyClassification,
                ObligationKind::RecoveryClassification,
                ObligationKind::TaintHandling,
                ObligationKind::PolicyBinding
            ]
        );
        for entry in evidence.ledger {
            assert_eq!(entry.policy_digest, artifact.policy_digest);
            if entry.obligation == ObligationKind::ReplayEffectSeparation {
                match entry.closure {
                    ObligationClosure::ExternalAssumption { assumption_id, .. } => {
                        assert_eq!(assumption_id, AssumptionId::RuntimeReplayEffectSeparation)
                    }
                    other => return Err(format!("fabricated replay proof: {other:?}").into()),
                }
            }
        }
    }
    Ok(())
}

#[test]
fn strict_side_effects_fail_without_executor_conformance_and_cannot_upgrade_policy() -> TestResult {
    let workflow = action_workflow()?;
    let contracts = [contract()?];
    let artifact = prepare_compiled_artifact(&workflow, RuntimePolicy::Journaled, &contracts)?;
    assert_eq!(verify_accepted_artifact(&artifact)?, workflow);
    match prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &contracts) {
        Err(JournalError::MissingRequiredProofFlag { flag }) => {
            assert_eq!(flag, "executor_conformance")
        }
        other => return Err(format!("strict accepted unattested executor: {other:?}").into()),
    }
    assert!(matches!(
        verify_accepted_artifact_for_policy(&artifact, RuntimePolicy::Strict),
        Err(JournalError::ArtifactMalformed)
    ));
    let ArtifactEvidence::Verified(evidence) = &artifact.verification.evidence else {
        return Err("missing typed ledger".into());
    };
    let entry = evidence
        .ledger
        .iter()
        .find(|entry| entry.obligation == ObligationKind::EffectSafety)
        .ok_or("missing effect obligation")?;
    assert_eq!(
        entry.closure,
        ObligationClosure::ExternalAssumption {
            assumption_id: AssumptionId::ExecutorHonorsActionContracts,
            evidence_digest: evidence.action_contract_digest
        }
    );
    assert_eq!(artifact.verification.idempotency_keyed.as_ref(), &[]);
    assert_eq!(artifact.verification.idempotency_attested.as_ref(), &[]);
    Ok(())
}

#[test]
fn rehashed_forged_closure_and_duplicate_ledger_entries_are_rejected() -> TestResult {
    let original = prepare_compiled_artifact(&workflow()?, RuntimePolicy::Journaled, &[])?;
    for duplicate in [false, true] {
        let mut artifact = original.clone();
        let ArtifactEvidence::Verified(evidence) = &mut artifact.verification.evidence else {
            return Err("missing typed ledger".into());
        };
        let first = *evidence.ledger.first().ok_or("missing first entry")?;
        let replay = evidence
            .ledger
            .iter_mut()
            .find(|entry| entry.obligation == ObligationKind::ReplayEffectSeparation)
            .ok_or("missing replay obligation")?;
        if duplicate {
            *replay = first;
        } else {
            replay.closure = ObligationClosure::RuntimeValidated {
                verifier_digest: evidence.verifier_digest,
            };
        }
        rebind(&mut artifact)?;
        assert!(matches!(
            verify_accepted_artifact(&artifact),
            Err(JournalError::ArtifactMalformed)
        ));
    }
    Ok(())
}

#[test]
fn every_identity_binding_rejects_rehashed_tampering() -> TestResult {
    let original = prepare_compiled_artifact(&workflow()?, RuntimePolicy::Journaled, &[])?;
    for role in 0..7 {
        let mut artifact = original.clone();
        let bad = WorkflowDigest::from_bytes([9; 32]);
        match role {
            0 => artifact.source_digest = bad,
            1 => artifact.policy_digest = bad,
            _ => {
                let ArtifactEvidence::Verified(evidence) = &mut artifact.verification.evidence
                else {
                    return Err("missing typed ledger".into());
                };
                match role {
                    2 => evidence.payload_digest = bad,
                    3 => evidence.workflow_digest = bad,
                    4 => evidence.action_contract_digest = bad,
                    5 => evidence.policy_digest = bad,
                    6 => evidence.verifier_digest = bad,
                    _ => return Err("invalid test role".into()),
                }
            }
        }
        rebind(&mut artifact)?;
        assert!(
            matches!(
                verify_accepted_artifact(&artifact),
                Err(JournalError::ArtifactMalformed)
            ),
            "binding role {role}"
        );
    }
    Ok(())
}

#[test]
fn legacy_and_unknown_schema_fail_closed() -> TestResult {
    let mut artifact = prepare_compiled_artifact(&workflow()?, RuntimePolicy::Journaled, &[])?;
    artifact.verification.evidence = ArtifactEvidence::Unverified;
    rebind(&mut artifact)?;
    assert!(matches!(
        verify_accepted_artifact(&artifact),
        Err(JournalError::MigrationRequired { from: 1, to: 2 })
    ));
    let mut artifact = prepare_compiled_artifact(&workflow()?, RuntimePolicy::Journaled, &[])?;
    let ArtifactEvidence::Verified(evidence) = &mut artifact.verification.evidence else {
        return Err("missing typed ledger".into());
    };
    evidence.schema_version = 999;
    rebind(&mut artifact)?;
    assert!(matches!(
        verify_accepted_artifact(&artifact),
        Err(JournalError::UnsupportedSchemaVersion { version: 999 })
    ));
    Ok(())
}

#[test]
fn persisted_artifact_reopens_with_checked_identity_and_runtime_admission() -> TestResult {
    use vb_runtime::admission::{AcceptedArtifactStore, StorageArtifactStore};
    let dir = tempfile::tempdir()?;
    let workflow = workflow()?;
    let digest = {
        let journal = vb_storage::FjallJournal::open(dir.path(), None)?;
        let artifact = vb_storage::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)?;
        artifact.digest
    };
    let journal = std::sync::Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    let store = StorageArtifactStore::new(journal);
    let artifact = store.load_accepted_artifact(digest)?;
    assert_eq!(
        verify_accepted_artifact_for_policy(&artifact, RuntimePolicy::Strict)?,
        workflow
    );
    let admission = vb_runtime::admission::admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        vb_core::RunId::new(77),
        digest,
        vb_core::capability::CapabilitySet::empty(),
    )?;
    assert_eq!(admission.artifact_digest(), digest);
    Ok(())
}
