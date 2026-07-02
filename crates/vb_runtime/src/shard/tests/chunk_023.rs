
#[test]
fn shard_submit_with_run_id_one_accepted() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(1);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn shard_config_default_has_expected_values() {
    let config = ShardConfig::default();
    assert_eq!(config.command_queue_capacity, 1024);
    assert_eq!(config.trace_capacity, 4096);
    assert_eq!(config.step_budget_per_tick, 1000);
    assert_eq!(config.max_active_runs, 1024);
}

#[test]
fn shard_command_timer_fired_equality() {
    let deadline = std::time::Instant::now();
    let a = ShardCommand::TimerFired {
        run: super::RunId::new(1),
        generation: 1,
        deadline,
        kind: PendingTimerKind::Wait, logical_deadline: None,
    };
    let b = ShardCommand::TimerFired {
        run: super::RunId::new(1),
        generation: 1,
        deadline,
        kind: PendingTimerKind::Wait, logical_deadline: None,
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_timer_fired_inequality() {
    let deadline = std::time::Instant::now();
    let a = ShardCommand::TimerFired {
        run: super::RunId::new(1),
        generation: 1,
        deadline,
        kind: PendingTimerKind::Wait, logical_deadline: None,
    };
    let b = ShardCommand::TimerFired {
        run: super::RunId::new(2),
        generation: 1,
        deadline,
        kind: PendingTimerKind::Wait, logical_deadline: None,
    };
    assert_ne!(a, b);
}

#[test]
fn shard_command_resume_equality() {
    let a = ShardCommand::Resume {
        run: super::RunId::new(5),
    };
    let b = ShardCommand::Resume {
        run: super::RunId::new(5),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_resume_inequality() {
    let a = ShardCommand::Resume {
        run: super::RunId::new(1),
    };
    let b = ShardCommand::Resume {
        run: super::RunId::new(2),
    };
    assert_ne!(a, b);
}

#[test]
fn shard_command_inspect_equality() {
    let a = ShardCommand::Inspect {
        run: super::RunId::new(3),
        correlation: 42,
    };
    let b = ShardCommand::Inspect {
        run: super::RunId::new(3),
        correlation: 42,
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_inspect_inequality_different_correlation() {
    let a = ShardCommand::Inspect {
        run: super::RunId::new(3),
        correlation: 1,
    };
    let b = ShardCommand::Inspect {
        run: super::RunId::new(3),
        correlation: 2,
    };
    assert_ne!(a, b);
}

#[test]
fn shard_command_shutdown_equality() {
    assert_eq!(ShardCommand::Shutdown, ShardCommand::Shutdown);
}

#[test]
fn shard_command_action_completed_legacy_equality() {
    let a = ShardCommand::ActionCompletedLegacy {
        run: super::RunId::new(7),
        step: vb_core::ids::StepIdx::new(2),
    };
    let b = ShardCommand::ActionCompletedLegacy {
        run: super::RunId::new(7),
        step: vb_core::ids::StepIdx::new(2),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_action_completed_legacy_inequality() {
    let a = ShardCommand::ActionCompletedLegacy {
        run: super::RunId::new(7),
        step: vb_core::ids::StepIdx::new(2),
    };
    let b = ShardCommand::ActionCompletedLegacy {
        run: super::RunId::new(7),
        step: vb_core::ids::StepIdx::new(3),
    };
    assert_ne!(a, b);
}

#[test]
fn inspect_response_not_found_equality_same_run_correlation() {
    let a = InspectResponse::NotFound {
        run: super::RunId::new(5),
        correlation: 10,
    };
    let b = InspectResponse::NotFound {
        run: super::RunId::new(5),
        correlation: 10,
    };
    assert_eq!(a, b);
}

#[test]
fn inspect_response_not_found_inequality_different_correlation() {
    let a = InspectResponse::NotFound {
        run: super::RunId::new(5),
        correlation: 10,
    };
    let b = InspectResponse::NotFound {
        run: super::RunId::new(5),
        correlation: 20,
    };
    assert_ne!(a, b);
}

#[test]
fn shard_tick_counts_zero_initially() {
    let config = small_config();
    let shard = Shard::new(config);
    let snap = shard.counters().snapshot();
    assert_eq!(snap.runs_submitted, 0);
    assert_eq!(snap.runs_completed, 0);
    assert_eq!(snap.runs_failed, 0);
    assert_eq!(snap.steps_executed, 0);
}

#[test]
fn shard_submit_with_inputs_completes_finished_workflow() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let inputs = Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::I64(99))]);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run: super::RunId::new(42),
            workflow,
            inputs,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}

#[test]
fn shard_multiple_submits_complete() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(0),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(workflow) = finished_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(workflow) = finished_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(2),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 3);
    assert_eq!(shard.counters().snapshot().runs_completed, 3);
}

#[test]
fn shard_command_variants_cross_inequality() {
    let cancel = ShardCommand::Cancel {
        run: super::RunId::new(1),
    reason: None};
    let resume = ShardCommand::Resume {
        run: super::RunId::new(1),
    };
    let timer = ShardCommand::TimerFired {
        run: super::RunId::new(1),
        generation: 1,
        deadline: std::time::Instant::now(),
        kind: PendingTimerKind::Wait, logical_deadline: None,
    };
    let shutdown = ShardCommand::Shutdown;
    assert_ne!(cancel, resume);
    assert_ne!(cancel, timer);
    assert_ne!(cancel, shutdown);
    assert_ne!(resume, timer);
    assert_ne!(resume, shutdown);
    assert_ne!(timer, shutdown);
}

#[test]
fn ask_ticket_copy_semantics() {
    let a = AskTicket {
        run: super::RunId::new(5),
        ask_step: vb_core::ids::StepIdx::new(1),
        resume_step: vb_core::ids::StepIdx::new(2),
    };
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn inspect_snapshot_debug_format() {
    let snap = InspectSnapshot {
        run: super::RunId::new(1),
        correlation: 0,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 0,
    };
    let debug = format!("{snap:?}");
    assert!(
        debug.contains("InspectSnapshot"),
        "Debug should contain InspectSnapshot: {debug}"
    );
}
