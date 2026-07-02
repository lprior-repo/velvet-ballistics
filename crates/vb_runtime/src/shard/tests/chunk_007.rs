#[test]
fn shard_command_equality_timer_fired() {
    // Given two identical TimerFired commands
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
fn shard_command_equality_resume() {
    // Given two identical Resume commands
    let a = ShardCommand::Resume {
        run: super::RunId::new(1),
    };
    let b = ShardCommand::Resume {
        run: super::RunId::new(1),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_cancel_nonexistent_does_not_increment_failed() {
    // Given a shard
    let config = small_config();
    let mut shard = Shard::new(config);
    // When cancelling a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(999),
            reason: None
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    // Then the failed counter is NOT incremented (run didn't exist)
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
}

#[test]
fn shard_finished_workflow_sets_completed_counter() {
    // Given a shard with a finished workflow
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(50);
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
}

#[test]
fn shard_finished_workflow_produces_run_finished_trace() {
    // Given a shard with a finished workflow
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(51);
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
}

#[test]
fn shard_inspect_response_not_found_for_unknown_run() {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config);
    // When inspecting a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: super::RunId::new(999),
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then response is NotFound
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run: super::RunId::new(999),
            correlation: 1
        })
    );
}

#[test]
fn inspect_response_found_equality() {
    // Given two identical Found responses
    let a = InspectResponse::Found(InspectSnapshot {
        run: super::RunId::new(1),
        correlation: 42,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 5,
    });
    let b = InspectResponse::Found(InspectSnapshot {
        run: super::RunId::new(1),
        correlation: 42,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 5,
    });
    assert_eq!(a, b);
}

#[test]
fn inspect_response_found_differs_executed() {
    // Given two Found responses with different executed counts
    let a = InspectResponse::Found(InspectSnapshot {
        run: super::RunId::new(1),
        correlation: 1,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 5,
    });
    let b = InspectResponse::Found(InspectSnapshot {
        run: super::RunId::new(1),
        correlation: 1,
        pc: vb_core::ids::StepIdx::new(0),
        executed: 10,
    });
    assert_ne!(a, b);
}

#[test]
fn inspect_response_not_found_equality() {
    // Given two identical NotFound responses
    let a = InspectResponse::NotFound {
        run: super::RunId::new(1),
        correlation: 42,
    };
    let b = InspectResponse::NotFound {
        run: super::RunId::new(1),
        correlation: 42,
    };
    assert_eq!(a, b);
}

#[test]
fn run_state_equality() {
    // Given a suspended workflow and run frame
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let frame = match vb_core::frame::RunFrame::new(
        super::RunId::new(1),
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
        action_attempts: super::new_action_attempts(4),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    };
    let frame2 = match vb_core::frame::RunFrame::new(
        super::RunId::new(1),
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
        action_attempts: super::new_action_attempts(4),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    };
    assert_eq!(state, state2);
}

// =======================================================================
// Adversarial BDD tests — shard
// =======================================================================

#[test]
fn shard_cancel_then_inspect_returns_not_found() {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(200);
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
    // Then inspect returns NotFound
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run,
            correlation: 1
        })
    );
}

#[test]
fn adversarial_shard_action_failed_for_unknown_run_returns_run_not_found() {
    // Given a shard with no runs
    let config = small_config();
    let mut shard = Shard::new(config);
    // When failing an action for a non-existent run
    let ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(999),
        step: vb_core::ids::StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
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
}
