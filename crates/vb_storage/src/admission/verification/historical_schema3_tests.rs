//! Synthetic provenance fixtures test comparison logic, not an old binary.
//! Authentic two-version journal/recovery qualification remains a separate gate.

use super::*;
use crate::executor::{ExecutorCatalog, ExecutorError};

#[path = "../../executor/issuance_test_support.rs"]
mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn historical_fingerprints() -> Result<[WorkflowDigest; 2], Box<dyn std::error::Error>> {
    let hex = [
        "6399d34635e7cf7ce61daa4410ce8b0c7a68b7c89e87019bdf886e8fa29e1848",
        "91544c6ca0b13bd85f62e62cc03adbe5c6eb103449cdecb232e7e08c60cd523e",
    ];
    let mut result = [zero_workflow_digest(); 2];
    for (destination, text) in result.iter_mut().zip(hex) {
        let mut bytes = [0; 32];
        for (byte, pair) in bytes.iter_mut().zip(text.as_bytes().chunks_exact(2)) {
            *byte = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
        }
        *destination = WorkflowDigest::from_bytes(bytes);
    }
    Ok(result)
}

fn evidence_mut(
    artifact: &mut AcceptedArtifact,
) -> Result<&mut ArtifactVerificationEvidence, Box<dyn std::error::Error>> {
    match &mut artifact.verification.evidence {
        ArtifactEvidence::Verified(evidence) => Ok(evidence),
        ArtifactEvidence::Packed(evidence) => Ok(&mut evidence.base),
        ArtifactEvidence::Unverified => Err("missing synthetic fixture evidence".into()),
    }
}

// Independent fixture construction: never call the production compatibility helper.
fn stamp_provenance(
    artifact: &mut AcceptedArtifact,
    header: WorkflowDigest,
    closure: WorkflowDigest,
) -> TestResult {
    let evidence = evidence_mut(artifact)?;
    evidence.verifier_digest = header;
    for entry in &mut evidence.ledger {
        if let ObligationClosure::RuntimeValidated { verifier_digest } = &mut entry.closure {
            *verifier_digest = closure;
        }
    }
    bind_artifact_digest(artifact)?;
    Ok(())
}

fn assert_malformed_after_rehash(artifact: &mut AcceptedArtifact) -> TestResult {
    bind_artifact_digest(artifact)?;
    assert_eq!(accepted_artifact_digest(artifact)?, artifact.digest);
    assert!(matches!(
        verify_accepted_artifact(artifact),
        Err(JournalError::ArtifactMalformed)
    ));
    Ok(())
}

#[test]
fn newly_prepared_artifacts_keep_current_provenance() -> TestResult {
    let workflow = fixture::minimal_workflow()?;
    let artifact = prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let ArtifactEvidence::Verified(evidence) = &artifact.verification.evidence else {
        return Err("missing current evidence".into());
    };
    assert_eq!(evidence.verifier_digest, verifier_digest());
    for historical in historical_fingerprints()? {
        assert_ne!(evidence.verifier_digest, historical);
    }
    assert_eq!(
        verify_accepted_artifact_for_policy(&artifact, RuntimePolicy::Strict)?,
        workflow
    );
    Ok(())
}

#[test]
fn both_historical_provenances_preserve_the_complete_retained_envelope() -> TestResult {
    let workflow = fixture::minimal_workflow()?;
    for historical in historical_fingerprints()? {
        for policy in [
            RuntimePolicy::Relaxed,
            RuntimePolicy::Journaled,
            RuntimePolicy::Strict,
        ] {
            let mut artifact = prepare_compiled_artifact(&workflow, policy, &[])?;
            artifact.accepted_at_seq = EventSeq::new(37);
            stamp_provenance(&mut artifact, historical, historical)?;
            let before = artifact.clone();
            let bytes = postcard::to_allocvec(&artifact)?;
            let proof_bytes = postcard::to_allocvec(&artifact.verification)?;
            assert_eq!(
                verify_accepted_artifact_for_policy(&artifact, policy)?,
                workflow
            );
            assert_eq!(artifact, before);
            assert_eq!(postcard::to_allocvec(&artifact)?, bytes);
            assert_eq!(postcard::to_allocvec(&artifact.verification)?, proof_bytes);
            assert_eq!(artifact.accepted_at_seq, EventSeq::new(37));
        }
    }
    Ok(())
}

#[test]
fn unknown_and_mixed_provenance_fail_after_valid_envelope_checksum() -> TestResult {
    let workflow = fixture::minimal_workflow()?;
    let [old_a, old_b] = historical_fingerprints()?;
    let current = verifier_digest();
    let unknown = WorkflowDigest::from_bytes([0x5a; 32]);
    for (header, closure) in [
        (unknown, unknown),
        (old_a, current),
        (current, old_a),
        (old_a, old_b),
        (old_b, old_a),
        (old_b, unknown),
    ] {
        let mut artifact = prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
        stamp_provenance(&mut artifact, header, closure)?;
        assert_malformed_after_rehash(&mut artifact)?;
    }
    Ok(())
}

#[test]
fn rehashed_historical_ledgers_cannot_change_obligations_or_claims() -> TestResult {
    let workflow = fixture::minimal_workflow()?;
    for historical in historical_fingerprints()? {
        for mutation in 0..10 {
            let mut artifact = prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
            stamp_provenance(&mut artifact, historical, historical)?;
            match mutation {
                0 => evidence_mut(&mut artifact)?.ledger.swap(0, 1),
                1 => {
                    let entry = evidence_mut(&mut artifact)?
                        .ledger
                        .first_mut()
                        .ok_or("empty ledger")?;
                    entry.obligation = ObligationKind::EffectSafety;
                }
                2 => {
                    let entry = evidence_mut(&mut artifact)?
                        .ledger
                        .first_mut()
                        .ok_or("empty ledger")?;
                    entry.policy_digest = WorkflowDigest::from_bytes([0x27; 32]);
                }
                3 => {
                    let entry = evidence_mut(&mut artifact)?
                        .ledger
                        .iter_mut()
                        .find(|entry| {
                            matches!(entry.closure, ObligationClosure::ExternalAssumption { .. })
                        })
                        .ok_or("missing assumption control")?;
                    if let ObligationClosure::ExternalAssumption {
                        evidence_digest, ..
                    } = &mut entry.closure
                    {
                        *evidence_digest = WorkflowDigest::from_bytes([0x38; 32]);
                    }
                }
                4 => artifact.verification.durable = true,
                5 => artifact.verification.gate_count = 1,
                6 => artifact.verification.replayable_claimed = true,
                7 => artifact.verification.bounded_claimed = false,
                8 => {
                    artifact.verification.idempotency_keyed = Box::new([vb_core::ActionId::new(0)])
                }
                _ => {
                    evidence_mut(&mut artifact)?.payload_digest =
                        WorkflowDigest::from_bytes([0x49; 32])
                }
            }
            assert_malformed_after_rehash(&mut artifact)?;
        }
    }
    Ok(())
}

#[test]
fn historical_action_evidence_preserves_its_external_assumption() -> TestResult {
    let workflow = fixture::action_workflow()?;
    let contract = fixture::action_contract()?;
    for historical in historical_fingerprints()? {
        let mut artifact = prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Journaled,
            std::slice::from_ref(&contract),
        )?;
        stamp_provenance(&mut artifact, historical, historical)?;
        let before = artifact.clone();
        assert_eq!(
            verify_accepted_artifact_for_policy(&artifact, RuntimePolicy::Journaled)?,
            workflow
        );
        assert_eq!(artifact, before);
        let evidence = evidence_mut(&mut artifact)?;
        let effect = evidence
            .ledger
            .iter()
            .find(|entry| entry.obligation == ObligationKind::EffectSafety)
            .ok_or("missing effect safety")?;
        assert!(matches!(
            effect.closure,
            ObligationClosure::ExternalAssumption {
                assumption_id: AssumptionId::ExecutorHonorsActionContracts,
                ..
            }
        ));
        evidence.admission_policy = RuntimePolicy::Strict;
        bind_artifact_digest(&mut artifact)?;
        assert!(matches!(
            verify_accepted_artifact(&artifact),
            Err(JournalError::MissingRequiredProofFlag {
                flag: "executor_conformance"
            })
        ));
    }
    Ok(())
}

#[test]
fn historical_packed_artifacts_still_require_the_actual_catalog_and_packs() -> TestResult {
    let fixture = fixture::PackedFixture::new()?;
    for historical in historical_fingerprints()? {
        let mut artifact = fixture.prepare(RuntimePolicy::Strict)?;
        artifact.accepted_at_seq = EventSeq::new(73);
        stamp_provenance(&mut artifact, historical, historical)?;
        let before = postcard::to_allocvec(&artifact)?;
        assert_eq!(
            fixture
                .catalog
                .verify_artifact(&artifact, RuntimePolicy::Strict)?,
            fixture.workflow
        );
        assert_eq!(postcard::to_allocvec(&artifact)?, before);
        assert!(matches!(
            verify_accepted_artifact(&artifact),
            Err(JournalError::ArtifactMalformed)
        ));
        for missing in [false, true] {
            let mut forged = artifact.clone();
            let ArtifactEvidence::Packed(packed) = &mut forged.verification.evidence else {
                return Err("missing packed fixture".into());
            };
            if missing {
                packed.packs = Box::new([]);
            } else {
                packed
                    .packs
                    .first_mut()
                    .ok_or("missing pack")?
                    .timeout
                    .timeout_ms = 1;
            }
            bind_artifact_digest(&mut forged)?;
            assert_eq!(accepted_artifact_digest(&forged)?, forged.digest);
            assert!(matches!(
                fixture
                    .catalog
                    .verify_artifact(&forged, RuntimePolicy::Strict),
                Err(ExecutorError::PackMismatch)
            ));
        }
    }
    Ok(())
}

#[test]
fn historical_provenance_still_runs_current_action_contract_validation() -> TestResult {
    let workflow = fixture::action_workflow()?;
    let contract = fixture::action_contract()?;
    assert_eq!(
        CompiledWorkflow::try_from_parts(workflow.to_parts())?,
        workflow
    );
    for historical in historical_fingerprints()? {
        let mut artifact = prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Journaled,
            std::slice::from_ref(&contract),
        )?;
        stamp_provenance(&mut artifact, historical, historical)?;
        assert_eq!(
            verify_accepted_artifact_for_policy(&artifact, RuntimePolicy::Journaled)?,
            workflow
        );
        artifact.action_contracts = Box::new([]);
        bind_artifact_digest(&mut artifact)?;
        assert_eq!(accepted_artifact_digest(&artifact)?, artifact.digest);
        assert!(matches!(
            verify_accepted_artifact(&artifact),
            Err(JournalError::ArtifactValidation(
                vb_validate::ValidationError::ActionContractMissing {
                    action_id: 0,
                    node_index: 1
                }
            ))
        ));
    }
    Ok(())
}
