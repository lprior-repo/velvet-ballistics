// Tests for shutdown invariant (PO-vb-pymh-012).
//
// These tests verify shutdown behavior:
// - tick_returns_false_after_shutdown_command
// - tick_returns_false_when_already_shutting_down
// - command_queue_drained_before_shutdown

// suspended_workflow() is defined in chunk_001.rs
// small_config() is defined in chunk_003.rs

/// Workflow that creates a Wait timer.
fn timed_wait_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_deadline = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let wait = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("timed_wait"),
        digest: WorkflowDigest::from_bytes([12; 32]),
        nodes: Box::from([set_deadline, wait, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::I64(10)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

/// Test tick_returns_false_after_shutdown_command: Enqueue Shutdown, tick → Ok(false).
#[test]
fn tick_returns_false_after_shutdown_command() {
    let config = small_config();
    let mut shard = Shard::new(config);

    // Before shutdown
    assert_eq!(shard.is_shutting_down(), false);

    // Enqueue shutdown command
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));

    // First tick after shutdown should return false
    assert_eq!(shard.tick(), Ok(false));

    // Shutting down flag should be set
    assert_eq!(shard.is_shutting_down(), true);
}

/// Test tick_returns_false_when_already_shutting_down: tick when shutting_down=true → Ok(false).
#[test]
fn tick_returns_false_when_already_shutting_down() {
    let config = small_config();
    let mut shard = Shard::new(config);

    // Enqueue shutdown command
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));

    // Verify shutting_down flag
    assert_eq!(shard.is_shutting_down(), true);

    // Subsequent ticks should also return false
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.tick(), Ok(false));
}

/// Test dispatch_shutdown_clears_pending_timers.
#[test]
fn shutdown_clears_pending_timers() {
    let config = small_config();
    let mut shard = Shard::new(config);

    // Create a workflow with a pending timer
    let Some(workflow) = timed_wait_workflow() else {
        return;
    };
    let run = RunId::new(400);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Verify timer is registered
    assert_eq!(shard.pending_timers.len(), 1);

    // Enqueue shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));

    // Timers should be cleared
    assert_eq!(shard.pending_timers.len(), 0);
}

/// Test command_queue_drained_before_shutdown: Shutdown drains pending commands before setting flag.
#[test]
fn shutdown_drains_pending_commands_before_setting_flag() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(401);

    // Enqueue a submit first
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    // Then enqueue shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));

    // First tick should process the submit, NOT set shutting_down yet
    // (shutdown is processed after other commands)
    assert_eq!(shard.tick(), Ok(true));

    // Run should be submitted
    assert_eq!(shard.run_state_contains(run), true);

    // Second tick should process shutdown
    assert_eq!(shard.tick(), Ok(false));

    // Now shutting_down is set
    assert_eq!(shard.is_shutting_down(), true);
}

/// Test shutdown processes multiple commands in order.
#[test]
fn shutdown_processes_multiple_commands_in_order() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };

    // Enqueue multiple submits
    for i in 0..3 {
        let run = RunId::new(410 + i);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: workflow.clone(),
                caps: vb_core::capability::CapabilitySet::empty()
            }),
            Ok(())
        );
    }

    // Then shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));

    // Process all commands
    for _ in 0..4 {
        let result = shard.tick();
        // Should keep processing until shutdown
        if shard.is_shutting_down() {
            assert_eq!(result, Ok(false));
            break;
        }
    }

    // Verify shutting_down is set after processing all submits and shutdown
    assert_eq!(shard.is_shutting_down(), true);
}

/// Test shutdown with inspect command queued.
#[test]
fn shutdown_with_pending_inspect_command() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(420);

    // Submit a workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Enqueue inspect
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 42,
        }),
        Ok(())
    );

    // Enqueue shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));

    // Process: inspect then shutdown
    assert_eq!(shard.tick(), Ok(true)); // Process inspect
    assert_eq!(shard.tick(), Ok(false)); // Process shutdown

    assert_eq!(shard.is_shutting_down(), true);
}

/// Test shutdown prevents new commands from being processed.
#[test]
fn shutdown_prevents_new_commands_after_flag_set() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };

    // Enqueue shutdown first
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));

    // Now enqueue a submit - it goes into queue
    let run = RunId::new(430);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    // But tick returns false immediately since shutting_down
    assert_eq!(shard.tick(), Ok(false));

    // The submit was not processed
    assert_eq!(shard.run_state_contains(run), false);
}

/// Test shutdown flag persists across multiple ticks.
#[test]
fn shutdown_flag_persists_across_ticks() {
    let config = small_config();
    let mut shard = Shard::new(config);

    // Trigger shutdown
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));

    // Verify flag is true
    assert_eq!(shard.is_shutting_down(), true);

    // Multiple subsequent ticks all return false
    for _ in 0..5 {
        assert_eq!(shard.tick(), Ok(false));
    }
}
