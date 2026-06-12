#![forbid(unsafe_code)]
//! Aggregate budget error mapping for admission.

use vb_core::budget::AggregateBudgetError;

use super::errors::AdmissionError;

pub(crate) fn map_budget_error(error: AggregateBudgetError) -> AdmissionError {
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
        other => map_budget_resource_error(other),
    }
}

fn map_budget_resource_error(error: AggregateBudgetError) -> AdmissionError {
    match error {
        AggregateBudgetError::Overflow { resource } => {
            AdmissionError::ResourceBudgetOverflow { resource }
        }
        AggregateBudgetError::Underflow { resource } => {
            AdmissionError::ResourceBudgetUnderflow { resource }
        }
        AggregateBudgetError::InvalidCapacity { resource } => {
            AdmissionError::ResourceBudgetInvalidCapacity { resource }
        }
        other => map_budget_ceiling_error(other),
    }
}

fn map_budget_ceiling_error(error: AggregateBudgetError) -> AdmissionError {
    match error {
        AggregateBudgetError::StepCeilingExceeded { requested, limit } => {
            AdmissionError::ResourceStepCeilingExceeded { requested, limit }
        }
        AggregateBudgetError::PerTickCeilingExceeded { requested, limit } => {
            AdmissionError::ResourcePerTickCeilingExceeded { requested, limit }
        }
        other => map_budget_fallback_error(other),
    }
}

fn map_budget_fallback_error(error: AggregateBudgetError) -> AdmissionError {
    match error {
        #[cfg(not(kani))]
        AggregateBudgetError::WorkflowBudget(_) => budget_policy_sentinel("workflow_budget"),
        #[cfg(kani)]
        AggregateBudgetError::WorkflowBudget => budget_policy_sentinel("workflow_budget"),
        AggregateBudgetError::ReservationNotFound { .. } => {
            budget_policy_sentinel("reservation_not_found")
        }
        _ => budget_policy_sentinel("unknown_aggregate_budget_error"),
    }
}

fn budget_policy_sentinel(resource: &'static str) -> AdmissionError {
    AdmissionError::BudgetPolicyExceeded {
        resource,
        actual: u64::MAX,
        limit: 0,
    }
}
