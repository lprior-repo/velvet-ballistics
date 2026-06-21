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
