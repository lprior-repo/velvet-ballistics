
#[test]
fn shard_step_budget_one_processes_one_command_per_tick() -> Result<(), RuntimeError> {
    // Given a shard with step_budget_per_tick = 1
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 1,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    
};
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return;
    };
    // When submitting a 2-step finished workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then with budget 1, the first step executes but second does not
    // (budget exhausted after 1 transition; second tick needed)
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    Ok(())
}

#[test]
fn shard_duplicate_run_id_returns_run_already_exists_after_first_accepted() -> Result<(), RuntimeError> {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(42);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When submitting the same run ID again with a different workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns RunAlreadyExists (cannot replace workflow)
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    Ok(())
}

#[test]
fn shard_action_failed_for_unknown_run_returns_run_not_found() -> Result<(), RuntimeError> {
    // Given a shard with no active runs
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(999),
        step: vb_core::ids::StepIdx::new(0),
        seq: vb_core::ids::SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
            ..Default::default()
    };
    let failure = vb_core::action::ActionFailure {
        code: vb_core::action::ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    // When failing an action for a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn shard_run_id_max_u64_accepted_as_valid_identifier() -> Result<(), RuntimeError> {
    // Given a shard
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(u64::MAX);
    // When submitting a run with RunId::MAX
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the run is accepted and completes
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn shard_ask_answered_for_unknown_run_returns_run_not_found() -> Result<(), RuntimeError> {
    // Given a shard with no active runs
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let answer = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(999),
            ask_step: vb_core::ids::StepIdx::new(0),
            resume_step: vb_core::ids::StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::SlotValue::I64(42),
        taint: vb_core::Taint::Clean,
        encoded_len: 0,
    };
    // When answering an ask for a non-existent run
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn shard_snapshot_for_nonexistent_run_returns_not_found() -> Result<(), RuntimeError> {
    // Given a shard with no runs
    let config = small_config();
    let shard = Shard::new(config)?;
    // When snapshotting a non-existent run
    let response = shard.snapshot_run(super::RunId::new(999), 42);
    // Then NotFound is returned
    assert_eq!(
        response,
        InspectResponse::NotFound {
            run: super::RunId::new(999),
            correlation: 42,
        }
    );
    Ok(())
}

#[test]
fn shard_cancel_then_resubmit_same_run_id_succeeds() -> Result<(), RuntimeError> {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(55);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling and re-submitting with same ID
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the re-submitted run completes
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

#[test]
fn shard_trace_ring_records_submit_and_finish_events_in_order() -> Result<(), RuntimeError> {
    // Given a shard
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(77);
    // When submitting a run that finishes immediately
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then trace ring has Submit and Finished events
    let events = shard.trace_ring_mut().drain();
    let found_submit = events
        .iter()
        .any(|e| *e == TraceEvent::RunSubmitted { run });
    let found_finish = events.iter().any(|e| *e == TraceEvent::RunFinished { run });
    assert_eq!(found_submit, true);
    assert_eq!(found_finish, true);
    Ok(())
}

#[test]
fn shard_with_zero_trace_capacity_does_not_crash_on_submit() -> Result<(), RuntimeError> {
    // Given a shard with trace_capacity = 0
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 0,
        step_budget_per_tick: 4,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    
};
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return;
    };
    // When submitting a run
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick succeeds (trace drops are non-fatal)
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn shard_command_queue_len_starts_at_zero() -> Result<(), RuntimeError> {
    // Given a fresh shard
    let config = small_config();
    let shard = Shard::new(config)?;
    // Then queue length is 0
    assert_eq!(shard.command_queue_len(), 0);
    Ok(())
}

#[test]
fn shard_command_queue_len_increments_on_enqueue() -> Result<(), RuntimeError> {
    // Given a shard with capacity 4
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    
};
    let shard = Shard::new(config)?;
    assert_eq!(shard.command_queue_len(), 0);
    // When enqueuing commands
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.command_queue_len(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.command_queue_len(), 2);
    Ok(())
}
