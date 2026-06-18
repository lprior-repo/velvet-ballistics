//! Budget policy and policy-violation checking.
//!
//! Policies define hard upper bounds on budget fields.  The `within` method
//! returns every field name that exceeds the policy limit, enabling callers
//! to report or reject an over-budget computation.

use super::budget::Budget;

#[derive(Debug, Clone)]
pub struct Policy {
    pub max_actions: u64,
    pub max_parallel: u64,
    pub max_run_time: u64,
    pub max_result_bytes: u64,
    pub max_steps: u64,
}

impl Policy {
    pub fn default_policy() -> Policy {
        Policy {
            max_actions: 100_000,
            max_parallel: 256,
            max_run_time: 30 * 24 * 60 * 60,
            max_result_bytes: 256 * 1024,
            max_steps: 1_000_000,
        }
    }

    /// Return every field name whose budget value exceeds the policy limit.
    ///
    /// An empty return value means the budget is fully within policy.
    pub fn within(&self, budget: &Budget) -> Vec<&'static str> {
        let mut violations = Vec::new();
        if budget.actions > self.max_actions {
            violations.push("actions");
        }
        if budget.parallel > self.max_parallel {
            violations.push("parallel");
        }
        if budget.run_time_secs > self.max_run_time {
            violations.push("run_time");
        }
        if budget.result_bytes > self.max_result_bytes {
            violations.push("result_bytes");
        }
        if budget.steps > self.max_steps {
            violations.push("steps");
        }
        violations
    }
}
