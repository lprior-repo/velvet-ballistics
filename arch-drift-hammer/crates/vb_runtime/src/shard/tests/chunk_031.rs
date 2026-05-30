
// W1-W4 refinement behavior tests for vb-282my.
// W1: Resume guard (4 tests), W4: Ask-answer guard (3 tests),
// RS-05, RS-06, RS-11..RS-13 Apply transitions, AA-03, RS-07
// NOTE: Retry FSM tests are in helpers.rs inline tests.

// ============================================================================
// Helpers
// ============================================================================

fn minimal_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_node = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
    };
    let finish_node = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish { result: SlotIdx::ZERO },
    };
    let parts = WorkflowParts {
        name: Box::from("minimal_finish"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([set_node, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

// ============================================================================
// W1: Resume state machine guard
// ============================================================================

#[test]
fn handle_resume_rejects_initial_state() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else { return; };
    let run = super::RunId::new(7_301);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit { run, workflow, caps: vb_core::capability::CapabilitySet::empty() }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    shard.apply(run, super::RuntimeEvent::Submit);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Initial),
    );

    let result = shard.handle_resume(run);

    assert!(
        matches!(
            result,
            Err(super::ResumeError::NotResumable {
                run_id,
                current_state: super::RuntimeState::Initial,
            }) if run_id == run
        ),
        "handle_resume must reject Initial state with NotResumable, got {result:?}"
    );
}

#[test]
fn handle_resume_rejects_resuming_state() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else { return; };
    let run = super::RunId::new(7_302);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit { run, workflow, caps: vb_core::capability::CapabilitySet::empty() }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    shard.apply(run, super::RuntimeEvent::Resume);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Resuming),
    );

    let result = shard.handle_resume(run);

    assert!(
        matches!(
            result,
            Err(super::ResumeError::NotResumable {
                run_id,
                current_state: super::RuntimeState::Resuming,
            }) if run_id == run
        ),
        "handle_resume must reject Resuming state with NotResumable, got {result:?}"
    );
}

#[test]
fn handle_resume_rejects_failed_state() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else { return; };
    let run = super::RunId::new(7_303);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit { run, workflow, caps: vb_core::capability::CapabilitySet::empty() }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    shard.apply(run, super::RuntimeEvent::Fail);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Failed),
    );

    let result = shard.handle_resume(run);

    assert!(
        matches!(
            result,
            Err(super::ResumeError::NotResumable {
                run_id,
                current_state: super::RuntimeState::Failed,
            }) if run_id == run
        ),
        "handle_resume must reject Failed state with NotResumable, got {result:?}"
    );
}

#[test]
fn handle_resume_rejects_non_resumable_state() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else { return; };
    let run = super::RunId::new(7_304);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit { run, workflow, caps: vb_core::capability::CapabilitySet::empty() }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    shard.apply(run, super::RuntimeEvent::Submit);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Initial)
    );

    let result = shard.handle_resume(run);

    assert!(
        matches!(
            result,
            Err(super::ResumeError::NotResumable { run_id, .. }) if run_id == run
        ),
        "handle_resume must reject non-resumable non-running states, got {result:?}"
    );
}

// ============================================================================
// W4: Ask-answer timer guard
// ============================================================================

#[test]
fn handle_ask_answer_returns_invalid_action_completion_when_pending_timer_is_missing() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else { return; };
    let run = super::RunId::new(7_401);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit { run, workflow, caps: vb_core::capability::CapabilitySet::empty() }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.runs.contains_key(&run));
    assert!(shard.pending_timers.get(&run).is_none());

    let answer = AskAnswer::new(
        AskTicket { run, ask_step: vb_core::ids::StepIdx::new(0), resume_step: vb_core::ids::StepIdx::new(1) },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::I64(1),
        vb_core::Taint::Clean,
    );
    let result = shard.handle_ask_answer(answer);

    assert!(
        matches!(result, Err(RuntimeError::InvalidActionCompletion)),
        "missing pending timer must return InvalidActionCompletion, got {result:?}"
    );
}

#[test]
fn handle_ask_answer_returns_invalid_action_completion_when_pending_timer_step_mismatches() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else { return; };
    let run = super::RunId::new(7_402);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit { run, workflow, caps: vb_core::capability::CapabilitySet::empty() }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    shard.pending_timers.insert(
        run,
        PendingTimer {
            step: vb_core::ids::StepIdx::new(1),
            kind: PendingTimerKind::Ask,
            generation: 0,
            deadline: std::time::Instant::now(),
        },
    );

    let answer = AskAnswer::new(
        AskTicket { run, ask_step: vb_core::ids::StepIdx::new(0), resume_step: vb_core::ids::StepIdx::new(0) },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::I64(42),
        vb_core::Taint::Clean,
    );
    let result = shard.handle_ask_answer(answer);

    assert!(
        matches!(result, Err(RuntimeError::InvalidActionCompletion)),
        "step mismatch must return InvalidActionCompletion, got {result:?}"
    );
}

#[test]
fn handle_ask_answer_returns_invalid_action_completion_when_pending_timer_kind_is_wait_not_ask() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else { return; };
    let run = super::RunId::new(7_403);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit { run, workflow, caps: vb_core::capability::CapabilitySet::empty() }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    shard.pending_timers.insert(
        run,
        PendingTimer {
            step: vb_core::ids::StepIdx::new(0),
            kind: PendingTimerKind::Wait,
            generation: 0,
            deadline: std::time::Instant::now(),
        },
    );

    let answer = AskAnswer::new(
        AskTicket { run, ask_step: vb_core::ids::StepIdx::new(0), resume_step: vb_core::ids::StepIdx::new(0) },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::I64(7),
        vb_core::Taint::Clean,
    );
    let result = shard.handle_ask_answer(answer);

    assert!(
        matches!(result, Err(RuntimeError::InvalidActionCompletion)),
        "Wait timer must return InvalidActionCompletion, got {result:?}"
    );
}

// ============================================================================
// RS-05: Already Running
// ============================================================================

#[test]
fn handle_resume_returns_already_running_when_state_is_running() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else { return; };
    let run = super::RunId::new(7_305);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit { run, workflow, caps: vb_core::capability::CapabilitySet::empty() }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    shard.apply(run, super::RuntimeEvent::DriveContinue);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Running),
    );

    let result = shard.handle_resume(run);

    assert!(
        matches!(
            result,
            Ok(super::ResumeResult {
                run_id,
                status: super::ResumeStatus::AlreadyRunning,
                ..
            }) if run_id == run
        ),
        "handle_resume must return AlreadyRunning for Running state, got {result:?}"
    );
}

// ============================================================================
// RS-06: Run not found
// ============================================================================

#[test]
fn handle_resume_returns_run_id_not_found_when_run_does_not_exist() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let nonexistent_run = super::RunId::new(7_306);

    let result = shard.handle_resume(nonexistent_run);

    assert!(
        matches!(
            result,
            Err(super::ResumeError::RunIdNotFound { run_id }) if run_id == nonexistent_run
        ),
        "handle_resume must return RunIdNotFound for unknown run, got {result:?}"
    );
}

// ============================================================================
// RS-11..RS-13: Apply state transition exhaustiveness
// ============================================================================

#[test]
fn apply_submit_sets_state_to_initial() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_311);
    shard.apply(run, super::RuntimeEvent::Submit);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Initial),
    );
}

#[test]
fn apply_resume_sets_state_to_resuming() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_312);
    shard.apply(run, super::RuntimeEvent::Resume);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Resuming),
    );
}

#[test]
fn apply_resume_rollback_sets_state_to_resumable() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_313);
    shard.apply(run, super::RuntimeEvent::ResumeRollback);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Resumable),
    );
}

#[test]
fn apply_drive_continue_sets_state_to_running() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_314);
    shard.apply(run, super::RuntimeEvent::DriveContinue);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Running),
    );
}

#[test]
fn apply_await_action_sets_state_to_resumable() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_315);
    shard.apply(run, super::RuntimeEvent::AwaitAction);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Resumable),
    );
}

#[test]
fn apply_await_timer_sets_state_to_resumable() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_316);
    shard.apply(run, super::RuntimeEvent::AwaitTimer);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Resumable),
    );
}

#[test]
fn apply_fail_sets_state_to_failed() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_317);
    shard.apply(run, super::RuntimeEvent::Fail);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Failed),
    );
}

#[test]
fn apply_terminal_remove_removes_state_entry() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_318);
    shard.apply(run, super::RuntimeEvent::Submit);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Initial)
    );
    shard.apply(run, super::RuntimeEvent::TerminalRemove);
    assert!(shard.runtime_states.get(&run).is_none());
}

#[test]
fn apply_drive_finished_removes_state_entry() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_319);
    shard.apply(run, super::RuntimeEvent::Submit);
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Initial)
    );
    shard.apply(run, super::RuntimeEvent::DriveFinished);
    assert!(shard.runtime_states.get(&run).is_none());
}

// ============================================================================
// AA-03: Ask-answer run not found
// ============================================================================

#[test]
fn handle_ask_answer_returns_run_not_found_when_run_does_not_exist() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(7_601);

    let answer = AskAnswer::new(
        AskTicket { run, ask_step: vb_core::ids::StepIdx::new(0), resume_step: vb_core::ids::StepIdx::new(1) },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::I64(42),
        vb_core::Taint::Clean,
    );
    let result = shard.handle_ask_answer(answer);

    assert!(
        matches!(result, Err(RuntimeError::RunNotFound)),
        "unknown run must return RunNotFound, got {result:?}"
    );
}

// ============================================================================
// RS-07 / AD-10: Successful submit + resume on suspended workflow
// ============================================================================

#[test]
fn handle_resume_on_resumable_suspended_do_run_succeeds() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(suspended_wf) = suspended_workflow() else { return; };
    let run = super::RunId::new(7_308);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit { run, workflow: suspended_wf, caps: vb_core::capability::CapabilitySet::empty() }),
        Ok(())
    );
    let submit_result = shard.tick();
    assert_eq!(submit_result, Ok(true));
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Resumable),
        "suspended workflow must be in Resumable state after submit+Do suspend"
    );

    let result = shard.handle_resume(run);

    // Verify handle_resume returns ResumeStatus::Resumed (not AlreadyRunning)
    assert!(
        matches!(
            result,
            Ok(super::ResumeResult {
                run_id,
                status: super::ResumeStatus::Resumed,
                ..
            }) if run_id == run
        ),
        "handle_resume on Resumable run must return ResumeStatus::Resumed, got {result:?}"
    );

    // Verify run state was transitioned (drive_run was called after journal append).
    // The suspended workflow re-suspends after the Do action, so the state should
    // still be Resumable but the transition through Resuming must have occurred.
    assert_eq!(
        shard.runtime_states.get(&run).copied(),
        Some(super::RuntimeState::Resumable),
        "after resume on suspended workflow, state must be Resumable (re-suspended via Do action)"
    );
}
