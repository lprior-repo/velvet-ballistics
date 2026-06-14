#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for resource bounds enforcement (vb-e4mt).
//!
//! These harnesses prove panic-freedom and correct error variant production
//! for expression stack bounds, ValueStore arena capacity, and budget
//! arithmetic boundaries.
//!
//! Uses kani::Arbitrary for all core types (GOD RULE 1: no hardcoded shapes).
//! Binds to actual Rust implementations (GOD RULE 2: no vacuum proofs).

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, BudgetError, WholeWorkflowBudget,
};
use crate::errors::CoreError;
use crate::ids::ConstIdx;
use crate::limits::{MAX_EXPRESSION_OPS, MAX_EXPRESSION_STACK};
use crate::value_store::ValueStore;
use crate::workflow::check_expr_stack_bound;

include!("resource_bounds_arbitrary.rs");
include!("resource_bounds_harnesses_all.rs");
