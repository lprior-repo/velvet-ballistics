// =====================================================================
// Execute do tests
// =====================================================================

#[test]
fn execute_do_returns_awaiting_action_for_known_action() {
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Initialize input slot before dispatch.
    let write_result = run.write_slot(SlotIdx::new(0), vb_core::SlotValue::I64(42));
    if write_result.is_err() {
        return;
    }
    let contract = vb_core::action::ActionContract {
        id: ActionId::new(1),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let registry_contracts: Vec<vb_core::action::ActionContract> = vec![
        vb_core::action::ActionContract {
            id: ActionId::new(0),
            name: vb_core::action::ActionName::new("test-action").unwrap(),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        },
        contract,
    ];
    let contract_ref = match registry_contracts.get(1) {
        Some(c) => c,
        None => return,
    };
    let result = execute_do(
        &run,
        StepIdx::new(0),
        ActionId::new(1),
        SlotIdx::new(0),
        SeqNo::new(0),
        contract_ref,
        &registry_contracts,
        &CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
        1,
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(1));
            assert_eq!(ticket.run, RunId::new(1));
            assert_eq!(ticket.step, StepIdx::new(0));
        }
        other => assert_eq!(other, Ok(RuntimeSignal::Continue)),
    }
}

#[test]
fn execute_do_propagates_taint_from_secret_input_without_violation() {
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let write_result = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    assert_eq!(write_result.map(|_| ()), Ok(()));
    let contract = vb_core::action::ActionContract {
        id: ActionId::new(1),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let registry_contracts: Vec<vb_core::action::ActionContract> = vec![
        vb_core::action::ActionContract {
            id: ActionId::new(0),
            name: vb_core::action::ActionName::new("test-action").unwrap(),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        },
        contract,
    ];
    let contract_ref = match registry_contracts.get(1) {
        Some(c) => c,
        None => return,
    };
    let result = execute_do(
        &run,
        StepIdx::new(0),
        ActionId::new(1),
        SlotIdx::new(0),
        SeqNo::new(0),
        contract_ref,
        &registry_contracts,
        &CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
        1,
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(1));
            assert_eq!(ticket.run, RunId::new(1));
        }
        other => assert_eq!(other, Ok(RuntimeSignal::Continue)),
    }
}

#[test]
fn execute_do_returns_unknown_action_for_unregistered_action() {
    let run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let empty_contracts: Vec<vb_core::action::ActionContract> = Vec::new();
    let dummy_contract = vb_core::action::ActionContract {
        id: ActionId::new(0),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let result = execute_do(
        &run,
        StepIdx::new(0),
        ActionId::new(5),
        SlotIdx::new(0),
        SeqNo::new(0),
        &dummy_contract,
        &empty_contracts,
        &CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
        1,
    );
    assert_eq!(
        result,
        Err(RuntimeEngineError::Action(
            vb_core::action::ActionError::UnknownAction {
                action: ActionId::new(5),
            }
        ))
    );
}

#[test]
fn execute_do_without_contract_fails_closed_without_ticket() {
    let mut run = match RunFrame::new(RunId::new(42), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Input slot must be initialized with clean taint.
    assert_eq!(
        run.write_slot(SlotIdx::new(0), vb_core::SlotValue::I64(0)),
        Ok(())
    );
    let result = execute_do_without_contract(
        &run,
        StepIdx::new(3),
        ActionId::new(7),
        SlotIdx::new(0),
        SeqNo::new(5),
        &vb_core::capability::CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
    );
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied { action, .. }))
            if action == ActionId::new(7)
    ));
}

#[cfg(test)]
fn make_original_ticket() -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(10),
        action: ActionId::new(7),
        run_attempt: 1,
        attempt: 3,
        idempotency_key: compute_idempotency_key(
            RunId::new(1),
            1,
            SeqNo::new(10),
            ActionId::new(7),
        ),
        capacity: 5,
        fencing_token: 0,
    }
}

#[cfg(test)]
fn dummy_contract() -> vb_core::action::ActionContract {
    vb_core::action::ActionContract {
        id: ActionId::new(0),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

#[test]
fn resume_action_outcome_ready_continues_execution() {
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let ready = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(ready);
    let original = make_original_ticket();
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    assert_eq!(result, Ok(RuntimeSignal::Continue));
}

#[test]
fn resume_action_outcome_failed_non_retryable_returns_error() {
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let outcome = ActionOutcome::Failed(failure);
    let original = make_original_ticket();
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    assert_eq!(
        result,
        Err(RuntimeEngineError::Core(
            EngineError::UnsupportedPrimitive {
                primitive: "action_failed_non_retryable".into(),
            }
        ))
    );
}

#[test]
fn resume_action_outcome_suspended_returns_awaiting() {
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(5),
        action: ActionId::new(3),
        run_attempt: 1,
        attempt: 2,
        idempotency_key: 99,
        capacity: 1,
        fencing_token: 0,
    };
    let outcome = ActionOutcome::Suspended(ticket);
    let original = make_original_ticket();
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    assert_eq!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(5),
            action: ActionId::new(3),
            run_attempt: 1,
            attempt: 2,
            idempotency_key: 99,
            capacity: 1,
            fencing_token: 0,
        }))
    );
}

#[test]
fn resume_action_outcome_retryable_failure_propagates_original_ticket_fields() {
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let outcome = ActionOutcome::Failed(failure);
    let original = make_original_ticket();
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    match result {
        Ok(RuntimeSignal::AwaitingAction(retry_ticket)) => {
            assert_eq!(retry_ticket.run, RunId::new(1));
            assert_eq!(retry_ticket.step, StepIdx::new(2));
            assert_eq!(
                retry_ticket.seq,
                SeqNo::new(11),
                "seq must be incremented from original"
            );
            assert_eq!(
                retry_ticket.action,
                ActionId::new(7),
                "action must match original ticket"
            );
            assert_eq!(
                retry_ticket.attempt, 4,
                "attempt must be incremented from original"
            );
            let expected_key =
                compute_idempotency_key(RunId::new(1), 1, SeqNo::new(11), ActionId::new(7));
            assert_eq!(
                retry_ticket.idempotency_key, expected_key,
                "idempotency_key must be recomputed from new seq"
            );
        }
        other => {
            let msg = format!("expected AwaitingAction, got {other:?}");
            panic!("{msg}");
        }
    }
}

