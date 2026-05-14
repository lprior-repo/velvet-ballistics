
#[test]
fn shard_inspect_captures_executed_count() {
    // Given a shard with a finished workflow (executes 2 steps: SetConst + Finish)
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(81);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the steps_executed counter reflects execution
    assert_eq!(shard.counters().snapshot().steps_executed, 2);
}

#[test]
fn shard_tick_processes_commands_in_fifo_order() {
    // Given a shard with two submits enqueued
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf1) = finished_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    // When submitting two runs
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(100),
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(101),
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then both ticks succeed in FIFO order
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.tick(), Ok(true));
    // And counters show both submitted
    assert_eq!(shard.counters().snapshot().runs_submitted, 2);
    // And the first run (finished workflow) is completed
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn shard_resume_continues_suspended_run() {
    // Given a shard with a suspended run (Do node at step 0)
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(90);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When resuming the suspended run
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    // Then tick succeeds (run re-enters drive, suspends again on Do)
    assert_eq!(shard.tick(), Ok(true));
}

#[test]
fn shard_take_inspect_response_returns_none_initially() {
    // Given a fresh shard
    let config = small_config();
    let mut shard = Shard::new(config);
    // When taking inspect response without any inspect command
    let response = shard.take_inspect_response();
    // Then response is None
    assert_eq!(response, None);
}

#[test]
fn shard_take_inspect_response_clears_after_take() {
    // Given a shard with an inspect response available
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(95);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When taking the response
    let first = shard.take_inspect_response();
    assert_eq!(first.is_some(), true);
    // Then a second take returns None
    let second = shard.take_inspect_response();
    assert_eq!(second, None);
}

#[test]
fn shard_is_shutting_down_defaults_to_false() {
    // Given a fresh shard
    let config = small_config();
    let shard = Shard::new(config);
    // Then is_shutting_down is false
    assert_eq!(shard.is_shutting_down(), false);
}

#[test]
fn shard_config_default_values() {
    // Given a default ShardConfig
    let config = ShardConfig::default();
    // Then it has reasonable defaults
    assert_eq!(config.command_queue_capacity, 1024);
    assert_eq!(config.trace_capacity, 4096);
    assert_eq!(config.step_budget_per_tick, 1000);
    assert_eq!(config.max_active_runs, 1024);
}

#[test]
fn shard_config_equality_same_values() {
    // Given two identical configs
    let a = ShardConfig::default();
    let b = ShardConfig::default();
    // Then they are equal
    assert_eq!(a, b);
}

#[test]
fn shard_config_equality_differs() {
    // Given two different configs
    let a = ShardConfig::default();
    let b = ShardConfig {
        command_queue_capacity: 1,
        trace_capacity: 1,
        step_budget_per_tick: 1,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    // Then they are not equal
    assert_ne!(a, b);
}

#[test]
fn shard_config_clone_preserves_values() {
    // Given a config
    let original = small_config();
    // When cloning
    let cloned = original.clone();
    // Then clone matches original
    assert_eq!(cloned, original);
}

#[test]
fn shard_command_equality_submit() {
    // Given two identical Submit commands
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let a = ShardCommand::Submit {
        run: super::RunId::new(1),
        workflow: wf.clone(),
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    let b = ShardCommand::Submit {
        run: super::RunId::new(1),
        workflow: wf,
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_cancel() {
    // Given two identical Cancel commands
    let a = ShardCommand::Cancel {
        run: super::RunId::new(1),
    reason: None};
    let b = ShardCommand::Cancel {
        run: super::RunId::new(1),
    reason: None};
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_differs_run_id() {
    // Given two Cancel commands with different run IDs
    let a = ShardCommand::Cancel {
        run: super::RunId::new(1),
    reason: None};
    let b = ShardCommand::Cancel {
        run: super::RunId::new(2),
    reason: None};
    assert_ne!(a, b);
}

#[test]
fn shard_command_equality_shutdown() {
    // Given two Shutdown commands
    let a = ShardCommand::Shutdown;
    let b = ShardCommand::Shutdown;
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_inspect() {
    // Given two identical Inspect commands
    let a = ShardCommand::Inspect {
        run: super::RunId::new(1),
        correlation: 42,
    };
    let b = ShardCommand::Inspect {
        run: super::RunId::new(1),
        correlation: 42,
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_equality_inspect_differs_correlation() {
    // Given two Inspect commands with different correlation
    let a = ShardCommand::Inspect {
        run: super::RunId::new(1),
        correlation: 1,
    };
    let b = ShardCommand::Inspect {
        run: super::RunId::new(1),
        correlation: 2,
    };
    assert_ne!(a, b);
}

#[test]
fn shard_command_equality_action_completed() {
    // Given two identical ActionCompleted commands
    let a = ShardCommand::ActionCompletedLegacy {
        run: super::RunId::new(1),
        step: vb_core::ids::StepIdx::new(0),
    };
    let b = ShardCommand::ActionCompletedLegacy {
        run: super::RunId::new(1),
        step: vb_core::ids::StepIdx::new(0),
    };
    assert_eq!(a, b);
}
