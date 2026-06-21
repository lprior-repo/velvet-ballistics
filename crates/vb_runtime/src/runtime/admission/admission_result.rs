#![forbid(unsafe_code)]
//! Admission result and error-mapping types for the runtime admission pipeline.
//!
//! This module provides the translation chain from crate-level admission
//! errors back into `RuntimeError` variants so that the preflight can
//! fail closed with a domain-specific failure code.
//!
//! ```text
//! AggregateBudgetError → AdmissionError → RuntimeError
//!   crate::admission::AdmissionError → RuntimeError
//! ```

use vb_core::WorkflowDigest;
use vb_core::budget::AggregateBudgetError;

/// Convert a [`AggregateBudgetError`](vb_core::budget::AggregateBudgetError)
/// into an [`AdmissionError`](crate::admission::AdmissionError) via the
/// staged mapping chain.
fn aggregate_budget_to_admission_error(
    error: AggregateBudgetError,
) -> crate::admission::AdmissionError {
    match error {
        AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        } => crate::admission::AdmissionError::BudgetPolicyExceeded {
            resource,
            actual,
            limit,
        },
        AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        } => crate::admission::AdmissionError::ResourceCapacityExceeded {
            resource,
            requested,
            available,
        },
        other => aggregate_budget_resource_error(other),
    }
}

/// Maps non-policy/capacity [`AggregateBudgetError`](vb_core::budget::AggregateBudgetError)
/// variants into their corresponding admission errors.
fn aggregate_budget_resource_error(
    error: AggregateBudgetError,
) -> crate::admission::AdmissionError {
    match error {
        AggregateBudgetError::Overflow { resource } => {
            crate::admission::AdmissionError::ResourceBudgetOverflow { resource }
        }
        AggregateBudgetError::Underflow { resource } => {
            crate::admission::AdmissionError::ResourceBudgetUnderflow { resource }
        }
        AggregateBudgetError::InvalidCapacity { resource } => {
            crate::admission::AdmissionError::ResourceBudgetInvalidCapacity { resource }
        }
        other => aggregate_budget_ceiling_error(other),
    }
}

/// Maps ceiling-level [`AggregateBudgetError`](vb_core::budget::AggregateBudgetError)
/// variants into admission errors.
fn aggregate_budget_ceiling_error(error: AggregateBudgetError) -> crate::admission::AdmissionError {
    match error {
        AggregateBudgetError::StepCeilingExceeded { requested, limit } => {
            crate::admission::AdmissionError::ResourceStepCeilingExceeded { requested, limit }
        }
        AggregateBudgetError::PerTickCeilingExceeded { requested, limit } => {
            crate::admission::AdmissionError::ResourcePerTickCeilingExceeded { requested, limit }
        }
        other => aggregate_budget_terminal_error(other),
    }
}

/// Maps terminal [`AggregateBudgetError`](vb_core::budget::AggregateBudgetError)
/// variants to a fallback admission error.
fn aggregate_budget_terminal_error(
    error: AggregateBudgetError,
) -> crate::admission::AdmissionError {
    match error {
        AggregateBudgetError::ReservationNotFound { .. } => aggregate_budget_fallback_error(),
        #[cfg(not(kani))]
        AggregateBudgetError::WorkflowBudget(_) => aggregate_budget_fallback_error(),
        #[cfg(kani)]
        AggregateBudgetError::WorkflowBudget => aggregate_budget_fallback_error(),
        _ => aggregate_budget_fallback_error(),
    }
}

/// Produces a fallback [`AdmissionError::BudgetPolicyExceeded`](crate::admission::AdmissionError)
/// for unclassifiable aggregate-budget errors.
fn aggregate_budget_fallback_error() -> crate::admission::AdmissionError {
    crate::admission::AdmissionError::BudgetPolicyExceeded {
        resource: "aggregate_budget",
        actual: u64::MAX,
        limit: 0,
    }
}

/// Maps a crate-level [`AdmissionError`](crate::admission::AdmissionError)
/// into a [`RuntimeError`](crate::RuntimeError).
///
/// RA-001: the original implementation collapsed every variant not explicitly
/// handled into `RuntimeError::AdmissionArtifactInvalid`, destroying the
/// distinction between artifact-corruption failures and resource/budget
/// exhaustion failures. This implementation preserves the typed failure class
/// for every known variant and only falls through to the artifact-class
/// fallback when a future `AdmissionError` variant is added that lacks a
/// dedicated `RuntimeError` arm.
#[allow(unreachable_patterns)] // `AdmissionError` is `#[non_exhaustive]`; the wildcard is required for future variants.
pub(super) fn map_admission_error(
    error: crate::admission::AdmissionError,
    workflow_digest: WorkflowDigest,
) -> crate::RuntimeError {
    match error {
        crate::admission::AdmissionError::ArtifactNotFound { digest } => {
            crate::RuntimeError::AdmissionArtifactNotFound { digest }
        }
        crate::admission::AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        } => crate::RuntimeError::AdmissionCapabilityDenied {
            action,
            required,
            granted,
        },
        crate::admission::AdmissionError::BudgetExceeded { actual, limit } => {
            crate::RuntimeError::AdmissionBudgetExceeded { actual, limit }
        }
        crate::admission::AdmissionError::ResourceCapacityExceeded {
            resource,
            requested,
            available,
        } => crate::RuntimeError::AdmissionResourceCapacityExceeded {
            resource,
            requested,
            available,
        },
        crate::admission::AdmissionError::BudgetPolicyExceeded {
            resource,
            actual,
            limit,
        } => crate::RuntimeError::AdmissionBudgetPolicyExceeded {
            resource,
            actual,
            limit,
        },
        crate::admission::AdmissionError::ResourceBudgetOverflow { resource } => {
            crate::RuntimeError::AdmissionResourceBudgetOverflow { resource }
        }
        crate::admission::AdmissionError::ResourceBudgetUnderflow { resource } => {
            crate::RuntimeError::AdmissionResourceBudgetUnderflow { resource }
        }
        crate::admission::AdmissionError::ResourceBudgetInvalidCapacity { resource } => {
            crate::RuntimeError::AdmissionResourceBudgetInvalidCapacity { resource }
        }
        crate::admission::AdmissionError::ResourceStepCeilingExceeded { requested, limit } => {
            crate::RuntimeError::AdmissionBudgetExceeded {
                actual: u32::try_from(requested).unwrap_or(u32::MAX),
                limit: u32::try_from(limit).unwrap_or(u32::MAX),
            }
        }
        crate::admission::AdmissionError::ResourcePerTickCeilingExceeded {
            requested,
            limit,
        } => crate::RuntimeError::AdmissionBudgetExceeded {
            actual: u32::try_from(requested).unwrap_or(u32::MAX),
            limit: u32::try_from(limit).unwrap_or(u32::MAX),
        },
        crate::admission::AdmissionError::ArtifactEnvelopeDecodeFailed => {
            crate::RuntimeError::AdmissionArtifactEnvelopeDecodeFailed
        }
        crate::admission::AdmissionError::ArtifactInvalidGateCount { found, required } => {
            crate::RuntimeError::AdmissionArtifactInvalidGateCount { found, required }
        }
        crate::admission::AdmissionError::ArtifactInvalidProofFlag { flag } => {
            crate::RuntimeError::AdmissionArtifactInvalidProofFlag { flag }
        }
        crate::admission::AdmissionError::ArtifactDigestMismatch { requested, found } => {
            crate::RuntimeError::AdmissionArtifactDigestMismatch { requested, found }
        }
        crate::admission::AdmissionError::ArtifactCertificateStale {
            digest,
            accepted_at_seq: _,
            required_at_least: _,
        } => crate::RuntimeError::AdmissionArtifactStale { digest },
        // `AdmissionError` is `#[non_exhaustive]`. A new variant must land
        // in the artifact-class fallback so an operator scraping runtime
        // errors can still distinguish "the artifact is bad" from "the
        // shard is full". Update the per-variant arms above when adding
        // new variants.
        _ => crate::RuntimeError::AdmissionArtifactInvalid {
            digest: workflow_digest,
        },
    }
}

/// Maps an [`AggregateBudgetError`](vb_core::budget::AggregateBudgetError)
/// directly into a [`RuntimeError`](crate::RuntimeError) by first converting
/// to an [`AdmissionError`](crate::admission::AdmissionError).
pub(super) fn map_aggregate_budget_error(
    error: AggregateBudgetError,
    workflow_digest: WorkflowDigest,
) -> crate::RuntimeError {
    let admission_error = aggregate_budget_to_admission_error(error);
    map_admission_error(admission_error, workflow_digest)
}

#[cfg(test)]
mod tests {
    //! RA-001 regression tests for `map_admission_error`.
    //!
    //! Every distinct `AdmissionError` variant must be translated to a
    //! distinct `RuntimeError` variant so operators scraping runtime errors
    //! can distinguish artifact-corruption failures from resource/budget
    //! exhaustion failures. The legacy implementation collapsed every
    //! unmatched variant into `RuntimeError::AdmissionArtifactInvalid`,
    //! hiding the real cause.

    use super::*;
    use vb_core::capability::{Capability, CapabilitySet};
    use vb_core::ids::{ActionId, WorkflowDigest};

    fn digest() -> WorkflowDigest {
        WorkflowDigest::from_bytes([0x11; 32])
    }

    #[test]
    fn ra001_resource_capacity_exhaustion_maps_to_distinct_runtime_variant() {
        let error = crate::admission::AdmissionError::ResourceCapacityExceeded {
            resource: "active_runs",
            requested: 9,
            available: 3,
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionResourceCapacityExceeded {
                    resource: "active_runs",
                    requested: 9,
                    available: 3,
                }
            ),
            "ResourceCapacityExceeded must map to AdmissionResourceCapacityExceeded, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_budget_policy_exceeded_maps_to_distinct_runtime_variant() {
        let error = crate::admission::AdmissionError::BudgetPolicyExceeded {
            resource: "step_count",
            actual: 5000,
            limit: 1000,
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionBudgetPolicyExceeded {
                    resource: "step_count",
                    actual: 5000,
                    limit: 1000,
                }
            ),
            "BudgetPolicyExceeded must map to AdmissionBudgetPolicyExceeded, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_resource_budget_overflow_maps_to_distinct_runtime_variant() {
        let error = crate::admission::AdmissionError::ResourceBudgetOverflow {
            resource: "ipc_bytes",
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionResourceBudgetOverflow {
                    resource: "ipc_bytes",
                }
            ),
            "ResourceBudgetOverflow must map to AdmissionResourceBudgetOverflow, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_resource_budget_underflow_maps_to_distinct_runtime_variant() {
        let error = crate::admission::AdmissionError::ResourceBudgetUnderflow {
            resource: "ipc_bytes",
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionResourceBudgetUnderflow {
                    resource: "ipc_bytes",
                }
            ),
            "ResourceBudgetUnderflow must map to AdmissionResourceBudgetUnderflow, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_resource_budget_invalid_capacity_maps_to_distinct_runtime_variant() {
        let error = crate::admission::AdmissionError::ResourceBudgetInvalidCapacity {
            resource: "step_count",
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionResourceBudgetInvalidCapacity {
                    resource: "step_count",
                }
            ),
            "ResourceBudgetInvalidCapacity must map to AdmissionResourceBudgetInvalidCapacity, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_artifact_envelope_decode_failed_maps_to_distinct_runtime_variant() {
        let error = crate::admission::AdmissionError::ArtifactEnvelopeDecodeFailed;
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(mapped, crate::RuntimeError::AdmissionArtifactEnvelopeDecodeFailed),
            "ArtifactEnvelopeDecodeFailed must map to AdmissionArtifactEnvelopeDecodeFailed, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_artifact_invalid_gate_count_maps_to_distinct_runtime_variant() {
        let error = crate::admission::AdmissionError::ArtifactInvalidGateCount {
            found: 12,
            required: 15,
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionArtifactInvalidGateCount {
                    found: 12,
                    required: 15,
                }
            ),
            "ArtifactInvalidGateCount must map to AdmissionArtifactInvalidGateCount, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_artifact_invalid_proof_flag_maps_to_distinct_runtime_variant() {
        let error = crate::admission::AdmissionError::ArtifactInvalidProofFlag {
            flag: "bounded",
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionArtifactInvalidProofFlag { flag: "bounded" }
            ),
            "ArtifactInvalidProofFlag must map to AdmissionArtifactInvalidProofFlag, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_artifact_digest_mismatch_maps_to_existing_artifact_digest_mismatch() {
        let requested = WorkflowDigest::from_bytes([0x01; 32]);
        let found = WorkflowDigest::from_bytes([0x02; 32]);
        let error = crate::admission::AdmissionError::ArtifactDigestMismatch {
            requested,
            found,
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionArtifactDigestMismatch { .. }
            ),
            "ArtifactDigestMismatch must map to AdmissionArtifactDigestMismatch, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_artifact_certificate_stale_maps_to_existing_artifact_stale() {
        let stale = WorkflowDigest::from_bytes([0x03; 32]);
        let error = crate::admission::AdmissionError::ArtifactCertificateStale {
            digest: stale,
            accepted_at_seq: vb_storage::EventSeq::new(5),
            required_at_least: vb_storage::EventSeq::new(10),
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(mapped, crate::RuntimeError::AdmissionArtifactStale { .. }),
            "ArtifactCertificateStale must map to AdmissionArtifactStale, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_resource_step_ceiling_exceeded_maps_to_admission_budget_exceeded() {
        // ResourceStepCeilingExceeded carries the requested/limit pair as u64,
        // while AdmissionBudgetExceeded uses u32. The mapping must saturate
        // rather than truncate or panic when the value overflows u32.
        let error = crate::admission::AdmissionError::ResourceStepCeilingExceeded {
            requested: 7,
            limit: 5,
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionBudgetExceeded {
                    actual: 7,
                    limit: 5,
                }
            ),
            "ResourceStepCeilingExceeded must map to AdmissionBudgetExceeded, got {mapped:?}"
        );

        let overflow = crate::admission::AdmissionError::ResourceStepCeilingExceeded {
            requested: u64::from(u32::MAX).checked_add(1).unwrap_or(u32::MAX as u64),
            limit: u64::MAX,
        };
        let mapped = map_admission_error(overflow, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionBudgetExceeded {
                    actual: u32::MAX,
                    limit: u32::MAX,
                }
            ),
            "ResourceStepCeilingExceeded must saturate to u32::MAX on u64 overflow, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_resource_per_tick_ceiling_exceeded_maps_to_admission_budget_exceeded() {
        let error = crate::admission::AdmissionError::ResourcePerTickCeilingExceeded {
            requested: 99,
            limit: 10,
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionBudgetExceeded {
                    actual: 99,
                    limit: 10,
                }
            ),
            "ResourcePerTickCeilingExceeded must map to AdmissionBudgetExceeded, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_artifact_not_found_still_maps_to_artifact_not_found() {
        let d = WorkflowDigest::from_bytes([0x04; 32]);
        let error = crate::admission::AdmissionError::ArtifactNotFound { digest: d };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(mapped, crate::RuntimeError::AdmissionArtifactNotFound { .. }),
            "ArtifactNotFound must map to AdmissionArtifactNotFound, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_capability_denied_still_maps_to_capability_denied() {
        let error = crate::admission::AdmissionError::CapabilityDenied {
            action: ActionId::new(7),
            required: Capability::new("io".into(), ActionId::new(7)),
            granted: CapabilitySet::empty(),
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(mapped, crate::RuntimeError::AdmissionCapabilityDenied { .. }),
            "CapabilityDenied must map to AdmissionCapabilityDenied, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_budget_exceeded_step_count_still_maps_to_admission_budget_exceeded() {
        let error = crate::admission::AdmissionError::BudgetExceeded {
            actual: 1500,
            limit: 1000,
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            matches!(
                mapped,
                crate::RuntimeError::AdmissionBudgetExceeded {
                    actual: 1500,
                    limit: 1000,
                }
            ),
            "BudgetExceeded must map to AdmissionBudgetExceeded, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_resource_capacity_is_not_collapsed_into_artifact_invalid() {
        // RA-001 key contract: capacity exhaustion must NOT be reported as
        // "artifact invalid". The shard is full; the artifact is fine.
        let error = crate::admission::AdmissionError::ResourceCapacityExceeded {
            resource: "active_runs",
            requested: 1,
            available: 0,
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            !matches!(mapped, crate::RuntimeError::AdmissionArtifactInvalid { .. }),
            "ResourceCapacityExceeded must NOT collapse to AdmissionArtifactInvalid, got {mapped:?}"
        );
    }

    #[test]
    fn ra001_budget_policy_is_not_collapsed_into_artifact_invalid() {
        // RA-001 key contract: budget policy violation must NOT be reported
        // as "artifact invalid".
        let error = crate::admission::AdmissionError::BudgetPolicyExceeded {
            resource: "step_count",
            actual: 10_000,
            limit: 1_000,
        };
        let mapped = map_admission_error(error, digest());
        assert!(
            !matches!(mapped, crate::RuntimeError::AdmissionArtifactInvalid { .. }),
            "BudgetPolicyExceeded must NOT collapse to AdmissionArtifactInvalid, got {mapped:?}"
        );
    }
}
