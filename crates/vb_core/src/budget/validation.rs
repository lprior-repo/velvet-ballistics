#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use crate::workflow::WorkflowError;

use super::aggregate_budget::{AggregateBudgetError, AggregateResourceBudget};
use super::budget_error::BudgetError;
use super::policy::BoundednessPolicy;
use super::traversal::BudgetTraversalError;

pub fn validate_aggregate_budget(
    budget: &AggregateResourceBudget,
    policy: &BoundednessPolicy,
) -> Result<(), AggregateBudgetError> {
    check_policy(
        "max_steps_executable",
        u64::from(budget.max_steps_executable),
        u64::from(policy.absolute_max_steps_executable),
    )?;
    check_policy(
        "max_action_tickets",
        u64::from(budget.max_action_tickets),
        u64::from(policy.absolute_max_action_tickets),
    )?;
    check_policy(
        "max_parallel_in_flight",
        u64::from(budget.max_parallel_in_flight),
        u64::from(policy.absolute_max_parallel),
    )?;
    check_policy(
        "max_retries_per_action",
        u64::from(budget.max_retries_per_action),
        u64::from(u16::MAX),
    )?;
    check_policy(
        "max_gather_pages",
        u64::from(budget.max_gather_pages),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_gather_items",
        u64::from(budget.max_gather_items),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_for_each_iterations",
        u64::from(budget.max_for_each_iterations),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_together_branches",
        u64::from(budget.max_together_branches),
        u64::from(policy.max_fanout),
    )?;
    check_policy(
        "max_repeat_attempts",
        u64::from(budget.max_repeat_attempts),
        u64::from(u16::MAX),
    )?;
    check_policy(
        "max_run_time_seconds",
        budget.max_run_time_seconds,
        policy.absolute_max_run_time_seconds,
    )?;
    check_policy(
        "max_result_bytes",
        u64::from(budget.max_result_bytes),
        u64::from(policy.absolute_max_result_bytes),
    )?;
    check_policy(
        "max_total_slots_written",
        u64::from(budget.max_total_slots_written),
        policy.max_total_slots,
    )?;
    check_policy(
        "max_timer_entries",
        u64::from(budget.max_timer_entries),
        u64::from(policy.absolute_max_timer_entries),
    )?;
    check_policy(
        "max_trace_events",
        budget.max_trace_events,
        policy.absolute_max_trace_events,
    )?;
    check_policy(
        "max_queue_depth",
        u64::from(budget.max_queue_depth),
        u64::from(policy.absolute_max_queue_depth),
    )?;
    check_policy(
        "max_journal_batch_bytes",
        u64::from(budget.max_journal_batch_bytes),
        u64::from(policy.absolute_max_journal_batch_bytes),
    )?;
    check_policy(
        "max_ipc_payload_bytes",
        u64::from(budget.max_ipc_payload_bytes),
        u64::from(policy.absolute_max_ipc_payload_bytes),
    )?;
    check_policy(
        "max_blob_bytes",
        budget.max_blob_bytes,
        policy.absolute_max_blob_bytes,
    )?;
    check_policy(
        "max_input_bytes",
        u64::from(budget.max_input_bytes),
        u64::from(policy.absolute_max_input_bytes),
    )
}

/// Validates step ceiling dimensions (max_step_budget_per_tick and
/// max_transitions_per_tick) against hard limits.
pub fn validate_step_ceilings(
    budget: &AggregateResourceBudget,
) -> Result<(), AggregateBudgetError> {
    // Hard limit for step budget per tick - derived from MAX_STEPS_PER_TICK if defined,
    // otherwise use a conservative upper bound.
    const HARD_MAX_STEP_BUDGET_PER_TICK: u64 = 1_000_000;
    const HARD_MAX_TRANSITIONS_PER_TICK: u64 = 1_000_000;

    if budget.max_step_budget_per_tick == 0 {
        return Err(AggregateBudgetError::StepCeilingExceeded {
            requested: 0,
            limit: HARD_MAX_STEP_BUDGET_PER_TICK,
        });
    }
    if budget.max_step_budget_per_tick > HARD_MAX_STEP_BUDGET_PER_TICK {
        return Err(AggregateBudgetError::StepCeilingExceeded {
            requested: budget.max_step_budget_per_tick,
            limit: HARD_MAX_STEP_BUDGET_PER_TICK,
        });
    }

    if budget.max_transitions_per_tick == 0 {
        return Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: 0,
            limit: HARD_MAX_TRANSITIONS_PER_TICK,
        });
    }
    if budget.max_transitions_per_tick > HARD_MAX_TRANSITIONS_PER_TICK {
        return Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: budget.max_transitions_per_tick,
            limit: HARD_MAX_TRANSITIONS_PER_TICK,
        });
    }

    Ok(())
}

pub(crate) fn add_dim(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, AggregateBudgetError> {
    current
        .checked_add(requested)
        .ok_or(AggregateBudgetError::Overflow { resource })
}

pub(crate) fn sub_dim(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, AggregateBudgetError> {
    current
        .checked_sub(requested)
        .ok_or(AggregateBudgetError::Underflow { resource })
}

pub(super) fn check_capacity(
    resource: &'static str,
    requested: u64,
    available: u64,
) -> Result<(), AggregateBudgetError> {
    if requested > available {
        Err(AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        })
    } else {
        Ok(())
    }
}

pub(super) fn check_policy(
    resource: &'static str,
    actual: u64,
    limit: u64,
) -> Result<(), AggregateBudgetError> {
    if actual > limit {
        Err(AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

impl From<WorkflowError> for BudgetError {
    fn from(_err: WorkflowError) -> Self {
        BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }
    }
}

impl From<BudgetTraversalError> for BudgetError {
    fn from(_err: BudgetTraversalError) -> Self {
        BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests — check_capacity boundary values (GAP-CAP-001 through GAP-CAP-003)
// ---------------------------------------------------------------------------
// check_capacity is pub(super) and only accessible from within the budget
// module tree.  The tests here cover the exact-equals boundary that the
// requested test plan identified as uncovered.

#[cfg(test)]
mod check_capacity_tests {
    use super::check_capacity;
    use crate::budget::AggregateBudgetError;

    fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
    where
        T: core::fmt::Debug + PartialEq,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }

    /// GAP-CAP-001: requested == available is the valid boundary — returns Ok.
    #[test]
    fn check_capacity_exact_equal_returns_ok() -> Result<(), String> {
        let result = check_capacity("steps", 100, 100);
        result.map_err(|e| format!("exact boundary should be Ok, got {e:?}"))
    }

    /// GAP-CAP-002: requested > available returns CapacityExceeded with
    /// correct requested/available fields.
    #[test]
    fn check_capacity_over_limit_returns_capacity_exceeded() -> Result<(), String> {
        let result = check_capacity("steps", 101, 100);
        match result {
            Err(AggregateBudgetError::CapacityExceeded {
                resource,
                requested,
                available,
            }) => {
                ensure_equal(resource, "steps")?;
                ensure_equal(requested, 101)?;
                ensure_equal(available, 100)
            }
            Err(other) => Err(format!("expected CapacityExceeded, got {other:?}"))?,
            Ok(_) => Err("expected Err(CapacityExceeded), got Ok".to_string())?,
        }
    }

    /// GAP-CAP-003: requested == 0 and available == 0 is a valid boundary — returns Ok.
    #[test]
    fn check_capacity_zero_zero_returns_ok() -> Result<(), String> {
        let result = check_capacity("steps", 0, 0);
        result.map_err(|e| format!("0 == 0 should be Ok, got {e:?}"))
    }
}
