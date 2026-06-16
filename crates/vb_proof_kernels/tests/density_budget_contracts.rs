#![forbid(unsafe_code)]

use vb_proof_kernels::resource_budget::{
    Budget, Policy, branch_compose, loop_compose, sequential_compose,
};

macro_rules! ktest {
    ($(#[$attr:meta])* $name:ident, $body:block) => {
        $(#[$attr])*
        fn $name() $body
    };
}

fn budget_with_steps_actions(steps: u64, actions: u64) -> Budget {
    let mut budget = Budget::new();
    budget.steps = steps;
    budget.actions = actions;
    budget
}

ktest!(
    #[test]
    budget_new_starts_with_zero_steps,
    {
        assert_eq!(Budget::new().steps, 0);
    }
);

ktest!(
    #[test]
    budget_new_starts_with_zero_actions,
    {
        assert_eq!(Budget::new().actions, 0);
    }
);

ktest!(
    #[test]
    budget_sequential_compose_adds_steps,
    {
        let result = sequential_compose(
            &budget_with_steps_actions(2, 3),
            &budget_with_steps_actions(5, 7),
        );
        assert_eq!(result.steps, 7);
    }
);

ktest!(
    #[test]
    budget_sequential_compose_adds_actions,
    {
        let result = sequential_compose(
            &budget_with_steps_actions(2, 3),
            &budget_with_steps_actions(5, 7),
        );
        assert_eq!(result.actions, 10);
    }
);

ktest!(
    #[test]
    budget_sequential_compose_saturates_steps,
    {
        let result = sequential_compose(
            &budget_with_steps_actions(u64::MAX, 0),
            &budget_with_steps_actions(1, 0),
        );
        assert_eq!(result.steps, u64::MAX);
    }
);

ktest!(
    #[test]
    budget_branch_compose_maxes_steps,
    {
        let result = branch_compose(
            &budget_with_steps_actions(2, 9),
            &budget_with_steps_actions(5, 7),
        );
        assert_eq!(result.steps, 5);
    }
);

ktest!(
    #[test]
    budget_branch_compose_maxes_actions,
    {
        let result = branch_compose(
            &budget_with_steps_actions(2, 9),
            &budget_with_steps_actions(5, 7),
        );
        assert_eq!(result.actions, 9);
    }
);

ktest!(
    #[test]
    budget_loop_compose_multiplies_steps,
    {
        let result = loop_compose(&budget_with_steps_actions(2, 3), 4);
        assert_eq!(result.steps, 8);
    }
);

ktest!(
    #[test]
    budget_loop_compose_multiplies_actions,
    {
        let result = loop_compose(&budget_with_steps_actions(2, 3), 4);
        assert_eq!(result.actions, 12);
    }
);

ktest!(
    #[test]
    budget_loop_compose_saturates_actions,
    {
        let result = loop_compose(&budget_with_steps_actions(0, u64::MAX), 2);
        assert_eq!(result.actions, u64::MAX);
    }
);

ktest!(
    #[test]
    budget_policy_default_max_actions,
    {
        assert_eq!(Policy::default_policy().max_actions, 100_000);
    }
);

ktest!(
    #[test]
    budget_policy_default_max_parallel,
    {
        assert_eq!(Policy::default_policy().max_parallel, 256);
    }
);

ktest!(
    #[test]
    budget_policy_default_max_steps,
    {
        assert_eq!(Policy::default_policy().max_steps, 1_000_000);
    }
);

ktest!(
    #[test]
    budget_policy_accepts_empty_budget,
    {
        assert!(Policy::default_policy().within(&Budget::new()).is_empty());
    }
);

ktest!(
    #[test]
    budget_policy_rejects_actions_over_limit,
    {
        let mut budget = Budget::new();
        budget.actions = 100_001;
        assert_eq!(Policy::default_policy().within(&budget), vec!["actions"]);
    }
);

ktest!(
    #[test]
    budget_policy_rejects_parallel_over_limit,
    {
        let mut budget = Budget::new();
        budget.parallel = 257;
        assert_eq!(Policy::default_policy().within(&budget), vec!["parallel"]);
    }
);

ktest!(
    #[test]
    budget_policy_rejects_runtime_over_limit,
    {
        let policy = Policy::default_policy();
        let mut budget = Budget::new();
        budget.run_time_secs = policy.max_run_time.saturating_add(1);
        assert_eq!(policy.within(&budget), vec!["run_time"]);
    }
);

ktest!(
    #[test]
    budget_policy_rejects_result_bytes_over_limit,
    {
        let policy = Policy::default_policy();
        let mut budget = Budget::new();
        budget.result_bytes = policy.max_result_bytes.saturating_add(1);
        assert_eq!(policy.within(&budget), vec!["result_bytes"]);
    }
);

ktest!(
    #[test]
    budget_policy_rejects_steps_over_limit,
    {
        let mut budget = Budget::new();
        budget.steps = 1_000_001;
        assert_eq!(Policy::default_policy().within(&budget), vec!["steps"]);
    }
);

ktest!(
    #[test]
    budget_policy_reports_multiple_violations_in_order,
    {
        let mut budget = Budget::new();
        budget.actions = 100_001;
        budget.steps = 1_000_001;
        assert_eq!(
            Policy::default_policy().within(&budget),
            vec!["actions", "steps"]
        );
    }
);
