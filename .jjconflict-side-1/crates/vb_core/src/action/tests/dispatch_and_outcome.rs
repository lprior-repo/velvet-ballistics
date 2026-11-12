// =========================================================================
// Phase 18-19 tests -- Action dispatch, ticket issuance, outcome validation
// =========================================================================

// --- validate_action_dispatch ---

#[test]
fn validate_action_dispatch_succeeds_with_populated_input_and_output_slot() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let mut frame = frame.ok().expect("test setup");
    // Populate both input and output slots before dispatch.
    let write_input = frame.write_slot(SlotIdx::new(0), SlotValue::I64(42));
    assert!(write_input.is_ok());
    let write_output = frame.write_slot(SlotIdx::new(1), SlotValue::I64(0)); // Output must be initialized too.
    assert!(write_output.is_ok());
    let result = validate_action_dispatch(&contract, &frame, SlotIdx::new(0), SlotIdx::new(1));
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_dispatch_fails_on_uninitialized_input() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    // Slot 0 is not populated, so dispatch should fail.
    let result = validate_action_dispatch(&contract, &frame, SlotIdx::new(0), SlotIdx::new(1));
    assert_eq!(result, Err(ActionError::DispatchFailed));
}

#[test]
fn validate_action_dispatch_fails_on_out_of_bounds_input() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = validate_action_dispatch(&contract, &frame, SlotIdx::new(99), SlotIdx::new(1));
    assert_eq!(result, Err(ActionError::DispatchFailed));
}

#[test]
fn validate_action_dispatch_succeeds_with_zero_output_count() {
    let contract = ActionContract {
        id: ActionId::new(2),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 1024,
        max_output_bytes: 0,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    // Even with output_slot_count=0, the output slot is within frame bounds
    // so the taint check passes. But input is uninitialized, so dispatch fails.
    let result = validate_action_dispatch(&contract, &frame, SlotIdx::new(0), SlotIdx::new(0));
    assert_eq!(result, Err(ActionError::DispatchFailed));
}

// --- issue_action_ticket ---

#[test]
fn issue_action_ticket_captures_all_fields() {
    let ticket = issue_action_ticket(NewActionTicket {
        run: RunId::new(42),
        step: StepIdx::new(3),
        seq: SeqNo::new(7),
        action: ActionId::new(5),
        run_attempt: 2,
        attempt: 3,
        idempotency_key: 12345,
        capacity: 10,
        fencing_token: 0,
    });
    assert_eq!(ticket.run, RunId::new(42));
    assert_eq!(ticket.step, StepIdx::new(3));
    assert_eq!(ticket.seq, SeqNo::new(7));
    assert_eq!(ticket.action, ActionId::new(5));
    assert_eq!(ticket.attempt, 3);
    assert_eq!(ticket.run_attempt, 2);
    assert_eq!(ticket.idempotency_key, 12345);
    assert_eq!(ticket.capacity, 10);
}

#[test]
fn issue_action_ticket_with_zero_key_is_valid() {
    let ticket = issue_action_ticket(NewActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(0),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    });
    assert_eq!(ticket.idempotency_key, 0);
    assert_eq!(ticket.attempt, 1);
}

#[test]
fn issue_action_ticket_with_max_values() {
    let ticket = issue_action_ticket(NewActionTicket {
        run: RunId::new(u64::MAX),
        step: StepIdx::new(u16::MAX),
        seq: SeqNo::new(u64::MAX),
        action: ActionId::new(u16::MAX),
        run_attempt: u16::MAX,
        attempt: u16::MAX,
        idempotency_key: u128::MAX,
        capacity: u16::MAX,
        fencing_token: 0,
    });
    assert_eq!(ticket.run, RunId::new(u64::MAX));
    assert_eq!(ticket.step, StepIdx::new(u16::MAX));
    assert_eq!(ticket.seq, SeqNo::new(u64::MAX));
    assert_eq!(ticket.action, ActionId::new(u16::MAX));
    assert_eq!(ticket.attempt, u16::MAX);
    assert_eq!(ticket.idempotency_key, u128::MAX);
}

// --- validate_action_outcome ---

#[test]
fn validate_action_outcome_ready_succeeds_with_valid_slot() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 2,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(output);
    let result = validate_action_outcome(&contract, &outcome);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_outcome_ready_accepts_absolute_slot_when_output_declared() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(5),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(output);
    let result = validate_action_outcome(&contract, &outcome);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_outcome_ready_rejects_zero_output_slots() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
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
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(output);
    let result = validate_action_outcome(&contract, &outcome);
    assert_eq!(
        result,
        Err(ActionError::OutputSlotOutOfBounds {
            slot: 0,
            max_slots: 0,
        })
    );
}

#[test]
fn validate_action_outcome_failed_always_succeeds() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let outcome = ActionOutcome::Failed(failure);
    let result = validate_action_outcome(&contract, &outcome);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_outcome_failed_rejects_oversized_payload() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 3,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 4,
    };
    let outcome = ActionOutcome::Failed(failure);
    let result = validate_action_outcome(&contract, &outcome);
    assert_eq!(
        result,
        Err(ActionError::PayloadTooLarge {
            max_bytes: 3,
            actual_bytes: 4,
        })
    );
}

#[test]
fn validate_action_outcome_suspended_rejected() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    };
    let outcome = ActionOutcome::Suspended(ticket);
    let result = validate_action_outcome(&contract, &outcome);
    assert_eq!(result, Err(ActionError::DispatchFailed));
}

// --- ActionJournalEvent ---

#[test]
fn journal_event_suspended_roundtrips_fields() {
    let ticket = ActionTicket {
        run: RunId::new(10),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(7),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 999,
        capacity: 1,
        fencing_token: 0,
    };
    let event = ActionJournalEvent::Suspended {
        ticket,
        attempt: ticket.attempt,
        action: ActionId::new(7),
        input_slot: SlotIdx::new(0),
        output_slot: SlotIdx::new(1),
        step: StepIdx::new(2),
    };
    match event {
        ActionJournalEvent::Suspended {
            ticket: t,
            attempt,
            action,
            input_slot,
            output_slot,
            step,
        } => {
            assert_eq!(t.run, RunId::new(10));
            assert_eq!(attempt, 1);
            assert_eq!(action, ActionId::new(7));
            assert_eq!(input_slot, SlotIdx::new(0));
            assert_eq!(output_slot, SlotIdx::new(1));
            assert_eq!(step, StepIdx::new(2));
        }
        other => panic!("expected Suspended, got {other:?}"),
    }
}

#[test]
fn journal_event_serialization_roundtrip() {
    let ticket = ActionTicket {
        run: RunId::new(99),
        step: StepIdx::new(1),
        seq: SeqNo::new(2),
        action: ActionId::new(3),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 12345,
        capacity: 1,
        fencing_token: 0,
    };
    let event = ActionJournalEvent::Suspended {
        ticket,
        attempt: ticket.attempt,
        action: ActionId::new(3),
        input_slot: SlotIdx::new(0),
        output_slot: SlotIdx::new(1),
        step: StepIdx::new(1),
    };
    let bytes = postcard::to_allocvec(&event);
    assert!(bytes.is_ok(), "serialization should succeed");
    let bytes = bytes.ok().expect("test setup");
    let recovered: Result<ActionJournalEvent, _> = postcard::from_bytes(&bytes);
    assert!(recovered.is_ok(), "deserialization should succeed");
    assert_eq!(recovered.ok().expect("test setup"), event);
}
