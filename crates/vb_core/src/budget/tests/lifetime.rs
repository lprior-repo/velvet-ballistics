// =========================================================================
// §77.18 bounded run lifetime (vb-8vknz)
// =========================================================================

#[test]
fn lifetime_allows_exact_limit_and_rejects_limit_plus_one() {
    let lifetime = RunLifetimeBudget::try_new(1_000, 2).expect("two seconds fit");
    assert_eq!(lifetime.started_millis(), 1_000);
    assert_eq!(lifetime.limit_millis(), 2_000);
    assert_eq!(lifetime.deadline_millis(), 3_000);
    assert_eq!(lifetime.remaining_millis(3_000), 0);
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
    assert_eq!(lifetime.deadline_millis(), 50);
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
    assert_eq!(error.outcome(), BudgetExhaustionOutcome::Fail);
    assert_eq!(
        error,
        LifetimeBudgetError::Expired {
            now_millis: 1_001,
            deadline_millis: 1_000,
        }
    );
}

#[test]
fn lifetime_remaining_never_exceeds_limit_when_clock_goes_backward() {
    let lifetime = RunLifetimeBudget::try_new(1_000, 2).expect("two seconds fit");
    assert_eq!(lifetime.remaining_millis(1_000), 2_000);
    assert_eq!(lifetime.remaining_millis(999), 2_000);
    assert_eq!(lifetime.remaining_millis(0), 2_000);
    assert_eq!(lifetime.remaining_millis(3_001), 0);
}

#[test]
fn lifetime_is_stable_across_repeated_checks_and_replay() {
    let lifetime = RunLifetimeBudget::try_new(500, 60).expect("one minute fits");
    let first_remaining = lifetime.remaining_millis(10_500);
    assert_eq!(first_remaining, 50_000);
    assert_eq!(lifetime.check(20_500), Ok(()));
    assert_eq!(lifetime.check(10_500), Ok(()));

    let bytes = postcard::to_allocvec(&lifetime).expect("lifetime serializes");
    let restored: RunLifetimeBudget = postcard::from_bytes(&bytes).expect("lifetime decodes");
    assert_eq!(restored, lifetime);
    assert_eq!(restored.deadline_millis(), 60_500);
    assert_eq!(restored.remaining_millis(10_500), first_remaining);
    assert_eq!(
        restored.check(60_501),
        Err(LifetimeBudgetError::Expired {
            now_millis: 60_501,
            deadline_millis: 60_500,
        })
    );
}

#[test]
fn lifetime_long_wait_expires_one_step_workflow() {
    let nodes = vec![finish_node(0)];
    let contract = test_contract(1, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .expect("one-step workflow computes");
    assert_eq!(budget.max_total_steps, 1, "one instruction");
    assert_eq!(
        budget.max_run_time_seconds, 1,
        "admitted lifetime remains derived from the existing budget"
    );

    let aggregate = AggregateResourceBudget::from_whole_workflow_budget(budget, contract)
        .expect("aggregate projection succeeds");
    let lifetime = RunLifetimeBudget::from_budget(0, &aggregate).expect("lifetime builds");
    let seven_days_millis = 7 * 24 * 60 * 60 * 1_000;
    assert_eq!(
        lifetime.check(seven_days_millis),
        Err(LifetimeBudgetError::Expired {
            now_millis: seven_days_millis,
            deadline_millis: 1_000,
        }),
        "wall-clock suspension expires even when no instruction was consumed"
    );
}
