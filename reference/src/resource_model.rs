//! Reference resource budget model.
//!
//! This is the canonical reference implementation for resource budget arithmetic.
//! Use this to verify the optimized implementation matches this behavior.

mod accumulator;

pub use accumulator::BudgetAccumulator;

#[derive(Debug, Clone)]
pub struct PolicyBounds {
    pub absolute_max_action_tickets: u32,
    pub absolute_max_parallel: u16,
    pub absolute_max_run_time_seconds: u64,
    pub absolute_max_result_bytes: u32,
    pub absolute_max_steps_executable: u32,
}

impl Default for PolicyBounds {
    fn default() -> Self {
        PolicyBounds {
            absolute_max_action_tickets: 100_000,
            absolute_max_parallel: 256,
            absolute_max_run_time_seconds: 30 * 24 * 60 * 60,
            absolute_max_result_bytes: 256 * 1024,
            absolute_max_steps_executable: 1_000_000,
        }
    }
}

#[derive(Default)]
pub struct ResourceModel;

impl ResourceModel {
    pub fn new() -> Self {
        ResourceModel
    }

    pub fn compute_sequential(
        &self,
        a: &BudgetAccumulator,
        b: &BudgetAccumulator,
    ) -> BudgetAccumulator {
        let mut result = a.clone();
        result.add_sequential(b);
        result
    }

    pub fn compute_branch(
        &self,
        a: &BudgetAccumulator,
        b: &BudgetAccumulator,
    ) -> BudgetAccumulator {
        let mut result = a.clone();
        result.add_branch(b);
        result
    }

    pub fn compute_loop(&self, body: &BudgetAccumulator, iterations: u64) -> BudgetAccumulator {
        let mut result = body.clone();
        result.multiply_loop(iterations);
        result
    }

    pub fn validate_against_policy(
        &self,
        budget: &BudgetAccumulator,
        policy: &PolicyBounds,
    ) -> Result<(), Vec<String>> {
        let violations = budget.within_policy(policy);
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_sum() {
        let mut a = BudgetAccumulator::new();
        a.max_steps_executable = 10;
        a.max_action_tickets = 5;

        let mut b = BudgetAccumulator::new();
        b.max_steps_executable = 7;
        b.max_action_tickets = 3;

        let model = ResourceModel::new();
        let result = model.compute_sequential(&a, &b);

        assert_eq!(result.max_steps_executable, 17);
        assert_eq!(result.max_action_tickets, 8);
    }

    #[test]
    fn test_branch_max() {
        let mut a = BudgetAccumulator::new();
        a.max_steps_executable = 10;
        a.max_action_tickets = 5;

        let mut b = BudgetAccumulator::new();
        b.max_steps_executable = 7;
        b.max_action_tickets = 8;

        let model = ResourceModel::new();
        let result = model.compute_branch(&a, &b);

        assert_eq!(result.max_steps_executable, 10);
        assert_eq!(result.max_action_tickets, 8);
    }

    #[test]
    fn test_loop_multiply() {
        let mut body = BudgetAccumulator::new();
        body.max_steps_executable = 10;
        body.max_action_tickets = 2;

        let model = ResourceModel::new();
        let result = model.compute_loop(&body, 5);

        assert_eq!(result.max_steps_executable, 50);
        assert_eq!(result.max_action_tickets, 10);
    }

    #[test]
    fn test_saturating_add() {
        let mut a = BudgetAccumulator::new();
        a.max_steps_executable = u64::MAX;
        a.max_action_tickets = 1;

        let mut b = BudgetAccumulator::new();
        b.max_steps_executable = 1;
        b.max_action_tickets = 1;

        let model = ResourceModel::new();
        let result = model.compute_sequential(&a, &b);

        assert_eq!(result.max_steps_executable, u64::MAX);
        assert_eq!(result.max_action_tickets, 2);
    }

    #[test]
    fn test_policy_violation() {
        let mut budget = BudgetAccumulator::new();
        budget.max_action_tickets = 200_000;

        let policy = PolicyBounds::default();
        let model = ResourceModel::new();
        let result = model.validate_against_policy(&budget, &policy);

        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations.iter().any(|v| v.contains("max_action_tickets")));
    }
}
