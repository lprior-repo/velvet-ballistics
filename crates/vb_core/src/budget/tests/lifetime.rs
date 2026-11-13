// =========================================================================
// §77.18 bounded run lifetime (vb-8vknz)
// =========================================================================

#[test]
fn lifetime_allows_exact_limit_and_rejects_limit_plus_one() {
    let lifetime = RunLifetimeBudget::try_new(1_000, 2).expect("two seconds fit");
    assert_eq!(lifetime.check(3_000), Ok(()));
    assert_eq!(
        lifetime.check(3_001),
        Err(LifetimeBudgetError::Expired {
            now_millis: 3_001,
            deadline_millis: 3_000,
        })
    );
}

#[test]
fn lifetime_zero_limit_admits_start_and_rejects_one_millisecond() {
    let lifetime = RunLifetimeBudget::try_new(50, 0).expect("zero is an explicit bound");
    assert_eq!(lifetime.check(50), Ok(()));
    assert_eq!(
        lifetime.check(51),
        Err(LifetimeBudgetError::Expired {
            now_millis: 51,
            deadline_millis: 50,
        })
    );
}

#[test]
fn lifetime_rejects_seconds_to_millis_overflow() {
    assert_eq!(
        RunLifetimeBudget::try_new(0, u64::MAX),
        Err(LifetimeBudgetError::LimitOverflow {
            max_run_time_seconds: u64::MAX,
        })
    );
}

#[test]
fn lifetime_rejects_deadline_overflow() {
    assert_eq!(
        RunLifetimeBudget::try_new(u64::MAX - 500, 1),
        Err(LifetimeBudgetError::DeadlineOverflow {
            started_millis: u64::MAX - 500,
            limit_millis: 1_000,
        })
    );
}

#[test]
fn lifetime_expiry_is_typed_failure() {
    let lifetime = RunLifetimeBudget::try_new(0, 1).expect("one second fits");
    let error = lifetime.check(1_001).expect_err("one millisecond late");
    assert_eq!(
        error,
        LifetimeBudgetError::Expired {
            now_millis: 1_001,
            deadline_millis: 1_000,
        }
    );
}

#[test]
fn lifetime_is_stable_across_repeated_checks_and_replay() {
    let lifetime = RunLifetimeBudget::try_new(500, 60).expect("one minute fits");
    assert_eq!(lifetime.check(20_500), Ok(()));
    assert_eq!(lifetime.check(10_500), Ok(()));

    let bytes = postcard::to_allocvec(&lifetime).expect("lifetime serializes");
    let restored: RunLifetimeBudget = postcard::from_bytes(&bytes).expect("lifetime decodes");
    assert_eq!(restored, lifetime);
    assert_eq!(
        restored.check(60_501),
        Err(LifetimeBudgetError::Expired {
            now_millis: 60_501,
            deadline_millis: 60_500,
        })
    );
}

#[test]
fn lifetime_default_profile_cap_is_independent_of_step_count() -> Result<(), String> {
    let nodes = vec![finish_node(0)];
    let contract = test_contract(1, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|error| error.to_string())?;
    assert_eq!(budget.max_total_steps, 1, "one instruction");
    let profile_cap = BoundednessPolicy::DEFAULT.absolute_max_run_time_seconds;
    assert_eq!(
        budget.max_run_time_seconds, profile_cap,
        "one-step workflow receives the explicit default lifetime profile"
    );

    let aggregate = AggregateResourceBudget::from_whole_workflow_budget(budget, contract)
        .map_err(|error| format!("aggregate projection failed: {error}"))?;
    let lifetime = RunLifetimeBudget::from_budget(0, &aggregate)
        .map_err(|error| format!("lifetime construction failed: {error}"))?;
    let exact_deadline = profile_cap
        .checked_mul(1_000)
        .ok_or_else(|| String::from("profile lifetime milliseconds overflow"))?;
    let after_deadline = exact_deadline
        .checked_add(1)
        .ok_or_else(|| String::from("after-deadline instant overflow"))?;
    assert_eq!(lifetime.check(exact_deadline), Ok(()));
    assert_eq!(
        lifetime.check(after_deadline),
        Err(LifetimeBudgetError::Expired {
            now_millis: after_deadline,
            deadline_millis: exact_deadline,
        }),
        "the explicit lifetime cap is enforced independently of steps"
    );
    Ok(())
}

#[test]
fn general_budget_path_uses_the_same_default_lifetime_profile() -> Result<(), String> {
    let nodes = vec![finish_node(0), finish_node(1), finish_node(2)];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &test_contract(3, 1))
        .map_err(|error| error.to_string())?;
    assert_eq!(budget.max_total_steps, 1);
    assert_eq!(
        budget.max_run_time_seconds,
        BoundednessPolicy::DEFAULT.absolute_max_run_time_seconds
    );
    Ok(())
}

#[test]
fn lifetime_wire_decode_rejects_deadline_overflow() {
    let wire = crate::budget::lifetime::LifetimeBudgetWire {
        started_millis: u64::MAX - 500,
        limit_millis: 1_000,
    };
    assert_eq!(
        RunLifetimeBudget::try_from(wire),
        Err(LifetimeBudgetError::DeadlineOverflow {
            started_millis: u64::MAX - 500,
            limit_millis: 1_000,
        })
    );
}
