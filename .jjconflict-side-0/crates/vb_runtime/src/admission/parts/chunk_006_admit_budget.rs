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
