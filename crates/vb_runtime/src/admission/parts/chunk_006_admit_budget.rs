fn map_budget_error(error: AggregateBudgetError) -> AdmissionError {
    match error {
        AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        } => map_policy_budget_exceeded(resource, actual, limit),
        AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        } => map_capacity_budget_exceeded(resource, requested, available),
        AggregateBudgetError::Overflow { .. }
        | AggregateBudgetError::Underflow { .. }
        | AggregateBudgetError::InvalidCapacity { .. } => map_budget_arithmetic_error(error),
        AggregateBudgetError::StepCeilingExceeded { .. }
        | AggregateBudgetError::PerTickCeilingExceeded { .. } => map_budget_ceiling_error(error),
        other => map_budget_other_error(other),
    }
}

fn map_policy_budget_exceeded(resource: &'static str, actual: u64, limit: u64) -> AdmissionError {
    AdmissionError::BudgetPolicyExceeded {
        resource,
        actual,
        limit,
    }
}

fn map_capacity_budget_exceeded(
    resource: &'static str,
    requested: u64,
    available: u64,
) -> AdmissionError {
    AdmissionError::ResourceCapacityExceeded {
        resource,
        requested,
        available,
    }
}

fn map_budget_arithmetic_error(error: AggregateBudgetError) -> AdmissionError {
    match error {
        AggregateBudgetError::Overflow { resource } => {
            AdmissionError::ResourceBudgetOverflow { resource }
        }
        AggregateBudgetError::Underflow { resource } => {
            AdmissionError::ResourceBudgetUnderflow { resource }
        }
        _ => match error {
            AggregateBudgetError::InvalidCapacity { resource } => {
                AdmissionError::ResourceBudgetInvalidCapacity { resource }
            }
            _ => AdmissionError::ResourceBudgetInvalidCapacity {
                resource: "unknown",
            },
        },
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
        _ => AdmissionError::BudgetPolicyExceeded {
            resource: "unknown_aggregate_budget_error",
            actual: u64::MAX,
            limit: 0,
        },
    }
}

fn map_budget_other_error(error: AggregateBudgetError) -> AdmissionError {
    match error {
        #[cfg(not(kani))]
        AggregateBudgetError::WorkflowBudget(_) => workflow_budget_error(),
        #[cfg(kani)]
        AggregateBudgetError::WorkflowBudget => workflow_budget_error(),
        AggregateBudgetError::ReservationNotFound { .. } => AdmissionError::BudgetPolicyExceeded {
            resource: "reservation_not_found",
            actual: u64::MAX,
            limit: 0,
        },
        _ => AdmissionError::BudgetPolicyExceeded {
            resource: "unknown_aggregate_budget_error",
            actual: u64::MAX,
            limit: 0,
        },
    }
}

fn workflow_budget_error() -> AdmissionError {
    AdmissionError::BudgetPolicyExceeded {
        resource: "workflow_budget",
        actual: u64::MAX,
        limit: 0,
    }
}
