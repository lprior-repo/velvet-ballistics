// Tests for error semantics across handler groups.
//
// These tests verify error handling behavior for:
// - Resume error mapping (PO-vb-pymh-016)
// - Action completion errors
// - Timer fire errors
// - Various RuntimeError variants

// suspended_workflow() is defined in chunk_001.rs
// small_config() is defined in chunk_003.rs

/// Workflow with an Ask.
fn ask_workflow_for_error_tests() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let ask = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: None,
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
        name: Box::from("ask"),
        digest: WorkflowDigest::from_bytes([13; 32]),
        nodes: Box::from([set_prompt, ask, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1))]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

/// Workflow with a Wait.
fn wait_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
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
        name: Box::from("wait"),
        digest: WorkflowDigest::from_bytes([14; 32]),
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

// ============================================================================
// Resume Error Semantics (PO-vb-pymh-016)
// ============================================================================

/// Test resume_run_not_found_error: Resume non-existent run → Err(RunNotFound).
#[test]
fn resume_nonexistent_run_returns_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;

    assert_eq!(
        shard.enqueue(ShardCommand::Resume {
            run: RunId::new(999),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

/// Test resume_not_resumable_error: Resume run in non-Resumable state.
/// The suspended_workflow has a Do action that suspends the run into
/// `Resumable` state after the first tick. To exercise the NotResumable
/// FSM contract, force the run into `Running` via runtime_state_insert
/// before enqueueing the resume command.
#[test]
fn resume_active_run_returns_error() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(500);

    // Submit the workflow and drive it once so the run is tracked.
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Force the run into Running state to exercise the NotResumable FSM
    // contract via the resume path. Production handle_resume rejects any
    // state that is not Resumable or Resuming.
    shard.runtime_state_insert(run, RuntimeState::Running);

    assert_eq!(
        shard.enqueue(ShardCommand::Resume { run }),
        Ok(())
    );
    let result = shard.tick();
    // Result must surface NotResumable for the Running run (FSM RQ-W0-07)
    assert!(
        matches!(result, Err(RuntimeError::NotResumable { .. })),
        "resume of Running run must surface NotResumable, got {result:?}"
    );
    // And the run must remain present in the shard's runtime state map
    assert!(
        shard.run_state_contains(run),
        "run must still exist in shard state after failed resume"
    );
    Ok(())
}

// ============================================================================
// Action Completion Errors
// ============================================================================

/// Test action_completion_run_not_found_error: Completion for vanished run → Err(RunNotFound).
#[test]
fn action_completion_for_nonexistent_run_returns_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;

    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run: RunId::new(888),
            step: StepIdx::ZERO,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

// ============================================================================
// Timer Fire Errors
// ============================================================================

/// Test timer_no_pending_timer_error: No pending timer for run → Err(InvalidTimerFire).
#[test]
fn timer_fire_without_pending_timer_returns_error() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(520);

    // Submit - creates an action-suspended run, not a timer-suspended run
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Try to fire a timer for this run
    let command = ShardCommand::TimerFired {
        run,
        generation: 1,
        deadline: std::time::Instant::now(),
        kind: PendingTimerKind::Wait,
    };

    assert_eq!(shard.enqueue(command), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

/// Test timer_generation_mismatch_error: Wrong generation → Err(InvalidTimerFire).
#[test]
fn timer_fire_with_wrong_generation_returns_error() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = wait_workflow() else {
        return Ok(());
    };
    let run = RunId::new(521);

    // Submit - creates a Wait timer
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Get the pending timer
    let pending = shard.pending_timer_get(run).expect("should have timer");
    let correct_generation = pending.generation;

    // Fire with wrong generation
    let command = ShardCommand::TimerFired {
        run,
        generation: correct_generation + 1, // Wrong generation
        deadline: pending.deadline,
        kind: pending.kind,
    };

    assert_eq!(shard.enqueue(command), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

/// Test timer_kind_mismatch_error: Wait vs Ask kind mismatch → Err(InvalidTimerFire).
#[test]
fn timer_fire_with_wrong_kind_returns_error() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = wait_workflow() else {
        return Ok(());
    };
    let run = RunId::new(522);

    // Submit - creates a Wait timer
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Get the pending timer
    let pending = shard.pending_timer_get(run).expect("should have timer");

    // Fire with wrong kind (Ask instead of Wait)
    let command = ShardCommand::TimerFired {
        run,
        generation: pending.generation,
        deadline: pending.deadline,
        kind: PendingTimerKind::Ask, // Wrong kind
    };

    assert_eq!(shard.enqueue(command), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

/// Test timer_wait_kind_resolves_wait: Valid Wait timer advances state.
#[test]
fn valid_wait_timer_fire_advances_state() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = wait_workflow() else {
        return Ok(());
    };
    let run = RunId::new(523);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Timer should be pending
    assert!(shard.pending_timer_get(run).is_some());

    // Fire the timer
    let pending = shard.pending_timer_get(run).unwrap();
    let command = ShardCommand::TimerFired {
        run,
        generation: pending.generation,
        deadline: pending.deadline,
        kind: pending.kind,
    };

    assert_eq!(shard.enqueue(command), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    // Timer should be cleared and run should complete
    assert!(shard.pending_timer_get(run).is_none());
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

/// Test timer_ask_kind_clears_pending_timer: Valid Ask timer removes pending timer.
#[test]
fn valid_ask_timer_fire_clears_pending_timer() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = ask_workflow_for_error_tests() else {
        return Ok(());
    };
    let run = RunId::new(524);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Timer should be pending (Ask kind)
    let pending = shard.pending_timer_get(run).expect("should have ask timer");
    assert_eq!(pending.kind, PendingTimerKind::Ask);

    // Fire the timer
    let command = ShardCommand::TimerFired {
        run,
        generation: pending.generation,
        deadline: pending.deadline,
        kind: pending.kind,
    };

    assert_eq!(shard.enqueue(command), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    // Timer should be cleared
    assert!(shard.pending_timer_get(run).is_none());
    Ok(())
}

// ============================================================================
// AskAnswer Error Semantics
// ============================================================================

/// Test ask_answer_timer_authority_mismatch_error: Step/kind mismatch → Err(InvalidActionCompletion).
#[test]
fn ask_answer_with_wrong_step_returns_error() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = ask_workflow_for_error_tests() else {
        return Ok(());
    };
    let run = RunId::new(530);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let _pending = shard.pending_timer_get(run).expect("should have ask timer");

    // Create answer with wrong ask_step
    let answer = AskAnswer::with_encoded_len(
        AskTicket {
            run,
            ask_step: StepIdx::new(99), // Wrong step
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        vb_core::value::Taint::Clean,
        10,
    );

    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered { answer }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    Ok(())
}

// ============================================================================
// Inspect Error Semantics
// ============================================================================

/// Test inspect_correlation_preserved: Correlation ID passed through to response.
#[test]
fn inspect_preserves_correlation_id() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(540);
    let correlation = 42u64;

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
        shard.enqueue(ShardCommand::Inspect { run, correlation }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snapshot)) => {
            assert_eq!(snapshot.correlation, correlation);
        }
        other => assert_eq!(other.is_some(), true),
    }
    Ok(())
}
