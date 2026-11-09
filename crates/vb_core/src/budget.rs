#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

//! Whole-workflow budget computation and boundedness policy enforcement.

use crate::ids::StepIdx;
use crate::workflow::WorkflowError;
use loop_region::visit_body_region_node;

pub use aggregate::{
    AggregateBudgetError, AggregateReservation, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, ShardUsageSnapshot,
};
pub use aggregate_validate::{check_global_quota, check_tenant_quota, validate_aggregate_budget};
pub use compute::WholeWorkflowBudget;
pub use validate::BoundednessPolicy;
pub use validate_error::{BudgetDimension, BudgetError};

pub use remaining::{BudgetExhaustionOutcome, RemainingBudget, RemainingBudgetError};
pub(crate) use traversal_error::BudgetTraversalError;

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

pub(super) fn add_dim(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, AggregateBudgetError> {
    current
        .checked_add(requested)
        .ok_or(AggregateBudgetError::Overflow { resource })
}

pub(super) fn sub_dim(
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

/// Counts the worst-case total number of runtime steps by performing a DFS walk
/// from the entry node. Unlike a naive unique-node count, this function accounts
/// for loop iteration limits: when a loop header (ForEachStart, CollectStart,
/// RepeatStart, ReduceStart) is encountered, the body subgraph step count is
/// multiplied by the iteration limit and added once for the header itself.
///
/// The algorithm works in two phases:
/// 1. **Body counting phase**: A DFS walk counts unique nodes in each loop body
///    region (nodes reachable from `body` but not from `done`). This avoids
///    infinite recursion from back-edges.
/// 2. **Worst-case multiplication**: Loop body counts are multiplied by the
///    declared iteration limits and summed with non-loop node counts.
mod aggregate;
mod aggregate_validate;
mod compute;
mod conditional_counter;
mod small_linear;
mod traversal_error;
mod validate;
mod validate_error;

fn find_node_position(
    nodes: &[crate::workflow::CompiledNode],
    step: StepIdx,
    node_count: usize,
) -> Result<usize, BudgetTraversalError> {
    let direct_idx = step.as_usize();
    if direct_idx < node_count
        && let Some(node) = nodes.get(direct_idx)
        && node.id == step
    {
        return Ok(direct_idx);
    }

    for (position, node) in nodes.iter().enumerate() {
        if node.id == step {
            return Ok(position);
        }
    }

    if direct_idx < node_count {
        return Ok(direct_idx);
    }

    Err(BudgetTraversalError::StepOutOfBounds { step })
}

fn node_at_position(
    nodes: &[crate::workflow::CompiledNode],
    position: usize,
    step: StepIdx,
) -> Result<&crate::workflow::CompiledNode, BudgetTraversalError> {
    match nodes.get(position) {
        Some(node) => Ok(node),
        None => Err(BudgetTraversalError::StepOutOfBounds { step }),
    }
}

mod loop_body;
mod loop_region;
mod region_counter;

mod depth;
mod reachable;
mod remaining;
mod successors;
mod usage;

mod tests_and_verification;

#[cfg(test)]
#[path = "budget/tests.rs"]
mod tests;

#[cfg(test)]
mod vb_qi37_2_4_state8_tests;

#[cfg(test)]
mod depth_overflow_tests {
    //! Regression tests for CB-004 / `vb-lk26g`:
    //! `compute_child_depth` must return `BudgetTraversalError::DepthOverflow`
    //! carrying the actual pre-overflow depth value, not the lossy
    //! `StepCountOverflow { actual: u64::MAX }` sentinel it used to produce.

    use super::BudgetTraversalError;
    use super::depth::compute_child_depth;
    use crate::workflow::CompiledNodeKind;

    #[test]
    fn compute_child_depth_returns_depth_overflow_carrying_actual_depth() {
        let kind = CompiledNodeKind::ForEachStart {
            input: crate::ids::SlotIdx::new(0),
            item_slot: crate::ids::SlotIdx::new(1),
            limit: 1,
            body: crate::ids::StepIdx::new(0),
            done: crate::ids::StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;
        let err = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth)
            .expect_err("u16::MAX + 1 must overflow");
        match err {
            BudgetTraversalError::DepthOverflow { depth } => {
                assert_eq!(
                    depth,
                    u16::MAX,
                    "DepthOverflow must carry the actual pre-overflow depth"
                );
            }
            other => {
                let msg = format!(
                    "expected BudgetTraversalError::DepthOverflow {{ depth: u16::MAX }}, got {other:?}"
                );
                debug_assert!(false, "{msg}");
            }
        }
    }

    #[test]
    fn compute_child_depth_does_not_emit_step_count_overflow_at_u16_max() {
        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 1,
            body: crate::ids::StepIdx::new(0),
            done: crate::ids::StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;
        let err = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth)
            .expect_err("u16::MAX + 1 must overflow");
        assert!(
            !matches!(err, BudgetTraversalError::StepCountOverflow { .. }),
            "depth overflow must not be misclassified as StepCountOverflow (CB-004)"
        );
    }

    #[test]
    fn depth_overflow_converts_to_workflow_error_carrying_actual_depth() {
        let err: crate::workflow::WorkflowError =
            BudgetTraversalError::DepthOverflow { depth: 42 }.into();
        match err {
            crate::workflow::WorkflowError::DepthOverflow { depth } => {
                assert_eq!(depth, 42, "From conversion must preserve depth");
            }
            other => {
                let msg =
                    format!("expected WorkflowError::DepthOverflow {{ depth: 42 }}, got {other:?}");
                debug_assert!(false, "{msg}");
            }
        }
    }
}
