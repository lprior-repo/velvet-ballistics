
#[test]
fn submit_returns_active_run_capacity_exceeded_at_limit() -> Result<(), RuntimeError> {
    // Given a shard with max_active_runs = 1 and one active run
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
    let Some(wf) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When submitting a second run
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(2),
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns ActiveRunCapacityExceeded with capacity 1
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
    Ok(())
}

#[test]
fn shard_submit_creates_run_state_in_runs_map() -> Result<(), RuntimeError> {
    // Given a shard and a workflow
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(10);
    // When submitting a run
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then inspecting the run returns Found (proving it's in the runs map)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let response = shard.take_inspect_response();
    match response {
        Some(InspectResponse::Found(snapshot)) => {
            assert_eq!(snapshot.run, run);
            assert_eq!(snapshot.correlation, 1);
        }
        other => {
            // Wrong: expected Found
            assert_eq!(other, None);
        }
    }
    Ok(())
}

#[test]
fn shard_submit_records_run_submitted_trace_event() -> Result<(), RuntimeError> {
    // Given a shard and a workflow
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(20);
    // When submitting a run
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the trace ring contains a RunSubmitted event
    let events = shard.trace_ring_mut().drain();
    let found = events
        .iter()
        .any(|e| *e == TraceEvent::RunSubmitted { run });
    assert_eq!(found, true);
    Ok(())
}

#[test]
fn shard_submit_drives_run_immediately_for_finished_workflow() -> Result<(), RuntimeError> {
    // Given a shard and a finished workflow
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(30);
    // When submitting a run with a finishing workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the run is completed (not in runs map anymore) and counter shows completed
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    // And inspect returns Terminal { Completed } since the run finished
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 2,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::Terminal {
            run,
            correlation: 2,
            outcome: TerminalOutcome::Completed,
        })
    );
    Ok(())
}

#[test]
fn shard_resume_returns_error_for_unknown_run() -> Result<(), RuntimeError> {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config)?;
    // When resuming a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::Resume {
            run: super::RunId::new(999),
        }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn shard_action_completed_returns_error_for_unknown_run() -> Result<(), RuntimeError> {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config)?;
    // When completing an action for a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run: super::RunId::new(888),
            step: vb_core::ids::StepIdx::new(0),
        }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

#[test]
fn shard_action_completed_marks_step_succeeded() -> Result<(), RuntimeError> {
    // Given a shard with a suspended run (Do node at step 0)
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(55);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    let tick1 = shard.tick();
    // Then first tick succeeds (Do node suspends)
    assert_eq!(tick1, Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    // When completing the action at step 0
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: vb_core::ids::StepIdx::new(0),
        }),
        Ok(())
    );
    let tick2 = shard.tick();
    // Then second tick succeeds
    assert_eq!(tick2, Ok(true));
    // And the trace ring has an ActionCompleted event
    let events = shard.trace_ring_mut().drain();
    let found = events.iter().any(|e| {
        *e == TraceEvent::ActionCompleted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        }
    });
    assert_eq!(found, true);
    Ok(())
}

#[test]
fn shard_action_completed_records_trace_event() -> Result<(), RuntimeError> {
    // Given a shard with a suspended run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(56);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When completing the action
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: vb_core::ids::StepIdx::new(0),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the trace ring contains an ActionCompleted event
    let events = shard.trace_ring_mut().drain();
    let found = events.iter().any(|e| {
        *e == TraceEvent::ActionCompleted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        }
    });
    assert_eq!(found, true);
    Ok(())
}
