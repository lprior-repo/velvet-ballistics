use vb_proof_kernels::resource_budget::{
    Budget, Policy, branch_compose, loop_compose, sequential_compose,
};

fn budget_with_all_fields(value: u64) -> Budget {
    Budget {
        steps: value,
        actions: value,
        parallel: value,
        retries: value,
        gather_pages: value,
        gather_items: value,
        for_each_iters: value,
        together_branches: value,
        repeat_attempts: value,
        run_time_secs: value,
        result_bytes: value,
        slots_written: value,
    }
}

#[test]
fn budget_new_has_zero_steps() {
    assert_eq!(Budget::new().steps, 0);
}

#[test]
fn budget_new_has_zero_actions() {
    assert_eq!(Budget::new().actions, 0);
}

#[test]
fn budget_default_matches_new_resource_fields() {
    let budget = Budget::default();
    assert_eq!(budget.gather_pages, 0);
    assert_eq!(budget.gather_items, 0);
    assert_eq!(budget.slots_written, 0);
}

#[test]
fn sequential_add_sums_steps() {
    let mut budget = Budget::new();
    budget.steps = 4;
    budget.sequential_add(&Budget {
        steps: 5,
        ..Budget::new()
    });
    assert_eq!(budget.steps, 9);
}

#[test]
fn sequential_add_sums_actions() {
    let mut budget = Budget::new();
    budget.actions = 4;
    budget.sequential_add(&Budget {
        actions: 5,
        ..Budget::new()
    });
    assert_eq!(budget.actions, 9);
}

#[test]
fn sequential_add_sums_gather_pages() {
    let mut budget = Budget::new();
    budget.gather_pages = 4;
    budget.sequential_add(&Budget {
        gather_pages: 5,
        ..Budget::new()
    });
    assert_eq!(budget.gather_pages, 9);
}

#[test]
fn sequential_add_sums_gather_items() {
    let mut budget = Budget::new();
    budget.gather_items = 4;
    budget.sequential_add(&Budget {
        gather_items: 5,
        ..Budget::new()
    });
    assert_eq!(budget.gather_items, 9);
}

#[test]
fn sequential_add_sums_run_time_secs() {
    let mut budget = Budget::new();
    budget.run_time_secs = 4;
    budget.sequential_add(&Budget {
        run_time_secs: 5,
        ..Budget::new()
    });
    assert_eq!(budget.run_time_secs, 9);
}

#[test]
fn sequential_add_sums_slots_written() {
    let mut budget = Budget::new();
    budget.slots_written = 4;
    budget.sequential_add(&Budget {
        slots_written: 5,
        ..Budget::new()
    });
    assert_eq!(budget.slots_written, 9);
}

#[test]
fn sequential_add_takes_parallel_max() {
    let mut budget = Budget::new();
    budget.parallel = 4;
    budget.sequential_add(&Budget {
        parallel: 5,
        ..Budget::new()
    });
    assert_eq!(budget.parallel, 5);
}

#[test]
fn sequential_add_takes_retries_max() {
    let mut budget = Budget::new();
    budget.retries = 7;
    budget.sequential_add(&Budget {
        retries: 5,
        ..Budget::new()
    });
    assert_eq!(budget.retries, 7);
}

#[test]
fn sequential_add_takes_for_each_iters_max() {
    let mut budget = Budget::new();
    budget.for_each_iters = 3;
    budget.sequential_add(&Budget {
        for_each_iters: 6,
        ..Budget::new()
    });
    assert_eq!(budget.for_each_iters, 6);
}

#[test]
fn sequential_add_takes_together_branches_max() {
    let mut budget = Budget::new();
    budget.together_branches = 8;
    budget.sequential_add(&Budget {
        together_branches: 6,
        ..Budget::new()
    });
    assert_eq!(budget.together_branches, 8);
}

#[test]
fn sequential_add_takes_repeat_attempts_max() {
    let mut budget = Budget::new();
    budget.repeat_attempts = 2;
    budget.sequential_add(&Budget {
        repeat_attempts: 6,
        ..Budget::new()
    });
    assert_eq!(budget.repeat_attempts, 6);
}

#[test]
fn sequential_add_takes_result_bytes_max() {
    let mut budget = Budget::new();
    budget.result_bytes = 512;
    budget.sequential_add(&Budget {
        result_bytes: 128,
        ..Budget::new()
    });
    assert_eq!(budget.result_bytes, 512);
}

#[test]
fn sequential_add_saturates_steps() {
    let mut budget = Budget::new();
    budget.steps = u64::MAX;
    budget.sequential_add(&Budget {
        steps: 1,
        ..Budget::new()
    });
    assert_eq!(budget.steps, u64::MAX);
}

#[test]
fn sequential_add_saturates_actions() {
    let mut budget = Budget::new();
    budget.actions = u64::MAX;
    budget.sequential_add(&Budget {
        actions: 1,
        ..Budget::new()
    });
    assert_eq!(budget.actions, u64::MAX);
}

#[test]
fn branch_max_takes_larger_steps() {
    let mut budget = Budget {
        steps: 10,
        ..Budget::new()
    };
    budget.branch_max(&Budget {
        steps: 11,
        ..Budget::new()
    });
    assert_eq!(budget.steps, 11);
}

#[test]
fn branch_max_keeps_larger_actions() {
    let mut budget = Budget {
        actions: 10,
        ..Budget::new()
    };
    budget.branch_max(&Budget {
        actions: 9,
        ..Budget::new()
    });
    assert_eq!(budget.actions, 10);
}

#[test]
fn branch_max_takes_larger_gather_pages() {
    let mut budget = Budget {
        gather_pages: 10,
        ..Budget::new()
    };
    budget.branch_max(&Budget {
        gather_pages: 12,
        ..Budget::new()
    });
    assert_eq!(budget.gather_pages, 12);
}

#[test]
fn branch_max_takes_larger_result_bytes() {
    let mut budget = Budget {
        result_bytes: 128,
        ..Budget::new()
    };
    budget.branch_max(&Budget {
        result_bytes: 256,
        ..Budget::new()
    });
    assert_eq!(budget.result_bytes, 256);
}

#[test]
fn branch_max_keeps_larger_slots_written() {
    let mut budget = Budget {
        slots_written: 512,
        ..Budget::new()
    };
    budget.branch_max(&Budget {
        slots_written: 256,
        ..Budget::new()
    });
    assert_eq!(budget.slots_written, 512);
}

#[test]
fn loop_mul_multiplies_steps() {
    let mut budget = Budget {
        steps: 7,
        ..Budget::new()
    };
    budget.loop_mul(6);
    assert_eq!(budget.steps, 42);
}

#[test]
fn loop_mul_multiplies_actions() {
    let mut budget = Budget {
        actions: 7,
        ..Budget::new()
    };
    budget.loop_mul(6);
    assert_eq!(budget.actions, 42);
}

#[test]
fn loop_mul_multiplies_parallel() {
    let mut budget = Budget {
        parallel: 7,
        ..Budget::new()
    };
    budget.loop_mul(6);
    assert_eq!(budget.parallel, 42);
}

#[test]
fn loop_mul_saturates_on_overflow() {
    let mut budget = Budget {
        steps: u64::MAX,
        ..Budget::new()
    };
    budget.loop_mul(2);
    assert_eq!(budget.steps, u64::MAX);
}

#[test]
fn loop_mul_zero_iterations_clears_counted_fields() {
    let mut budget = budget_with_all_fields(9);
    budget.loop_mul(0);
    assert_eq!(budget.steps, 0);
    assert_eq!(budget.result_bytes, 0);
}

#[test]
fn default_policy_has_expected_parallel_limit() {
    assert_eq!(Policy::default_policy().max_parallel, 256);
}

#[test]
fn policy_within_accepts_empty_budget() {
    assert!(Policy::default_policy().within(&Budget::new()).is_empty());
}

#[test]
fn policy_within_reports_all_enforced_limits() {
    let policy = Policy::default_policy();
    let budget = Budget {
        actions: policy.max_actions + 1,
        parallel: policy.max_parallel + 1,
        run_time_secs: policy.max_run_time + 1,
        result_bytes: policy.max_result_bytes + 1,
        steps: policy.max_steps + 1,
        ..Budget::new()
    };
    let violations = policy.within(&budget);
    assert_eq!(violations.len(), 5);
    assert!(violations.contains(&"actions"));
    assert!(violations.contains(&"parallel"));
    assert!(violations.contains(&"run_time"));
    assert!(violations.contains(&"result_bytes"));
    assert!(violations.contains(&"steps"));
}

#[test]
fn compose_functions_match_in_place_operations() {
    let a = budget_with_all_fields(2);
    let b = budget_with_all_fields(3);
    assert_eq!(sequential_compose(&a, &b).steps, 5);
    assert_eq!(branch_compose(&a, &b).steps, 3);
    assert_eq!(loop_compose(&a, 4).steps, 8);
}
