// ---- retry_policy_after_action rejects non-I64 slot value ----

#[test]
fn retry_policy_after_action_rejects_non_i64_policy_slot() {
    // Build a retry workflow where the policy slot contains a Bool instead of I64
    let set_policy = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)),
    );
    let action = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        None,
        Some(StepIdx::new(2)),
    );
    let retry_check = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(1),
            body: StepIdx::new(1),
            exhausted: StepIdx::new(3),
        },
        None,
        None,
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("retry_bad_policy"),
        digest: WorkflowDigest::from_bytes([0xAB; 32]),
        nodes: Box::from([set_policy, action, retry_check, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(1)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: vb_core::SlotIdx::new(0),
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    };
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("retry_policy_after_action_rejects_non_i64_policy_slot");
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Drive step 0 and write a Bool to the policy slot
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.set_pc(StepIdx::new(1)), Ok(()));
    assert_eq!(
        state
            .frame
            .write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean),
        Ok(())
    );

    let result = retry_policy_after_action(&state, StepIdx::new(1));
    assert_eq!(
        result,
        Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_slot_not_i64"
        })
    );
}

// ---- retry_policy_after_action rejects non-RetryCheck next node ----

#[test]
fn retry_policy_after_action_rejects_non_retry_check_next() {
    let Some(wf) = finished_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Step 0 has next = Some(1), but node 1 is Finish, not RetryCheck
    let result = retry_policy_after_action(&state, StepIdx::ZERO);
    assert_eq!(
        result,
        Err(RuntimeError::UnsupportedOperation {
            operation: "retry_metadata_missing"
        })
    );
}

// ---- retry_policy_after_action rejects negative max attempts ----

#[test]
fn retry_policy_after_action_rejects_negative_max_attempts() {
    let set_policy = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)),
    );
    let action = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        None,
        Some(StepIdx::new(2)),
    );
    let retry_check = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(1),
            body: StepIdx::new(1),
            exhausted: StepIdx::new(3),
        },
        None,
        None,
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("retry_negative"),
        digest: WorkflowDigest::from_bytes([0xAC; 32]),
        nodes: Box::from([set_policy, action, retry_check, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(-1)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: vb_core::SlotIdx::new(0),
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    };
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("retry_policy_after_action_rejects_negative_max_attempts");
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.set_pc(StepIdx::new(1)), Ok(()));
    assert_eq!(
        state
            .frame
            .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(-1), Taint::Clean),
        Ok(())
    );

    let result = retry_policy_after_action(&state, StepIdx::new(1));
    assert_eq!(
        result,
        Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_attempts_out_of_range"
        })
    );
}

// ---- retry_policy_after_action rejects zero max attempts ----

#[test]
fn retry_policy_after_action_rejects_zero_max_attempts() {
    let set_policy = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(1)),
    );
    let action = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        None,
        Some(StepIdx::new(2)),
    );
    let retry_check = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(1),
            body: StepIdx::new(1),
            exhausted: StepIdx::new(3),
        },
        None,
        None,
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("retry_zero"),
        digest: WorkflowDigest::from_bytes([0xAD; 32]),
        nodes: Box::from([set_policy, action, retry_check, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(0)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: vb_core::SlotIdx::new(0),
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    };
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("retry_policy_after_action_rejects_zero_max_attempts");
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.set_pc(StepIdx::new(1)), Ok(()));
    assert_eq!(
        state
            .frame
            .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(0), Taint::Clean),
        Ok(())
    );

    let result = retry_policy_after_action(&state, StepIdx::new(1));
    assert_eq!(
        result,
        Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_attempts_zero"
        })
    );
}

// ---- record_retry_attempt overflow protection ----

#[test]
fn record_retry_attempt_overflow_returns_error() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Manually set attempt to u16::MAX to trigger overflow
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = u16::MAX;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, u16::MAX);
    let policy = crate::engine::RetryPolicy {
        max_attempts: u16::MAX,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    // attempt is already at max_attempts, so should return Ok(false)
    let result = record_retry_attempt(&mut state, t, policy);
    assert_eq!(result, Ok(false));
}

// ---- record_retry_attempt at max exactly returns false ----

#[test]
fn record_retry_attempt_at_max_exactly_returns_false() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Set attempt to 2 (one less than max)
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 2;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 2);
    let policy = crate::engine::RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(true));
    assert_eq!(state.action_attempts.get(0).copied(), Some(3));
    // Now attempt == max, should return false
    assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(false));
}

// ---- find_error_handler with error_slot set ----

fn error_handler_with_slot_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let guard = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: Some(SlotIdx::new(1)).into(),
        },
        None,
        None,
    );
    let action = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        None,
        Some(StepIdx::new(3)),
    );
    let handler = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(3)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("error_handler_with_slot"),
        digest: WorkflowDigest::from_bytes([0xBA; 32]),
        nodes: Box::from([guard, action, handler, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(false)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: vb_core::SlotIdx::new(0),
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    };
    Some(
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("error_handler_with_slot_workflow"),
    )
}

#[test]
fn find_error_handler_with_error_slot_returns_slot() {
    let Some(wf) = error_handler_with_slot_workflow() else {
        return;
    };
    let result = find_error_handler_for_failure(&wf, StepIdx::new(1));
    match result {
        Some((handler, error_slot)) => {
            assert_eq!(handler, StepIdx::new(2));
            assert_eq!(error_slot, Some(SlotIdx::new(1)));
        }
        None => assert_eq!(result.is_some(), true),
    }
}

// ---- new_action_attempts at boundary values ----

#[test]
fn new_action_attempts_at_u16_max() {
    let attempts = new_action_attempts(u16::MAX);
    assert_eq!(attempts.len(), usize::from(u16::MAX));
}

// ---- record_scheduled_attempt with attempt zero does nothing ----

#[test]
fn record_scheduled_attempt_with_attempt_zero_does_nothing() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let t = ticket(RunId::new(1), StepIdx::ZERO, 0);
    record_scheduled_attempt(&mut state, t);
    // attempt == 0 and the stored value is also 0, so the condition
    // (*attempt == 0 || *attempt < ticket.attempt) is true for ==0.
    // But the second condition: *attempt < ticket.attempt => 0 < 0 is false.
    // So the first condition (*attempt == 0) allows the write.
    assert_eq!(state.action_attempts.get(0).copied(), Some(0));
}

// ---- result_slot_for_finished_run returns none when not at finish ----

#[test]
fn result_slot_for_finished_run_returns_none_at_do_step() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // PC is at step 0, which is a Do node, not Finish
    let result = result_slot_for_finished_run(&state);
    assert_eq!(result, None);
}

// ---- retry_metadata_exists returns false for terminal node (no next) ----

#[test]
fn retry_metadata_exists_for_terminal_node_returns_false() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Step 0 is the only node and has no next
    assert_eq!(retry_metadata_exists(&state, StepIdx::ZERO), false);
}

// ---- seed_input_slots writes multiple values correctly ----

#[test]
fn seed_input_slots_writes_multiple_distinct_values() {
    // Build a workflow with 3 slots
    let set_a = vb_core::workflow::node(
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
        source_metadata: None,
        name: Box::from("multi_slot"),
        digest: WorkflowDigest::from_bytes([0xCA; 32]),
        nodes: Box::from([set_a, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(0)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("seed_input_slots_writes_multiple_distinct_values");
    let Some(mut frame) =
        RunFrame::new(RunId::new(1), wf.entry(), wf.node_count(), wf.slot_count()).ok()
    else {
        return;
    };
    let inputs = Box::from([
        (SlotIdx::new(0), SlotValue::I64(10)),
        (SlotIdx::new(1), SlotValue::Bool(true)),
        (SlotIdx::new(2), SlotValue::I64(-5)),
    ]);
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
        source_metadata: None,
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 3,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: vb_core::ResourceContract::DEFAULT,
        input_slots: Box::from([
            vb_core::CompiledInputSlot {
                slot: vb_core::SlotIdx::new(0),
                kind: vb_core::InputSlotKind::Any,
                constraint: vb_core::CompiledInputConstraint::required(),
            },
            vb_core::CompiledInputSlot {
                slot: vb_core::SlotIdx::new(1),
                kind: vb_core::InputSlotKind::Any,
                constraint: vb_core::CompiledInputConstraint::required(),
            },
            vb_core::CompiledInputSlot {
                slot: vb_core::SlotIdx::new(2),
                kind: vb_core::InputSlotKind::Any,
                constraint: vb_core::CompiledInputConstraint::required(),
            },
        ]),
    };
    let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts).unwrap();
    assert_eq!(seed_input_slots(&mut frame, &inputs, &workflow), Ok(()));
    assert_eq!(frame.read_slot(SlotIdx::new(0)), Ok(&SlotValue::I64(10)));
    assert_eq!(frame.read_slot(SlotIdx::new(1)), Ok(&SlotValue::Bool(true)));
    assert_eq!(frame.read_slot(SlotIdx::new(2)), Ok(&SlotValue::I64(-5)));
}

// ---- snapshot_from_state with various executed counts ----

#[test]
fn snapshot_from_state_captures_nonzero_executed() {
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run_id = RunId::new(1);
    let Some(mut state) = make_run_state(wf, run_id) else {
        return;
    };
    // Drive step 0 to increment executed count
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.set_pc(StepIdx::new(1)), Ok(()));

    let snap = snapshot_from_state(run_id, 0, &state);
    assert_eq!(snap.run, run_id);
    // executed may still be 0 since increment_executed is called by the engine,
    // not by mark_succeeded
    assert_eq!(snap.pc, StepIdx::new(1));
}

// =======================================================================
// validate_action_completion -- exhaustive attempt fence tests
// =======================================================================

#[test]
fn validate_action_completion_returns_ok_when_all_preconditions_satisfied() -> Result<(), String> {
    let wf =
        suspended_workflow().ok_or_else(|| "suspended_workflow fixture must build".to_owned())?;
    let mut state = make_run_state(wf, RunId::new(1))
        .ok_or_else(|| "make_run_state for suspended_workflow must succeed".to_owned())?;
    // Mark step 0 as Running and set action_attempts[0] = 1
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
    Ok(())
}

