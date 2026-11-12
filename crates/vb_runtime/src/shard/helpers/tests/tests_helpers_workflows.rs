fn ticket(run: RunId, step: StepIdx, attempt: u16) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        run_attempt: 1,
        attempt,
        idempotency_key: 0,
        capacity: u16::MAX,
        fencing_token: 0,
    }
}

// ---- Workflow factories ----

fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let node = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("suspended"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    Some(vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("suspended_workflow"))
}

fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_const = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("finished"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    Some(vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("finished_workflow"))
}

// ---- advance_after_timer_fire rejects terminal WaitUntil (no next) ----

fn wait_workflow_no_next() -> Option<vb_core::workflow::CompiledWorkflow> {
    let wait = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("wait_no_next"),
        digest: WorkflowDigest::from_bytes([0xAA; 32]),
        nodes: Box::from([wait]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    Some(vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("wait_workflow_no_next"))
}

#[test]
fn advance_after_timer_fire_rejects_wait_until_without_next() {
    let Some(wf) = wait_workflow_no_next() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    let result = advance_after_timer_fire(&mut state, timer);
    assert_eq!(result, Err(RuntimeError::InvalidTimerFire));
}

// ---- timer_registration_required for WaitEvent with timeout ----

fn wait_event_with_timeout_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let wait_event = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: Some(SlotIdx::new(1)).into(),
        },
        None,
        Some(StepIdx::new(1)),
    );
    let resume = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("wait_event_timeout"),
        digest: WorkflowDigest::from_bytes([0xBB; 32]),
        nodes: Box::from([wait_event, resume]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10), ConstValue::I64(100)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    Some(
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("wait_event_with_timeout_workflow"),
    )
}

#[test]
fn timer_required_for_wait_event_with_timeout() {
    let Some(wf) = wait_event_with_timeout_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(timer_registration_required(&state, StepIdx::ZERO), true);
}

// ---- timer_registration_required for Ask with timeout ----

fn ask_with_timeout_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let ask = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)).into(),
        },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(1)),
    );
    let resume = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::AskResume {
            answer: SlotIdx::new(2),
        },
        None,
        Some(StepIdx::new(2)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("ask_with_timeout"),
        digest: WorkflowDigest::from_bytes([0xCC; 32]),
        nodes: Box::from([ask, resume, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10), ConstValue::I64(100)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    Some(
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("ask_with_timeout_workflow"),
    )
}

#[test]
fn timer_required_for_ask_with_timeout() {
    let Some(wf) = ask_with_timeout_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(timer_registration_required(&state, StepIdx::ZERO), true);
}

// ---- timer_registration_required for Ask without timeout ----

fn ask_without_timeout_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let ask = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: None.into(),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)),
    );
    let resume = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::AskResume {
            answer: SlotIdx::new(1),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("ask_no_timeout"),
        digest: WorkflowDigest::from_bytes([0xDD; 32]),
        nodes: Box::from([ask, resume]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    Some(
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("ask_without_timeout_workflow"),
    )
}

#[test]
fn timer_not_required_for_ask_without_timeout() {
    let Some(wf) = ask_without_timeout_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(timer_registration_required(&state, StepIdx::ZERO), false);
}

// ---- validate_action_completion accepts running step with matching action ----

#[test]
fn validate_action_completion_accepts_running_step_with_matching_action() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Mark step 0 as running so validate passes
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }

    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_completion_accepts_matching_current_attempt() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 2;
    }

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 2)
        },
    );
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_completion_rejects_stale_attempt_without_state_change() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    let pc_before = state.frame.pc();
    let executed_before = state.frame.executed();
    let frame_before = state.frame.clone();
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 2)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::StaleAttempt {
            incoming: 2,
            current: 3
        })
    );
    assert_eq!(state.action_attempts.get(0).copied(), Some(3));
    assert_eq!(state.frame.pc(), pc_before);
    assert_eq!(state.frame.executed(), executed_before);
    assert_eq!(state.frame, frame_before);
    assert_eq!(
        state.frame.step_state(StepIdx::ZERO),
        Ok(StepState::Running)
    );
}

#[test]
fn validate_action_completion_rejects_zero_attempt() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 0)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 0, max: 3 })
    );
}

#[test]
fn validate_action_completion_rejects_zero_capacity() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 0,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 1, max: 0 })
    );
}

#[test]
fn validate_action_completion_rejects_attempt_beyond_max() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 4)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 })
    );
}

#[test]
fn normalize_scheduled_ticket_promotes_attempts_one_two_three() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let first = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(first.map(|t| t.attempt), Ok(1));

    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 2;
    }
    let second = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(second.map(|t| t.attempt), Ok(2));

    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    let third = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(third.map(|t| t.attempt), Ok(3));
}

#[test]
fn normalize_scheduled_ticket_rejects_zero_capacity_as_attempt_beyond_max() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let result = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 0,
            ..ticket(RunId::new(1), StepIdx::ZERO, 0)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 1, max: 0 })
    );
}

#[test]
fn normalize_scheduled_ticket_rejects_attempt_beyond_max_with_exact_error() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 4;
    }
    let result = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 })
    );
    assert_eq!(state.action_attempts.get(0).copied(), Some(4));
}

// ---- validate_action_completion rejects wrong action id ----

#[test]
fn validate_action_completion_rejects_wrong_action_id() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let t = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(99),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

// ---- advance_after_action_completion advances to next step ----

#[test]
fn advance_after_action_completion_advances_pc_to_next() {
    let Some(wf) = finished_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Step 0 has next = Some(1)
    let result = advance_after_action_completion(&mut state, StepIdx::ZERO);
    assert_eq!(result, Ok(()));
    assert_eq!(state.frame.pc(), StepIdx::new(1));
}

// ---- seed_input_slots rejects out-of-bounds slot ----

#[test]
fn seed_input_slots_rejects_out_of_bounds_slot() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut frame) =
        RunFrame::new(RunId::new(1), wf.entry(), wf.node_count(), wf.slot_count()).ok()
    else {
        return;
    };
    // Slot 99 is out of bounds for a workflow with 1 slot
    let inputs = Box::from([(SlotIdx::new(99), SlotValue::I64(1))]);
    let node = vb_core::workflow::node(
        vb_core::ids::StepIdx::ZERO,
        vb_core::workflow::CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::ZERO,
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: vb_core::ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts).unwrap();
    let result = seed_input_slots(&mut frame, &inputs, &workflow);
    assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration));
}

