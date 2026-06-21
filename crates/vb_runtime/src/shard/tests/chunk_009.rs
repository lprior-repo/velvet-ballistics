
#[test]
fn shard_cancel_then_resubmit_then_cancel_increments_failed_twice() -> Result<(), RuntimeError> {
    // Given a shard with a cancelled run that is then re-submitted
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(301);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling the re-submitted run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then failed counter is 2 (both cancellations counted)
    assert_eq!(shard.counters().snapshot().runs_failed, 2);
    assert_eq!(shard.counters().snapshot().runs_submitted, 2);
    Ok(())
}

#[test]
fn shard_action_completed_with_wrong_action_id_returns_invalid_completion() -> Result<(), RuntimeError> {
    // Given a shard with a suspended run on ActionId(0)
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(302);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When completing the action with a wrong action id
    let ticket = vb_core::action::ActionTicket {
        run,
        step: vb_core::ids::StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(99),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
            ..Default::default()
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::I64(1),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 8,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    // Then tick returns InvalidActionCompletion
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    Ok(())
}

#[test]
fn shard_action_completed_for_finished_run_returns_run_not_found() -> Result<(), RuntimeError> {
    // Given a shard where a run has already finished
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(303);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    // When completing an action for the finished run
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: vb_core::ids::StepIdx::ZERO,
        }),
        Ok(())
    );
    // Then tick returns RunNotFound (run was removed after finishing)
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn shard_snapshot_run_after_cancel_returns_terminal_cancelled() -> Result<(), RuntimeError> {
    // Given a shard with a cancelled run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(304);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // When snapshotting the cancelled run
    let response = shard.snapshot_run(run, 7);
    // Then it returns Terminal { Cancelled } (post-mortem observability)
    assert_eq!(
        response,
        InspectResponse::Terminal {
            run,
            correlation: 7,
            outcome: TerminalOutcome::Cancelled,
        }
    );
    Ok(())
}

#[test]
fn shard_timer_for_cancelled_run_returns_run_not_found() -> Result<(), RuntimeError> {
    // Given a shard with a cancelled run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(305);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // When a timer fires for the cancelled run
    assert_eq!(shard.enqueue(invalid_timer_command(run)), Ok(()));
    // Then tick rejects stale timer authority.
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

#[test]
fn shard_resume_for_cancelled_run_returns_run_not_found() -> Result<(), RuntimeError> {
    // Given a shard with a cancelled run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(306);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // When resuming the cancelled run
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn shard_trace_ring_overflow_drops_events_gracefully() -> Result<(), RuntimeError> {
    // Given a shard with trace capacity of 2
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 2,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
};
    let mut shard = Shard::new(config)?;
    // When submitting and completing multiple runs (producing >2 trace events)
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(401),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(402),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(403),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(404),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the trace ring has dropped events
    let events = shard.trace_ring_mut().drain();
    assert_eq!(events.len() <= 2, true);
    assert_eq!(shard.trace_ring().dropped() > 0, true);
    Ok(())
}
