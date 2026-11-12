use super::super::{PolicyBounds, ResourceModel};
use super::BudgetAccumulator;

fn budget(values: [u64; 5]) -> BudgetAccumulator {
    let [tickets, parallel, seconds, bytes, steps] = values;
    let mut budget = BudgetAccumulator::new();
    budget.max_action_tickets = tickets;
    budget.max_parallel_in_flight = parallel;
    budget.max_run_time_seconds = seconds;
    budget.max_result_bytes = bytes;
    budget.max_steps_executable = steps;
    budget
}

fn policy() -> PolicyBounds {
    PolicyBounds {
        absolute_max_action_tickets: 11,
        absolute_max_parallel: 7,
        absolute_max_run_time_seconds: 13,
        absolute_max_result_bytes: 17,
        absolute_max_steps_executable: 19,
    }
}

#[test]
fn policy_accepts_equal_and_below_boundaries() {
    assert_eq!(
        budget([11, 7, 13, 17, 19]).within_policy(&policy()),
        Vec::<String>::new()
    );
    assert_eq!(
        budget([10, 6, 12, 16, 18]).within_policy(&policy()),
        Vec::<String>::new()
    );
    assert_eq!(
        ResourceModel::new().validate_against_policy(&budget([11, 7, 13, 17, 19]), &policy()),
        Ok(())
    );
}

#[test]
fn policy_reports_every_violation_in_original_order() {
    let expected = vec![
        "max_action_tickets 12 exceeds policy 11".to_string(),
        "max_parallel_in_flight 8 exceeds policy 7".to_string(),
        "max_run_time_seconds 14 exceeds policy 13".to_string(),
        "max_result_bytes 18 exceeds policy 17".to_string(),
        "max_steps_executable 20 exceeds policy 19".to_string(),
    ];
    let actual = budget([12, 8, 14, 18, 20]);
    assert_eq!(actual.within_policy(&policy()), expected);
    assert_eq!(
        ResourceModel::new().validate_against_policy(&actual, &policy()),
        Err(expected)
    );
}

#[test]
fn policy_preserves_partial_violation_order_and_accepted_dimensions() {
    assert_eq!(
        budget([11, 7, 14, 17, 20]).within_policy(&policy()),
        vec![
            "max_run_time_seconds 14 exceeds policy 13".to_string(),
            "max_steps_executable 20 exceeds policy 19".to_string(),
        ]
    );
}

fn maximum_policy() -> PolicyBounds {
    PolicyBounds {
        absolute_max_action_tickets: u32::MAX,
        absolute_max_parallel: u16::MAX,
        absolute_max_run_time_seconds: u64::MAX,
        absolute_max_result_bytes: u32::MAX,
        absolute_max_steps_executable: u32::MAX,
    }
}

#[test]
fn widening_preserves_all_maximum_policy_values() {
    let actual = budget([
        u64::from(u32::MAX),
        u64::from(u16::MAX),
        u64::MAX,
        u64::from(u32::MAX),
        u64::from(u32::MAX),
    ]);
    assert_eq!(
        actual.within_policy(&maximum_policy()),
        Vec::<String>::new()
    );
}

#[test]
fn maximum_actuals_retain_exact_widened_limit_diagnostics() {
    assert_eq!(
        budget([u64::MAX; 5]).within_policy(&maximum_policy()),
        vec![
            "max_action_tickets 18446744073709551615 exceeds policy 4294967295".to_string(),
            "max_parallel_in_flight 18446744073709551615 exceeds policy 65535".to_string(),
            "max_result_bytes 18446744073709551615 exceeds policy 4294967295".to_string(),
            "max_steps_executable 18446744073709551615 exceeds policy 4294967295".to_string(),
        ]
    );
}

#[test]
fn zero_limits_accept_zero_and_reject_exactly_one() {
    let zero = PolicyBounds {
        absolute_max_action_tickets: 0,
        absolute_max_parallel: 0,
        absolute_max_run_time_seconds: 0,
        absolute_max_result_bytes: 0,
        absolute_max_steps_executable: 0,
    };
    assert_eq!(budget([0; 5]).within_policy(&zero), Vec::<String>::new());
    assert_eq!(
        budget([1; 5]).within_policy(&zero),
        vec![
            "max_action_tickets 1 exceeds policy 0".to_string(),
            "max_parallel_in_flight 1 exceeds policy 0".to_string(),
            "max_run_time_seconds 1 exceeds policy 0".to_string(),
            "max_result_bytes 1 exceeds policy 0".to_string(),
            "max_steps_executable 1 exceeds policy 0".to_string(),
        ]
    );
}

#[test]
fn previously_unchecked_policy_dimensions_remain_unchecked() {
    let mut actual = budget([0; 5]);
    actual.max_retries_per_action = u64::MAX;
    actual.max_gather_pages = u64::MAX;
    actual.max_gather_items = u64::MAX;
    actual.max_for_each_iterations = u64::MAX;
    actual.max_together_branches = u64::MAX;
    actual.max_repeat_attempts = u64::MAX;
    actual.max_total_slots_written = u64::MAX;
    assert_eq!(actual.within_policy(&policy()), Vec::<String>::new());
}
