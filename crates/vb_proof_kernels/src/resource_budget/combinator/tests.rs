use super::*;
use crate::resource_budget::Policy;

#[test]
fn test_sequential_add() {
    let mut a = Budget::new();
    a.steps = 10;
    a.actions = 5;
    let mut b = Budget::new();
    b.steps = 7;
    b.actions = 3;
    a.sequential_add(&b);
    assert_eq!(a.steps, 17);
    assert_eq!(a.actions, 8);
}

#[test]
fn test_branch_max() {
    let mut a = Budget::new();
    a.steps = 10;
    a.actions = 5;
    let mut b = Budget::new();
    b.steps = 7;
    b.actions = 8;
    a.branch_max(&b);
    assert_eq!(a.steps, 10);
    assert_eq!(a.actions, 8);
}

#[test]
fn test_loop_multiply() {
    let mut body = Budget::new();
    body.steps = 10;
    body.actions = 2;
    body.loop_mul(5);
    assert_eq!(body.steps, 50);
    assert_eq!(body.actions, 10);
}

#[test]
fn test_saturating_add() {
    let mut a = Budget::new();
    a.steps = u64::MAX;
    a.actions = 1;
    let mut b = Budget::new();
    b.steps = 1;
    b.actions = 1;
    a.sequential_add(&b);
    assert_eq!(a.steps, u64::MAX);
    assert_eq!(a.actions, 2);
}

#[test]
fn test_saturating_mul() {
    let mut body = Budget::new();
    body.steps = u64::MAX;
    body.actions = 2;
    body.loop_mul(2);
    assert_eq!(body.steps, u64::MAX);
    assert_eq!(body.actions, 4);
}

#[test]
fn test_policy_violation() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.actions = 200_000;
    let violations = policy.within(&budget);
    assert!(!violations.is_empty());
    assert!(violations.contains(&"actions"));
}

#[test]
fn test_policy_pass() {
    let policy = Policy::default_policy();
    let budget = Budget::new();
    let violations = policy.within(&budget);
    assert!(violations.is_empty());
}

#[test]
fn test_budget_new_is_all_zeros() {
    let budget = Budget::new();
    assert_eq!(budget.steps, 0);
    assert_eq!(budget.actions, 0);
    assert_eq!(budget.parallel, 0);
    assert_eq!(budget.retries, 0);
    assert_eq!(budget.gather_pages, 0);
    assert_eq!(budget.gather_items, 0);
    assert_eq!(budget.for_each_iters, 0);
    assert_eq!(budget.together_branches, 0);
    assert_eq!(budget.repeat_attempts, 0);
    assert_eq!(budget.run_time_secs, 0);
    assert_eq!(budget.result_bytes, 0);
    assert_eq!(budget.slots_written, 0);
}

#[test]
fn test_budget_default_equals_new() {
    let default_budget = Budget::default();
    let new_budget = Budget::new();
    assert_eq!(default_budget.steps, new_budget.steps);
    assert_eq!(default_budget.actions, new_budget.actions);
}

#[test]
fn test_policy_default_values() {
    let policy = Policy::default_policy();
    assert_eq!(policy.max_actions, 100_000);
    assert_eq!(policy.max_parallel, 256);
    assert_eq!(policy.max_run_time, 30 * 24 * 60 * 60);
    assert_eq!(policy.max_result_bytes, 256 * 1024);
    assert_eq!(policy.max_steps, 1_000_000);
}

#[test]
fn test_sequential_compose_zero_budgets() {
    let a = Budget::new();
    let b = Budget::new();
    let result = sequential_compose(&a, &b);
    assert_eq!(result.steps, 0);
    assert_eq!(result.actions, 0);
}

#[test]
fn test_sequential_compose_adds() {
    let mut a = Budget::new();
    a.steps = 10;
    a.actions = 5;
    let mut b = Budget::new();
    b.steps = 7;
    b.actions = 3;
    let result = sequential_compose(&a, &b);
    assert_eq!(result.steps, 17);
    assert_eq!(result.actions, 8);
}

#[test]
fn test_branch_compose_takes_max() {
    let mut a = Budget::new();
    a.steps = 10;
    let mut b = Budget::new();
    b.steps = 7;
    b.actions = 8;
    let result = branch_compose(&a, &b);
    assert_eq!(result.steps, 10);
    assert_eq!(result.actions, 8);
}

#[test]
fn test_loop_compose_multiplies() {
    let mut body = Budget::new();
    body.steps = 3;
    body.actions = 4;
    let result = loop_compose(&body, 10);
    assert_eq!(result.steps, 30);
    assert_eq!(result.actions, 40);
}

#[test]
fn test_loop_compose_zero_iterations() {
    let body = Budget::new();
    let result = loop_compose(&body, 0);
    assert_eq!(result.steps, 0);
    assert_eq!(result.actions, 0);
}

#[test]
fn test_sequential_compose_saturates() {
    let mut a = Budget::new();
    a.steps = u64::MAX;
    let mut b = Budget::new();
    b.steps = 1;
    let result = sequential_compose(&a, &b);
    assert_eq!(result.steps, u64::MAX);
}
