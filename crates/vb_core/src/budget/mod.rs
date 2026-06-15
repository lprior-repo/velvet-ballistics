#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

//! Whole-workflow budget computation and boundedness policy enforcement.

mod aggregate_budget;
mod aggregate_usage;
mod aggregate_usage_checks;
mod budget_error;
mod policy;
mod small_linear;
mod traversal;
mod traversal_fanout;
mod traversal_loop;
mod traversal_path;
mod traversal_step_count;
mod traversal_successors;
mod traversal_tracking;
mod types;
mod validation;

pub use aggregate_budget::{
    AggregateBudgetError, AggregateReservation, AggregateResourceBudget, AggregateResourceCapacity,
};
pub use aggregate_usage::AggregateResourceUsage;
pub use budget_error::BudgetError;
pub use policy::BoundednessPolicy;
pub use types::WholeWorkflowBudget;
pub use validation::{validate_aggregate_budget, validate_step_ceilings};

#[cfg(test)]
pub(crate) use validation::{add_dim, sub_dim};

#[cfg(kani)]
mod tests_and_verification;

#[cfg(test)]
mod tests;

#[cfg(all(test, kani))]
mod vb_qi37_2_4_state8_tests;
