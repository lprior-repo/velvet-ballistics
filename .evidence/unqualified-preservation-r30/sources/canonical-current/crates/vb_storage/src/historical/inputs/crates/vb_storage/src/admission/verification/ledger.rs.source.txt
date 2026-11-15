#![forbid(unsafe_code)]

use super::*;

pub(super) fn effect_closure(
    contracts: &[ActionContract],
    verifier_digest: WorkflowDigest,
) -> Result<ObligationClosure, JournalError> {
    if contracts.is_empty() {
        return Ok(ObligationClosure::RuntimeValidated { verifier_digest });
    }
    let declarations =
        postcard::to_allocvec(contracts).map_err(JournalError::PostcardEncodeFailed)?;
    Ok(ObligationClosure::ExternalAssumption {
        assumption_id: AssumptionId::ExecutorHonorsActionContracts,
        evidence_digest: digest(&declarations),
    })
}

pub(super) fn obligation_ledger(
    policy_digest: WorkflowDigest,
    verifier: WorkflowDigest,
    effect_safety: ObligationClosure,
) -> Result<[VerificationLedgerEntry; 10], JournalError> {
    use ObligationKind::*;
    let validated = ObligationClosure::RuntimeValidated {
        verifier_digest: verifier,
    };
    let entry = |obligation, closure| VerificationLedgerEntry {
        obligation,
        closure,
        policy_digest,
    };
    Ok([
        entry(Determinism, validated),
        entry(EffectSafety, effect_safety),
        entry(CapabilityNonMinting, validated),
        entry(Boundedness, validated),
        entry(
            ReplayEffectSeparation,
            assumption(AssumptionId::RuntimeReplayEffectSeparation)?,
        ),
        entry(ArtifactValidity, validated),
        entry(IdempotencyClassification, validated),
        entry(
            RecoveryClassification,
            assumption(AssumptionId::RuntimeRecoverySemantics)?,
        ),
        entry(
            TaintHandling,
            assumption(AssumptionId::RuntimeTaintEnforcement)?,
        ),
        entry(PolicyBinding, validated),
    ])
}

fn assumption(assumption_id: AssumptionId) -> Result<ObligationClosure, JournalError> {
    // Identifies this explicit declaration; it is not a verifier or test result.
    let declaration = ("velvet-ballistics/external-assumption/v1", assumption_id);
    let bytes = postcard::to_allocvec(&declaration).map_err(JournalError::PostcardEncodeFailed)?;
    Ok(ObligationClosure::ExternalAssumption {
        assumption_id,
        evidence_digest: digest(&bytes),
    })
}
