
#[test]
fn vb1u88_shutdown_is_permanent_no_unshutdown() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    for _ in 0..10 {
        assert_eq!(shard.tick(), Ok(false));
    }
    assert_eq!(shard.is_shutting_down(), true);
    let status = shard.status();
    assert_eq!(status.health, super::ShardHealth::ShuttingDown);
    Ok(())
}

// ---------------------------------------------------------------------------
// Section 5: Error Paths — RunNotFound variants
// ---------------------------------------------------------------------------

#[test]
fn vb1u88_action_completion_unknown_run_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(9999),
        step: vb_core::ids::StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
            ..Default::default()
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: vb_core::value::SlotValue::I64(1),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn vb1u88_action_failure_unknown_run_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(9999),
        step: vb_core::ids::StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
            ..Default::default()
    };
    let failure = vb_core::action::ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn vb1u88_resume_unknown_run_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    assert_eq!(
        shard.enqueue(ShardCommand::Resume {
            run: super::RunId::new(9999)
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn vb1u88_timer_fire_unknown_run_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    assert_eq!(
        shard.enqueue(invalid_timer_command(super::RunId::new(9999))),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

#[test]
fn vb1u88_ask_answer_unknown_run_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let answer = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(9999),
            ask_step: vb_core::ids::StepIdx::ZERO,
            resume_step: vb_core::ids::StepIdx::new(1),
        },
        answer_slot: SlotIdx::ZERO,
        value: vb_core::value::SlotValue::Bool(true),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

// ---------------------------------------------------------------------------
// Section 6: Invariants
// ---------------------------------------------------------------------------

#[test]
fn vb1u88_invariant_runs_len_never_exceeds_max() -> Result<(), RuntimeError> {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
};
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    for i in 0..5 {
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: super::RunId::new(i as u64),
                workflow: workflow.clone(),
                caps: vb_core::capability::CapabilitySet::empty()
            }),
            Ok(())
        );
        let _ = shard.tick();
        assert!(
            shard.runs.len() <= config.max_active_runs,
            "runs.len() = {} should never exceed max_active_runs = {}",
            shard.runs.len(),
            config.max_active_runs
        );
    }
    Ok(())
}

#[test]
fn vb1u88_invariant_queue_len_never_exceeds_capacity() -> Result<(), RuntimeError> {
    let config = ShardConfig {
        command_queue_capacity: 3,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
};
    let shard = Shard::new(config)?;
    for _ in 0..3 {
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    }
    assert_eq!(
        shard.enqueue(ShardCommand::Shutdown),
        Err(RuntimeError::QueueFull)
    );
    assert!(shard.command_queue.len() <= shard.command_queue.capacity());
    Ok(())
}

#[test]
fn vb1u88_invariant_no_trace_dropped_during_operation() -> Result<(), RuntimeError> {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 32,
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
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    for i in 0..4 {
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: super::RunId::new(i as u64),
                workflow: workflow.clone(),
                caps: vb_core::capability::CapabilitySet::empty()
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }
    assert_eq!(shard.trace_ring().dropped(), 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Section 7: Edge Cases
// ---------------------------------------------------------------------------

#[test]
fn vb1u88_run_id_zero_handled_correctly() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(0),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn vb1u88_max_run_id_handled_correctly() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(u64::MAX),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn vb1u88_multiple_sequential_finished_runs_no_leakage() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    for i in 0..10 {
        let run_id = super::RunId::new(i);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: run_id,
                workflow: workflow.clone(),
                caps: vb_core::capability::CapabilitySet::empty()
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.run_state_get(run_id), None);
    }
    assert_eq!(shard.counters().snapshot().runs_completed, 10);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    Ok(())
}
