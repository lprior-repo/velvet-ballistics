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

/// Checks whether a capability is granted for an action.
///
/// Returns `Ok(())` if the action's capability is covered by the granted set,
/// or `Err(AdmissionError::CapabilityDenied)` otherwise.
pub fn check_capability(
    action: ActionId,
    required: &Capability,
    granted: &CapabilitySet,
) -> Result<(), AdmissionError> {
    if granted.grants(required) {
        Ok(())
    } else {
        Err(AdmissionError::CapabilityDenied {
            action,
            required: required.clone(),
            granted: granted.clone(),
        })
    }
}
