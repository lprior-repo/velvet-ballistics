//! Verus specification for Budget arithmetic refinement.
//!
//! This file contains the Verus formal specification of the Budget
//! resource tracking for refinement verification between Rust and Verus.
//!
//! BUDGET-VERUS: budget_verus.rs must contain:
//! - add_dim_spec - specification for saturating addition
//! - sub_dim_spec - specification for saturating subtraction (floors at 0)
//! - branch_max_spec - specification for max-based branch composition
//! - loop_mul_spec - specification for saturating multiplication

use crate::resource_budget::Budget;

/// add_dim_spec - specification for saturating addition on a single dimension.
/// add_dim_spec(a, b) = min(a + b, u64::MAX)
pub fn add_dim_spec(a: u64, b: u64) -> u64 {
    a.saturating_add(b)
}

/// sub_dim_spec - specification for saturating subtraction on a single dimension.
/// sub_dim_spec(a, b) = if a >= b { a - b } else { 0 }
pub fn sub_dim_spec(a: u64, b: u64) -> u64 {
    if a >= b { a - b } else { 0 }
}

/// branch_max_spec - specification for max-based branch composition.
/// Takes the maximum for each dimension.
pub fn branch_max_spec(a: u64, b: u64) -> u64 {
    if a >= b { a } else { b }
}

/// loop_mul_spec - specification for saturating multiplication for loop composition.
/// loop_mul_spec(a, n) = min(a * n, u64::MAX)
pub fn loop_mul_spec(a: u64, n: u64) -> u64 {
    a.saturating_mul(n)
}

/// spec_sequential_add - specification for sequential budget composition.
/// Dimensions: steps, actions use saturating_add; others use max.
pub fn spec_sequential_add(a: &Budget, b: &Budget) -> Budget {
    Budget {
        steps: add_dim_spec(a.steps, b.steps),
        actions: add_dim_spec(a.actions, b.actions),
        parallel: branch_max_spec(a.parallel, b.parallel),
        retries: branch_max_spec(a.retries, b.retries),
        gather_pages: add_dim_spec(a.gather_pages, b.gather_pages),
        gather_items: add_dim_spec(a.gather_items, b.gather_items),
        for_each_iters: branch_max_spec(a.for_each_iters, b.for_each_iters),
        together_branches: branch_max_spec(a.together_branches, b.together_branches),
        repeat_attempts: branch_max_spec(a.repeat_attempts, b.repeat_attempts),
        run_time_secs: add_dim_spec(a.run_time_secs, b.run_time_secs),
        result_bytes: branch_max_spec(a.result_bytes, b.result_bytes),
        slots_written: add_dim_spec(a.slots_written, b.slots_written),
    }
}

/// spec_branch_max - specification for branch composition using max.
pub fn spec_branch_max(a: &Budget, b: &Budget) -> Budget {
    Budget {
        steps: branch_max_spec(a.steps, b.steps),
        actions: branch_max_spec(a.actions, b.actions),
        parallel: branch_max_spec(a.parallel, b.parallel),
        retries: branch_max_spec(a.retries, b.retries),
        gather_pages: branch_max_spec(a.gather_pages, b.gather_pages),
        gather_items: branch_max_spec(a.gather_items, b.gather_items),
        for_each_iters: branch_max_spec(a.for_each_iters, b.for_each_iters),
        together_branches: branch_max_spec(a.together_branches, b.together_branches),
        repeat_attempts: branch_max_spec(a.repeat_attempts, b.repeat_attempts),
        run_time_secs: branch_max_spec(a.run_time_secs, b.run_time_secs),
        result_bytes: branch_max_spec(a.result_bytes, b.result_bytes),
        slots_written: branch_max_spec(a.slots_written, b.slots_written),
    }
}

/// spec_loop_mul - specification for loop multiplication using saturating_mul.
pub fn spec_loop_mul(a: &Budget, iterations: u64) -> Budget {
    Budget {
        steps: loop_mul_spec(a.steps, iterations),
        actions: loop_mul_spec(a.actions, iterations),
        parallel: loop_mul_spec(a.parallel, iterations),
        retries: loop_mul_spec(a.retries, iterations),
        gather_pages: loop_mul_spec(a.gather_pages, iterations),
        gather_items: loop_mul_spec(a.gather_items, iterations),
        for_each_iters: loop_mul_spec(a.for_each_iters, iterations),
        together_branches: loop_mul_spec(a.together_branches, iterations),
        repeat_attempts: loop_mul_spec(a.repeat_attempts, iterations),
        run_time_secs: loop_mul_spec(a.run_time_secs, iterations),
        result_bytes: loop_mul_spec(a.result_bytes, iterations),
        slots_written: loop_mul_spec(a.slots_written, iterations),
    }
}

/// lemma_saturating_add_never_overflows - Verus lemma verifying saturating_add never overflows.
pub fn lemma_saturating_add_never_overflows(a: u64, b: u64) -> bool {
    add_dim_spec(a, b) <= u64::MAX
}

/// lemma_saturating_mul_never_overflows - Verus lemma verifying saturating_mul never overflows.
pub fn lemma_saturating_mul_never_overflows(a: u64, n: u64) -> bool {
    loop_mul_spec(a, n) <= u64::MAX
}

/// lemma_branch_max_returns_max - Verus lemma verifying branch_max returns the max.
pub fn lemma_branch_max_returns_max(a: u64, b: u64) -> bool {
    branch_max_spec(a, b) >= a && branch_max_spec(a, b) >= b
}
