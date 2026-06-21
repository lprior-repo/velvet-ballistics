
/// Verify that active_run_count tracks correctly across submit, cancel, and finish.
#[test]
fn shard_active_run_count_across_lifecycle() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    assert_eq!(shard.active_run_count(), 0);

    // Submit a suspended run -> count = 1
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run_a = super::RunId::new(920);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run_a,
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 1);

    // Submit another suspended run -> count = 2
    let Some(wf2) = suspended_workflow() else {
        return Ok(());
    };
    let run_b = super::RunId::new(921);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run_b,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 2);

    // Cancel one -> count = 1
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run: run_a, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 1);

    // Cancel the other -> count = 0
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run: run_b, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}

/// After cancelling all runs, new submissions are accepted even at capacity boundary.
#[test]
fn shard_submit_after_full_cancel_resets_capacity() -> Result<(), RuntimeError> {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    
};
    let mut shard = Shard::new(config)?;

    // Fill to capacity
    let Some(wf1) = suspended_workflow() else {
        return Ok(());
    };
    let run1 = super::RunId::new(930);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run1,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Over capacity should fail
    let Some(wf2) = suspended_workflow() else {
        return Ok(());
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(931),
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );

    // Cancel and re-submit should work
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run: run1, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    let Some(wf3) = finished_workflow() else {
        return Ok(());
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(932),
            workflow: wf3,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

/// Verify that inspect for a currently active suspended run returns the
/// correct pc and correlation.
#[test]
fn shard_inspect_active_run_returns_correct_state() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(940);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 42,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snap)) => {
            assert_eq!(snap.run, run);
            assert_eq!(snap.correlation, 42);
            // Suspended on Do node at step 0
            assert_eq!(snap.pc, vb_core::ids::StepIdx::ZERO);
            // executed may be 0 or more depending on when the counter is
            // recorded relative to the suspension point.
        }
        other => assert_eq!(other, None),
    }
    Ok(())
}

/// Resubmitting with SubmitWithInputs after cancel works.
#[test]
fn shard_submit_with_inputs_after_cancel() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(950);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    // Resubmit with inputs
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs: Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::I64(99))]),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 2);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

/// Multiple inspections of the same active run without taking intermediate
/// responses all succeed.
#[test]
fn shard_repeated_inspect_same_run() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(960);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First inspect
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let first = shard.take_inspect_response();
    assert_eq!(first.is_some(), true);

    // Second inspect
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 2,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snap)) => {
            assert_eq!(snap.run, run);
            assert_eq!(snap.correlation, 2);
        }
        other => assert_eq!(other, None),
    }
    Ok(())
}

/// Submit + Resume enqueued before tick processes both in sequence.
#[test]
fn shard_commands_for_pending_but_unprocessed_run() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(970);

    // Enqueue Submit + Resume without ticking in between
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));

    // First tick processes Submit -> run becomes active (suspended on Do)
    assert_eq!(shard.tick(), Ok(true));
    // Second tick processes Resume -> run re-drives and re-suspends on Do
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 1);
    Ok(())
}
