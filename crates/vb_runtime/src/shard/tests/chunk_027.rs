#[test]
fn test_drain_for_shutdown_clears_mixed_wait_and_ask_timers() {
    // Given: a shard with runs suspended on both Wait and Ask timers
    let config = small_config();
    let mut shard = Shard::new(config); shard.configure_live_policy(crate::policy::test_permit_all_policy()).unwrap();

    let Some(wait_workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let Some(ask_workflow) = timed_ask_without_answer_workflow() else {
        return;
    };

    let run_wait = super::RunId::new(9004);
    let run_ask = super::RunId::new(9005);

    // Submit wait workflow
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run: run_wait,
            workflow: wait_workflow,
            caps: test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: test_fixtures::contracts_through(ActionId::new(0)),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Submit ask workflow
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run: run_ask,
            workflow: ask_workflow,
            caps: test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: test_fixtures::contracts_through(ActionId::new(0)),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timer_count(), 2);

    // When: drain_for_shutdown processes Shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));

    // Then: all pending timers are cleared regardless of kind
    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.is_shutting_down(), true);
}
