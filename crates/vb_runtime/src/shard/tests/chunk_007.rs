#[test]
fn shard_command_equality_timer_fired() -> Result<(), RuntimeError> {
    // Given two identical TimerFired commands
    let deadline = std::time::Instant::now();
    let a = ShardCommand::TimerFired {
        run: RunId::new(1),
        generation: 1,
        deadline,
        kind: PendingTimerKind::Wait,
    };
    let b = ShardCommand::TimerFired {
        run: RunId::new(1),
        generation: 1,
        deadline,
        kind: PendingTimerKind::Wait,
    };
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn shard_command_equality_resume() -> Result<(), RuntimeError> {
    // Given two identical Resume commands
    let a = ShardCommand::Resume {
        run: RunId::new(1),
    };
    let b = ShardCommand::Resume {
        run: RunId::new(1),
    };
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn shard_cancel_nonexistent_does_not_increment_failed() -> Result<(), RuntimeError> {
    // Given a shard
    let config = small_config();
    let mut shard = Shard::new(config)?;
    // When cancelling a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: RunId::new(999),
            reason: None
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    // Then the failed counter is NOT incremented (run didn't exist)
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    Ok(())
}

#[test]
fn shard_finished_workflow_sets_completed_counter() -> Result<(), RuntimeError> {
    // Given a shard with a finished workflow
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(50);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then completed counter is 1
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    Ok(())
}

#[test]
fn shard_finished_workflow_produces_run_finished_trace() -> Result<(), RuntimeError> {
    // Given a shard with a finished workflow
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(51);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the trace contains RunFinished
    let events = shard.trace_ring_mut().drain();
    let found = events.iter().any(|e| *e == TraceEvent::RunFinished { run });
    assert_eq!(found, true);
    Ok(())
}

#[test]
fn shard_inspect_response_not_found_for_unknown_run() -> Result<(), RuntimeError> {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config)?;
    // When inspecting a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: RunId::new(999),
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then response is NotFound
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run: RunId::new(999),
            correlation: 1
        })
    );
    Ok(())
}

#[test]
fn inspect_response_found_equality() -> Result<(), RuntimeError> {
    // Given two identical Found responses
    let a = InspectResponse::Found(InspectSnapshot {
        run: RunId::new(1),
        correlation: 42,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 5,
    });
    let b = InspectResponse::Found(InspectSnapshot {
        run: RunId::new(1),
        correlation: 42,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 5,
    });
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn inspect_response_found_differs_executed() -> Result<(), RuntimeError> {
    // Given two Found responses with different executed counts
    let a = InspectResponse::Found(InspectSnapshot {
        run: RunId::new(1),
        correlation: 1,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 5,
    });
    let b = InspectResponse::Found(InspectSnapshot {
        run: RunId::new(1),
        correlation: 1,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 10,
    });
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn inspect_response_not_found_equality() -> Result<(), RuntimeError> {
    // Given two identical NotFound responses
    let a = InspectResponse::NotFound {
        run: RunId::new(1),
        correlation: 42,
    };
    let b = InspectResponse::NotFound {
        run: RunId::new(1),
        correlation: 42,
    };
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn run_state_equality() -> Result<(), RuntimeError> {
    // Given a suspended workflow and run frame
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let frame = match vb_core::frame::RunFrame::new(
        RunId::new(1),
        vb_core::ids::StepIdx::ZERO,
        4,
        1,
    ) {
        Ok(f) => f,
        Err(_) => return,
    };
    let state = RunState {
        frame,
        workflow: wf.clone(),
        store: vb_core::value_store::ValueStore::new(),
        action_attempts: new_action_attempts(4),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
        last_snapshot_executed: 0,
    };
    let frame2 = match vb_core::frame::RunFrame::new(
        RunId::new(1),
        vb_core::ids::StepIdx::ZERO,
        4,
        1,
    ) {
        Ok(f) => f,
        Err(_) => return,
    };
    let state2 = RunState {
        frame: frame2,
        workflow: wf,
        store: vb_core::value_store::ValueStore::new(),
        action_attempts: new_action_attempts(4),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
        last_snapshot_executed: 0,
    };
    assert_eq!(state, state2);
    Ok(())
}

// =======================================================================
// Adversarial BDD tests — shard
// =======================================================================

#[test]
fn shard_cancel_then_inspect_returns_terminal_cancelled() -> Result<(), RuntimeError> {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(200);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling then inspecting
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then inspect returns Terminal { Cancelled } (post-mortem observability)
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::Terminal {
            run,
            correlation: 1,
            outcome: TerminalOutcome::Cancelled,
        })
    );
    Ok(())
}

#[test]
fn snapshot_run_returns_cancelled_status_for_terminal_cancelled_run() -> Result<(), RuntimeError> {
    // Given a shard with a cancelled run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(201);
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
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When snapshotting the cancelled run
    let response = shard.snapshot_run(run, 7);
    // Then it returns Terminal { Cancelled }, not NotFound
    match response {
        InspectResponse::Terminal {
            run: r,
            correlation,
            outcome,
        } => {
            assert_eq!(r, run);
            assert_eq!(correlation, 7);
            assert_eq!(outcome, TerminalOutcome::Cancelled);
        }
        InspectResponse::NotFound { .. } => {
            panic!("expected Terminal Cancelled, got NotFound");
        }
        InspectResponse::Found(_) => {
            panic!("expected Terminal Cancelled, got Found");
        }
        InspectResponse::Tombstoned { .. } => {
            panic!("expected Terminal Cancelled, got Tombstoned");
        }
    }
    Ok(())
}

#[test]
fn snapshot_run_returns_killed_status_for_terminal_killed_run() -> Result<(), RuntimeError> {
    // Given a shard with a killed run
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(202);
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
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When snapshotting the killed run
    let response = shard.snapshot_run(run, 11);
    // Then it returns Terminal { Killed }, not NotFound
    match response {
        InspectResponse::Terminal {
            run: r,
            correlation,
            outcome,
        } => {
            assert_eq!(r, run);
            assert_eq!(correlation, 11);
            assert_eq!(outcome, TerminalOutcome::Killed);
        }
        InspectResponse::NotFound { .. } => {
            panic!("expected Terminal Killed, got NotFound");
        }
        InspectResponse::Found(_) => {
            panic!("expected Terminal Killed, got Found");
        }
        InspectResponse::Tombstoned { .. } => {
            panic!("expected Terminal Killed, got Tombstoned");
        }
    }
    Ok(())
}

#[test]
fn snapshot_run_still_returns_found_for_active_run() -> Result<(), RuntimeError> {
    // Regression guard: active runs must still return Found after the
    // terminal_runs-first branch was added.
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(203);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let response = shard.snapshot_run(run, 3);
    match response {
        InspectResponse::Found(snap) => {
            assert_eq!(snap.run, run);
            assert_eq!(snap.correlation, 3);
        }
        InspectResponse::Terminal { .. } => {
            panic!("expected Found for active run, got Terminal");
        }
        InspectResponse::NotFound { .. } => {
            panic!("expected Found for active run, got NotFound");
        }
        InspectResponse::Tombstoned { .. } => {
            panic!("expected Found for active run, got Tombstoned");
        }
    }
    Ok(())
}

#[test]
fn adversarial_shard_action_failed_for_unknown_run_returns_run_not_found() -> Result<(), RuntimeError> {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config)?;
    // When failing an action for a non-existent run
    let ticket = vb_core::action::ActionTicket {
        run: RunId::new(999),
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
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}
