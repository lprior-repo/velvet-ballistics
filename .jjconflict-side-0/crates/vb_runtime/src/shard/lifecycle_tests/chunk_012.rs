#[test]
fn journal_before_dispatch() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(workflow) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return Ok(());
    };
    let run = RunId::new(60_001);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(ticket) = shard.pending_action_get(run) else {
        assert_eq!(None::<()>, Some(()), "run should have a pending action");
        return Ok(());
    };
    assert_eq!(ticket.step, StepIdx::ZERO);
    assert_eq!(ticket.action, ActionId::new(0));
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                RuntimeJournalEvent::ActionScheduledTicket { ticket: scheduled, .. }
                    if scheduled.run == run
                        && scheduled.step == StepIdx::ZERO
                        && scheduled.action == ActionId::new(0)
            ))
            .count(),
        1
    );
    assert!(matches!(
        events.last(),
        Some(RuntimeJournalEvent::ActionScheduledTicket { ticket: scheduled, .. })
            if scheduled.run == run
                && scheduled.step == StepIdx::ZERO
                && scheduled.action == ActionId::new(0)
    ));
    Ok(())
}

#[test]
fn journal_before_dispatch_full_journal_blocks_dispatch() -> Result<(), String> {
    let Some(capacity) = std::num::NonZeroUsize::new(2) else {
        assert_eq!(None::<()>, Some(()), "capacity 2 must be nonzero");
        return Ok(());
    };
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::with_capacity(
        capacity,
    ));
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(workflow) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return Ok(());
    };
    let run = RunId::new(60_002);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::JournalFull { capacity: 2 }));
    assert_eq!(shard.pending_action_get(run), None);
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, RuntimeJournalEvent::ActionScheduledTicket { .. }))
            .count(),
        0
    );
    assert_eq!(events.len(), 2);
    assert!(shard.run_state_get_mut(run).is_some());
    Ok(())
}
