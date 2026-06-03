#![forbid(unsafe_code)]
//! Core admission functions for runtime admission control.

use vb_core::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, validate_aggregate_budget,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_storage::EventSeq;

use super::errors::{AdmissionError, map_artifact_envelope_error};
use super::guards::capability_count_mismatch_error;
pub use super::guards::check_capability;
use super::stores::{AcceptedArtifactStore, ArtifactStore};
use super::types::{AdmissionBudgetRequest, RunAdmission};
use super::validation::validate_accepted_artifact_envelope;

/// Performs the admission gate check for a submit.
///
/// - Strict / Journaled: artifact must exist in the store.
/// - Relaxed: always succeeds.
///
/// Returns a `RunAdmission` on success or an `AdmissionError` on rejection.
pub fn admit_run(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
) -> Result<RunAdmission, AdmissionError> {
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            let artifact = store
                .load_accepted_artifact(digest)
                .map_err(map_artifact_envelope_error)?;
            validate_accepted_artifact_envelope(&artifact).map_err(map_artifact_envelope_error)?;
            if artifact.digest != digest {
                return Err(AdmissionError::ArtifactDigestMismatch {
                    requested: digest,
                    found: artifact.digest,
                });
            }
        }
        RuntimePolicy::Relaxed => {}
        _ => {
            return Err(AdmissionError::ArtifactInvalidProofFlag {
                flag: "runtime_policy",
            });
        }
    }
    Ok(RunAdmission::new(digest, run_id, caps, policy))
}

/// Performs full admission gate check with artifact validation before run creation.
///
/// For `RuntimePolicy::Strict` and `RuntimePolicy::Journaled`:
///   - Loads and validates the accepted artifact from storage
///   - Checks that the artifact has all 15 gates passing and proof flags set
///   - Validates that granted capabilities cover the artifact's required capabilities
///
/// For `RuntimePolicy::Relaxed`:
///   - Skips artifact loading and capability checking
///   - Returns a lightweight RunAdmission with no budget
///
/// Returns `Ok(RunAdmission)` on success, or an `AdmissionError` on rejection.
/// On error, no run frame is allocated, no run state is inserted, and no
/// `RunAccepted` journal event is recorded.
pub fn admit_artifact_run(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    run_id: RunId,
    artifact_digest: WorkflowDigest,
    caps: CapabilitySet,
) -> Result<RunAdmission, AdmissionError> {
    admit_artifact_run_with_certificate_floor(
        store,
        policy,
        run_id,
        artifact_digest,
        caps,
        EventSeq::ZERO,
    )
}

/// Performs full artifact admission with a caller-supplied certificate freshness floor.
///
/// This preserves relaxed-mode behavior and rejects Strict/Journaled artifacts whose
/// `accepted_at_seq` is below `required_at_least` after envelope validation.
pub fn admit_artifact_run_with_certificate_floor(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    run_id: RunId,
    artifact_digest: WorkflowDigest,
    caps: CapabilitySet,
    required_at_least: EventSeq,
) -> Result<RunAdmission, AdmissionError> {
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            // Load and validate the full artifact.
            let artifact = store
                .load_accepted_artifact(artifact_digest)
                .map_err(map_artifact_envelope_error)?;
            validate_accepted_artifact_envelope(&artifact).map_err(map_artifact_envelope_error)?;

            // INV-002: digest binding must be total. The loaded artifact's digest
            // must match the requested digest exactly — a crafted artifact with
            // valid gates but wrong identity must not be admitted.
            if artifact.digest != artifact_digest {
                return Err(AdmissionError::ArtifactDigestMismatch {
                    requested: artifact_digest,
                    found: artifact.digest,
                });
            }

            // INV-003: proof digest must match artifact digest. The verification
            // proof's digest field must bind to the artifact content exactly.
            if artifact.verification.digest != artifact.digest {
                return Err(AdmissionError::ArtifactDigestMismatch {
                    requested: artifact_digest,
                    found: artifact.verification.digest,
                });
            }

            if artifact.accepted_at_seq < required_at_least {
                return Err(AdmissionError::ArtifactCertificateStale {
                    digest: artifact_digest,
                    accepted_at_seq: artifact.accepted_at_seq,
                    required_at_least,
                });
            }

            // Check that granted capabilities cover the artifact's required capabilities.
            if caps.len() != artifact.required_capabilities.len() {
                return Err(capability_count_mismatch_error(
                    &artifact.required_capabilities,
                    &caps,
                ));
            }
            for required_cap in artifact.required_capabilities.iter() {
                check_capability(required_cap.action_id(), required_cap, &caps)?;
            }

            Ok(RunAdmission::with_idempotency_evidence(
                artifact_digest,
                run_id,
                caps,
                policy,
                artifact.verification.idempotency_attested,
            ))
        }
        RuntimePolicy::Relaxed => {
            // Relaxed: skip artifact loading and capability checking.
            Ok(RunAdmission::new(artifact_digest, run_id, caps, policy))
        }
        _ => Err(AdmissionError::ArtifactInvalidProofFlag {
            flag: "runtime_policy",
        }),
    }
}

/// Performs artifact admission plus aggregate resource capacity admission.
pub fn admit_run_with_budget(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    requested: AggregateResourceBudget,
    available: AggregateResourceCapacity,
) -> Result<RunAdmission, AdmissionError> {
    admit_run_with_budget_policy(
        store,
        policy,
        digest,
        run_id,
        caps,
        AdmissionBudgetRequest {
            requested,
            available,
            policy: BoundednessPolicy::DEFAULT,
        },
    )
}

/// Performs artifact admission plus policy and aggregate capacity admission.
pub fn admit_run_with_budget_policy(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    budget: AdmissionBudgetRequest,
) -> Result<RunAdmission, AdmissionError> {
    validate_aggregate_budget(&budget.requested, &budget.policy).map_err(map_budget_error)?;
    let requested_usage = AggregateResourceUsage::default()
        .try_add_budget(&budget.requested)
        .map_err(map_budget_error)?;
    requested_usage
        .check_policy(&budget.policy)
        .map_err(map_budget_error)?;
    requested_usage
        .fits_within(&budget.available)
        .map_err(map_budget_error)?;
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled if !store.compiled_ir_exists(digest) => {
            return Err(AdmissionError::ArtifactNotFound { digest });
        }
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {}
        RuntimePolicy::Relaxed => {}
        _ => {
            return Err(AdmissionError::ArtifactInvalidProofFlag {
                flag: "runtime_policy",
            });
        }
    }
    Ok(RunAdmission::with_budget(
        digest,
        run_id,
        caps,
        policy,
        budget.requested,
    ))
}

fn map_budget_error(error: AggregateBudgetError) -> AdmissionError {
    // NOTE: AggregateBudgetError is #[non_exhaustive]. This catch-all ensures
    // new error variants don't break existing code, but they lose specific semantics.
    // Consider adding explicit arms for new variants as they are added.
    match error {
        AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        } => AdmissionError::BudgetPolicyExceeded {
            resource,
            actual,
            limit,
        },
        AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        } => AdmissionError::ResourceCapacityExceeded {
            resource,
            requested,
            available,
        },
        AggregateBudgetError::Overflow { resource } => {
            AdmissionError::ResourceBudgetOverflow { resource }
        }
        AggregateBudgetError::Underflow { resource } => {
            AdmissionError::ResourceBudgetUnderflow { resource }
        }
        AggregateBudgetError::InvalidCapacity { resource } => {
            AdmissionError::ResourceBudgetInvalidCapacity { resource }
        }
        AggregateBudgetError::StepCeilingExceeded { requested, limit } => {
            AdmissionError::ResourceStepCeilingExceeded { requested, limit }
        }
        AggregateBudgetError::PerTickCeilingExceeded { requested, limit } => {
            AdmissionError::ResourcePerTickCeilingExceeded { requested, limit }
        }
        #[cfg(not(kani))]
        AggregateBudgetError::WorkflowBudget(_) => AdmissionError::BudgetPolicyExceeded {
            resource: "workflow_budget",
            actual: u64::MAX,
            limit: 0,
        },
        #[cfg(kani)]
        AggregateBudgetError::WorkflowBudget => AdmissionError::BudgetPolicyExceeded {
            resource: "workflow_budget",
            actual: u64::MAX,
            limit: 0,
        },
        AggregateBudgetError::ReservationNotFound { .. } => AdmissionError::BudgetPolicyExceeded {
            resource: "reservation_not_found",
            actual: u64::MAX,
            limit: 0,
        },
        _ => AdmissionError::BudgetPolicyExceeded {
            resource: "unknown_aggregate_budget_error", // DEAD: #[non_exhaustive] catch-all
            actual: u64::MAX,
            limit: 0,
        },
    }
}
