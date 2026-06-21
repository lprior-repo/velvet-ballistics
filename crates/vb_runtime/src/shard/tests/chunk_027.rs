
#[test]
fn test_drain_for_shutdown_clears_mixed_wait_and_ask_timers() -> Result<(), RuntimeError> {
    // Given: a shard with runs suspended on both Wait and Ask timers
    let config = small_config();
    let mut shard = Shard::new(config)?;

    let Some(wait_workflow) = timed_wait_then_finish_workflow() else {
        return Ok(());
    };
    let Some(ask_workflow) = timed_ask_without_answer_workflow() else {
        return Ok(());
    };

    let run_wait = super::RunId::new(9004);
    let run_ask = super::RunId::new(9005);

    // Submit wait workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run_wait,
            workflow: wait_workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Submit ask workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run_ask,
            workflow: ask_workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timers.len(), 2);

    // When: drain_for_shutdown processes Shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.drain_for_shutdown(), Ok(()));

    // Then: all pending timers are cleared regardless of kind
    assert_eq!(shard.pending_timers.len(), 0);
    assert_eq!(shard.is_shutting_down(), true);
    Ok(())
}
