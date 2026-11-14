#[test]
fn workflow_needing_signal_subscription_is_refused_by_zero_capacity() {
    let nodes = vec![wait_event_node(0, None)];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &test_contract(1, 1))
        .expect("single WaitEvent budget must compute");
    assert_eq!(budget.max_signal_subscriptions, 1);
    let usage = AggregateResourceUsage {
        max_signal_subscriptions: u64::from(budget.max_signal_subscriptions),
        ..AggregateResourceUsage::default()
    };
    let capacity = AggregateResourceCapacity {
        max_signal_subscriptions: 0,
        ..ample_capacity()
    };
    assert_eq!(
        usage.fits_within(&capacity),
        Err(AggregateBudgetError::CapacityExceeded {
            resource: "max_signal_subscriptions",
            requested: 1,
            available: 0,
        })
    );
}
