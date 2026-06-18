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
