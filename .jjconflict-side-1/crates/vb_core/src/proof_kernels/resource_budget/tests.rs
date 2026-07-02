use super::*;

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

// ── Budget new / default ────────────────────────────────────────────────

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
    assert_eq!(default_budget.parallel, new_budget.parallel);
    assert_eq!(default_budget.retries, new_budget.retries);
    assert_eq!(default_budget.gather_pages, new_budget.gather_pages);
    assert_eq!(default_budget.gather_items, new_budget.gather_items);
    assert_eq!(default_budget.for_each_iters, new_budget.for_each_iters);
    assert_eq!(
        default_budget.together_branches,
        new_budget.together_branches
    );
    assert_eq!(default_budget.repeat_attempts, new_budget.repeat_attempts);
    assert_eq!(default_budget.run_time_secs, new_budget.run_time_secs);
    assert_eq!(default_budget.result_bytes, new_budget.result_bytes);
    assert_eq!(default_budget.slots_written, new_budget.slots_written);
}

// ── Policy default values ───────────────────────────────────────────────

#[test]
fn test_policy_default_max_actions() {
    let policy = Policy::default_policy();
    assert_eq!(policy.max_actions, 100_000);
}

#[test]
fn test_policy_default_max_parallel() {
    let policy = Policy::default_policy();
    assert_eq!(policy.max_parallel, 256);
}

#[test]
fn test_policy_default_max_run_time() {
    let policy = Policy::default_policy();
    assert_eq!(policy.max_run_time, 30 * 24 * 60 * 60);
}

#[test]
fn test_policy_default_max_result_bytes() {
    let policy = Policy::default_policy();
    assert_eq!(policy.max_result_bytes, 256 * 1024);
}

#[test]
fn test_policy_default_max_steps() {
    let policy = Policy::default_policy();
    assert_eq!(policy.max_steps, 1_000_000);
}

// ── Policy::within exhaustive violations ───────────────────────────────

#[test]
fn test_policy_within_violates_parallel() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.parallel = 300;
    let violations = policy.within(&budget);
    assert!(violations.contains(&"parallel"));
}

#[test]
fn test_policy_within_violates_run_time() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.run_time_secs = policy.max_run_time + 1;
    let violations = policy.within(&budget);
    assert!(violations.contains(&"run_time"));
}

#[test]
fn test_policy_within_violates_result_bytes() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.result_bytes = policy.max_result_bytes + 1;
    let violations = policy.within(&budget);
    assert!(violations.contains(&"result_bytes"));
}

#[test]
fn test_policy_within_violates_steps() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.steps = policy.max_steps + 1;
    let violations = policy.within(&budget);
    assert!(violations.contains(&"steps"));
}

#[test]
fn test_policy_within_multiple_violations() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.actions = policy.max_actions + 1;
    budget.parallel = policy.max_parallel + 1;
    budget.run_time_secs = policy.max_run_time + 1;
    let violations = policy.within(&budget);
    assert_eq!(violations.len(), 3);
    assert!(violations.contains(&"actions"));
    assert!(violations.contains(&"parallel"));
    assert!(violations.contains(&"run_time"));
}

#[test]
fn test_policy_within_exact_boundary_actions() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.actions = policy.max_actions;
    let violations = policy.within(&budget);
    assert!(violations.is_empty());
}

#[test]
fn test_policy_within_exact_boundary_parallel() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.parallel = policy.max_parallel;
    let violations = policy.within(&budget);
    assert!(violations.is_empty());
}

// ── sequential_compose ─────────────────────────────────────────────────

#[test]
fn test_sequential_compose_zero_budgets() {
    let a = Budget::new();
    let b = Budget::new();
    let result = sequential_compose(&a, &b);
    assert_eq!(result.steps, 0);
    assert_eq!(result.actions, 0);
    assert_eq!(result.parallel, 0);
}

#[test]
fn test_sequential_compose_adds_steps_and_actions() {
    let mut a = Budget::new();
    a.steps = 10;
    a.actions = 5;
    a.parallel = 3;

    let mut b = Budget::new();
    b.steps = 7;
    b.actions = 3;
    b.parallel = 4;

    let result = sequential_compose(&a, &b);
    assert_eq!(result.steps, 17);
    assert_eq!(result.actions, 8);
    assert_eq!(result.parallel, 4); // max(3, 4)
}

#[test]
fn test_sequential_compose_saturates_on_overflow() {
    let mut a = Budget::new();
    a.steps = u64::MAX;
    a.actions = u64::MAX;

    let mut b = Budget::new();
    b.steps = 1;
    b.actions = 1;

    let result = sequential_compose(&a, &b);
    assert_eq!(result.steps, u64::MAX);
    assert_eq!(result.actions, u64::MAX);
}

#[test]
fn test_sequential_compose_preserves_other_fields() {
    let mut a = Budget::new();
    a.gather_pages = 5;
    a.gather_items = 10;
    a.for_each_iters = 2;
    a.together_branches = 3;
    a.repeat_attempts = 4;
    a.run_time_secs = 100;
    a.result_bytes = 200;
    a.slots_written = 300;

    let b = Budget::new();
    let result = sequential_compose(&a, &b);
    assert_eq!(result.gather_pages, 5);
    assert_eq!(result.gather_items, 10);
    assert_eq!(result.for_each_iters, 2);
    assert_eq!(result.together_branches, 3);
    assert_eq!(result.repeat_attempts, 4);
    assert_eq!(result.run_time_secs, 100);
    assert_eq!(result.result_bytes, 200);
    assert_eq!(result.slots_written, 300);
}

#[test]
fn test_sequential_compose_both_nonzero_retries() {
    let mut a = Budget::new();
    a.retries = 5;
    let mut b = Budget::new();
    b.retries = 3;
    let result = sequential_compose(&a, &b);
    assert_eq!(result.retries, 5); // max(5, 3)
}

// ── branch_compose ──────────────────────────────────────────────────────

#[test]
fn test_branch_compose_zero_budgets() {
    let a = Budget::new();
    let b = Budget::new();
    let result = branch_compose(&a, &b);
    assert_eq!(result.steps, 0);
    assert_eq!(result.actions, 0);
}

#[test]
fn test_branch_compose_takes_max_of_each_field() {
    let mut a = Budget::new();
    a.steps = 10;
    a.actions = 5;
    a.parallel = 3;
    a.retries = 2;

    let mut b = Budget::new();
    b.steps = 7;
    b.actions = 8;
    b.parallel = 4;
    b.retries = 1;

    let result = branch_compose(&a, &b);
    assert_eq!(result.steps, 10); // max(10, 7)
    assert_eq!(result.actions, 8); // max(5, 8)
    assert_eq!(result.parallel, 4); // max(3, 4)
    assert_eq!(result.retries, 2); // max(2, 1)
}

#[test]
fn test_branch_compose_preserves_budget_values() {
    let mut a = Budget::new();
    a.steps = 100;
    a.result_bytes = 256;
    a.slots_written = 500;

    let mut b = Budget::new();
    b.steps = 50;
    b.result_bytes = 128;
    b.slots_written = 300;

    let result = branch_compose(&a, &b);
    assert_eq!(result.steps, 100);
    assert_eq!(result.result_bytes, 256);
    assert_eq!(result.slots_written, 500);
}

#[test]
fn test_branch_compose_a_wins_all_fields() {
    let mut a = Budget::new();
    a.steps = 100;
    a.actions = 100;
    a.parallel = 100;
    a.retries = 100;
    a.gather_pages = 100;
    a.gather_items = 100;
    a.for_each_iters = 100;
    a.together_branches = 100;
    a.repeat_attempts = 100;
    a.run_time_secs = 100;
    a.result_bytes = 100;
    a.slots_written = 100;

    let b = Budget::new();
    let result = branch_compose(&a, &b);
    assert_eq!(result.steps, 100);
    assert_eq!(result.actions, 100);
    assert_eq!(result.parallel, 100);
}

// ── loop_compose ───────────────────────────────────────────────────────

#[test]
fn test_loop_compose_zero_iterations() {
    let body = Budget::new();
    let result = loop_compose(&body, 0);
    assert_eq!(result.steps, 0);
    assert_eq!(result.actions, 0);
}

#[test]
fn test_loop_compose_one_iteration() {
    let mut body = Budget::new();
    body.steps = 10;
    body.actions = 5;

    let result = loop_compose(&body, 1);
    assert_eq!(result.steps, 10);
    assert_eq!(result.actions, 5);
}

#[test]
fn test_loop_compose_multiplies_all_fields() {
    let mut body = Budget::new();
    body.steps = 3;
    body.actions = 4;
    body.parallel = 2;
    body.retries = 5;

    let result = loop_compose(&body, 10);
    assert_eq!(result.steps, 30);
    assert_eq!(result.actions, 40);
    assert_eq!(result.parallel, 20);
    assert_eq!(result.retries, 50);
}

#[test]
fn test_loop_compose_saturates_on_overflow() {
    let mut body = Budget::new();
    body.steps = u64::MAX;
    body.actions = 2;

    let result = loop_compose(&body, 2);
    assert_eq!(result.steps, u64::MAX);
    assert_eq!(result.actions, 4);
}

#[test]
fn test_loop_compose_zero_body() {
    let body = Budget::new();
    let result = loop_compose(&body, 999);
    assert_eq!(result.steps, 0);
    assert_eq!(result.actions, 0);
}

#[test]
fn test_loop_compose_large_iteration_count() {
    let mut body = Budget::new();
    body.steps = 1;
    body.actions = 1;

    let result = loop_compose(&body, 1_000_000);
    assert_eq!(result.steps, 1_000_000);
    assert_eq!(result.actions, 1_000_000);
}

// ── Budget derived traits ────────────────────────────────────────────────

#[test]
fn test_budget_debug() {
    let budget = Budget::new();
    let debug = format!("{:?}", budget);
    assert!(debug.contains("Budget"));
    assert!(debug.contains("steps"));
}

#[test]
fn test_budget_clone() {
    let mut budget = Budget::new();
    budget.steps = 42;
    budget.actions = 7;
    let cloned = budget.clone();
    assert_eq!(cloned.steps, 42);
    assert_eq!(cloned.actions, 7);
}

// ── Policy derived traits ────────────────────────────────────────────────

#[test]
fn test_policy_debug() {
    let policy = Policy::default_policy();
    let debug = format!("{:?}", policy);
    assert!(debug.contains("Policy"));
    assert!(debug.contains("max_actions"));
}

#[test]
fn test_policy_clone() {
    let policy = Policy::default_policy();
    let cloned = policy.clone();
    assert_eq!(cloned.max_actions, policy.max_actions);
    assert_eq!(cloned.max_parallel, policy.max_parallel);
}

// ── Budget::branch_max — non-trivial field coverage ─────────────────────

#[test]
fn test_branch_max_non_trivial_parallel() {
    let mut a = Budget::new();
    a.parallel = 50;
    let mut b = Budget::new();
    b.parallel = 100;
    a.branch_max(&b);
    assert_eq!(a.parallel, 100);
}

#[test]
fn test_branch_max_non_trivial_retries() {
    let mut a = Budget::new();
    a.retries = 3;
    let mut b = Budget::new();
    b.retries = 7;
    a.branch_max(&b);
    assert_eq!(a.retries, 7);
}

#[test]
fn test_branch_max_non_trivial_gather_pages() {
    let mut a = Budget::new();
    a.gather_pages = 10;
    let mut b = Budget::new();
    b.gather_pages = 25;
    a.branch_max(&b);
    assert_eq!(a.gather_pages, 25);
}

#[test]
fn test_branch_max_non_trivial_gather_items() {
    let mut a = Budget::new();
    a.gather_items = 100;
    let mut b = Budget::new();
    b.gather_items = 50;
    a.branch_max(&b);
    assert_eq!(a.gather_items, 100);
}

#[test]
fn test_branch_max_non_trivial_for_each_iters() {
    let mut a = Budget::new();
    a.for_each_iters = 4;
    let mut b = Budget::new();
    b.for_each_iters = 8;
    a.branch_max(&b);
    assert_eq!(a.for_each_iters, 8);
}

#[test]
fn test_branch_max_non_trivial_together_branches() {
    let mut a = Budget::new();
    a.together_branches = 12;
    let mut b = Budget::new();
    b.together_branches = 6;
    a.branch_max(&b);
    assert_eq!(a.together_branches, 12);
}

#[test]
fn test_branch_max_non_trivial_repeat_attempts() {
    let mut a = Budget::new();
    a.repeat_attempts = 2;
    let mut b = Budget::new();
    b.repeat_attempts = 9;
    a.branch_max(&b);
    assert_eq!(a.repeat_attempts, 9);
}

#[test]
fn test_branch_max_non_trivial_run_time_secs() {
    let mut a = Budget::new();
    a.run_time_secs = 3600;
    let mut b = Budget::new();
    b.run_time_secs = 7200;
    a.branch_max(&b);
    assert_eq!(a.run_time_secs, 7200);
}

#[test]
fn test_branch_max_non_trivial_result_bytes() {
    let mut a = Budget::new();
    a.result_bytes = 1024;
    let mut b = Budget::new();
    b.result_bytes = 2048;
    a.branch_max(&b);
    assert_eq!(a.result_bytes, 2048);
}

#[test]
fn test_branch_max_non_trivial_slots_written() {
    let mut a = Budget::new();
    a.slots_written = 500;
    let mut b = Budget::new();
    b.slots_written = 300;
    a.branch_max(&b);
    assert_eq!(a.slots_written, 500);
}

// ── Budget::sequential_add — non-trivial field coverage ───────────────

#[test]
fn test_sequential_add_non_trivial_gather_pages() {
    let mut a = Budget::new();
    a.gather_pages = 5;
    let mut b = Budget::new();
    b.gather_pages = 3;
    a.sequential_add(&b);
    assert_eq!(a.gather_pages, 8);
}

#[test]
fn test_sequential_add_non_trivial_gather_items() {
    let mut a = Budget::new();
    a.gather_items = 100;
    let mut b = Budget::new();
    b.gather_items = 200;
    a.sequential_add(&b);
    assert_eq!(a.gather_items, 300);
}

#[test]
fn test_sequential_add_non_trivial_for_each_iters() {
    // for_each_iters uses .max(), not saturating_add
    let mut a = Budget::new();
    a.for_each_iters = 10;
    let mut b = Budget::new();
    b.for_each_iters = 15;
    a.sequential_add(&b);
    assert_eq!(a.for_each_iters, 15); // max(10, 15)
}

#[test]
fn test_sequential_add_non_trivial_together_branches() {
    // together_branches uses .max(), not saturating_add
    let mut a = Budget::new();
    a.together_branches = 4;
    let mut b = Budget::new();
    b.together_branches = 6;
    a.sequential_add(&b);
    assert_eq!(a.together_branches, 6); // max(4, 6)
}

#[test]
fn test_sequential_add_non_trivial_repeat_attempts() {
    // repeat_attempts uses .max(), not saturating_add
    let mut a = Budget::new();
    a.repeat_attempts = 3;
    let mut b = Budget::new();
    b.repeat_attempts = 2;
    a.sequential_add(&b);
    assert_eq!(a.repeat_attempts, 3); // max(3, 2)
}

#[test]
fn test_sequential_add_non_trivial_run_time_secs() {
    let mut a = Budget::new();
    a.run_time_secs = 100;
    let mut b = Budget::new();
    b.run_time_secs = 50;
    a.sequential_add(&b);
    assert_eq!(a.run_time_secs, 150);
}

#[test]
fn test_sequential_add_non_trivial_result_bytes() {
    // result_bytes uses .max(), not saturating_add
    let mut a = Budget::new();
    a.result_bytes = 256;
    let mut b = Budget::new();
    b.result_bytes = 128;
    a.sequential_add(&b);
    assert_eq!(a.result_bytes, 256); // max(256, 128)
}

#[test]
fn test_sequential_add_non_trivial_slots_written() {
    let mut a = Budget::new();
    a.slots_written = 10;
    let mut b = Budget::new();
    b.slots_written = 20;
    a.sequential_add(&b);
    assert_eq!(a.slots_written, 30);
}

// ── Budget::loop_mul — non-trivial field coverage ──────────────────────

#[test]
fn test_loop_mul_non_trivial_gather_pages() {
    let mut body = Budget::new();
    body.gather_pages = 3;
    body.loop_mul(4);
    assert_eq!(body.gather_pages, 12);
}

#[test]
fn test_loop_mul_non_trivial_gather_items() {
    let mut body = Budget::new();
    body.gather_items = 5;
    body.loop_mul(7);
    assert_eq!(body.gather_items, 35);
}

#[test]
fn test_loop_mul_non_trivial_for_each_iters() {
    let mut body = Budget::new();
    body.for_each_iters = 2;
    body.loop_mul(10);
    assert_eq!(body.for_each_iters, 20);
}

#[test]
fn test_loop_mul_non_trivial_together_branches() {
    let mut body = Budget::new();
    body.together_branches = 3;
    body.loop_mul(5);
    assert_eq!(body.together_branches, 15);
}

#[test]
fn test_loop_mul_non_trivial_repeat_attempts() {
    let mut body = Budget::new();
    body.repeat_attempts = 4;
    body.loop_mul(3);
    assert_eq!(body.repeat_attempts, 12);
}

#[test]
fn test_loop_mul_non_trivial_run_time_secs() {
    let mut body = Budget::new();
    body.run_time_secs = 10;
    body.loop_mul(6);
    assert_eq!(body.run_time_secs, 60);
}

#[test]
fn test_loop_mul_non_trivial_result_bytes() {
    let mut body = Budget::new();
    body.result_bytes = 100;
    body.loop_mul(8);
    assert_eq!(body.result_bytes, 800);
}

#[test]
fn test_loop_mul_non_trivial_slots_written() {
    let mut body = Budget::new();
    body.slots_written = 15;
    body.loop_mul(4);
    assert_eq!(body.slots_written, 60);
}

#[test]
fn test_loop_mul_saturates_all_fields() {
    let mut body = Budget::new();
    body.gather_pages = u64::MAX;
    body.gather_items = u64::MAX;
    body.loop_mul(2);
    assert_eq!(body.gather_pages, u64::MAX);
    assert_eq!(body.gather_items, u64::MAX);
}

// ── Policy field accessors ─────────────────────────────────────────────

#[test]
fn test_policy_field_accessors() {
    let policy = Policy::default_policy();
    assert_eq!(policy.max_actions, 100_000);
    assert_eq!(policy.max_parallel, 256);
    assert_eq!(policy.max_run_time, 30 * 24 * 60 * 60);
    assert_eq!(policy.max_result_bytes, 256 * 1024);
    assert_eq!(policy.max_steps, 1_000_000);
}

// ── Policy::within — exact boundary ────────────────────────────────────

#[test]
fn test_policy_within_exact_boundary_run_time() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.run_time_secs = policy.max_run_time;
    let violations = policy.within(&budget);
    assert!(violations.is_empty());
}

#[test]
fn test_policy_within_exact_boundary_result_bytes() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.result_bytes = policy.max_result_bytes;
    let violations = policy.within(&budget);
    assert!(violations.is_empty());
}

#[test]
fn test_policy_within_exact_boundary_steps() {
    let policy = Policy::default_policy();
    let mut budget = Budget::new();
    budget.steps = policy.max_steps;
    let violations = policy.within(&budget);
    assert!(violations.is_empty());
}

// ── composition functions — non-trivial ────────────────────────────────

#[test]
fn test_sequential_compose_all_fields() {
    let mut a = Budget::new();
    a.gather_pages = 5;
    a.gather_items = 10;
    a.for_each_iters = 2;
    a.together_branches = 3;
    a.repeat_attempts = 4;
    a.run_time_secs = 100;
    a.result_bytes = 200;
    a.slots_written = 300;

    let mut b = Budget::new();
    b.gather_pages = 2;
    b.gather_items = 3;
    b.for_each_iters = 1;
    b.together_branches = 1;
    b.repeat_attempts = 1;
    b.run_time_secs = 50;
    b.result_bytes = 100;
    b.slots_written = 150;

    let result = sequential_compose(&a, &b);
    // saturating_add fields
    assert_eq!(result.gather_pages, 7); // 5 + 2
    assert_eq!(result.gather_items, 13); // 10 + 3
    assert_eq!(result.run_time_secs, 150); // 100 + 50
    assert_eq!(result.slots_written, 450); // 300 + 150
    // max fields
    assert_eq!(result.for_each_iters, 2); // max(2, 1)
    assert_eq!(result.together_branches, 3); // max(3, 1)
    assert_eq!(result.repeat_attempts, 4); // max(4, 1)
    assert_eq!(result.result_bytes, 200); // max(200, 100)
}

#[test]
fn test_branch_compose_all_fields() {
    let mut a = Budget::new();
    a.gather_pages = 5;
    a.gather_items = 10;
    a.for_each_iters = 2;
    a.together_branches = 3;
    a.repeat_attempts = 4;
    a.run_time_secs = 100;
    a.result_bytes = 200;
    a.slots_written = 300;

    let mut b = Budget::new();
    b.gather_pages = 10;
    b.gather_items = 3;
    b.for_each_iters = 8;
    b.together_branches = 1;
    b.repeat_attempts = 6;
    b.run_time_secs = 50;
    b.result_bytes = 400;
    b.slots_written = 150;

    let result = branch_compose(&a, &b);
    assert_eq!(result.gather_pages, 10);
    assert_eq!(result.gather_items, 10);
    assert_eq!(result.for_each_iters, 8);
    assert_eq!(result.together_branches, 3);
    assert_eq!(result.repeat_attempts, 6);
    assert_eq!(result.run_time_secs, 100);
    assert_eq!(result.result_bytes, 400);
    assert_eq!(result.slots_written, 300);
}

#[test]
fn test_loop_compose_all_fields() {
    let mut body = Budget::new();
    body.gather_pages = 2;
    body.gather_items = 3;
    body.for_each_iters = 4;
    body.together_branches = 5;
    body.repeat_attempts = 6;
    body.run_time_secs = 7;
    body.result_bytes = 8;
    body.slots_written = 9;

    let result = loop_compose(&body, 3);
    assert_eq!(result.gather_pages, 6);
    assert_eq!(result.gather_items, 9);
    assert_eq!(result.for_each_iters, 12);
    assert_eq!(result.together_branches, 15);
    assert_eq!(result.repeat_attempts, 18);
    assert_eq!(result.run_time_secs, 21);
    assert_eq!(result.result_bytes, 24);
    assert_eq!(result.slots_written, 27);
}

#[test]
fn test_loop_compose_saturates_gather_pages() {
    let mut body = Budget::new();
    body.gather_pages = u64::MAX;
    body.gather_items = 2;
    let result = loop_compose(&body, 2);
    assert_eq!(result.gather_pages, u64::MAX);
    assert_eq!(result.gather_items, 4);
}
