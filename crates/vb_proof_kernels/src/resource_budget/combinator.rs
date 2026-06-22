//! Standalone budget-composition combinators.
//!
//! These free functions mirror the mutable-method API on `Budget` but
//! return a fresh `Budget` so they can be composed in expressions
//! without in-place mutation.

use super::budget::Budget;

pub fn sequential_compose(a: &Budget, b: &Budget) -> Budget {
    let mut result = a.clone();
    result.sequential_add(b);
    result
}

pub fn branch_compose(a: &Budget, b: &Budget) -> Budget {
    let mut result = a.clone();
    result.branch_max(b);
    result
}

pub fn loop_compose(body: &Budget, iterations: u64) -> Budget {
    let mut result = body.clone();
    result.loop_mul(iterations);
    result
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
