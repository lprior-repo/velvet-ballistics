#[test]
fn action_error_payload_too_large_reports_exact_overflow() {
    let error = ActionError::PayloadTooLarge {
        max_bytes: 1024,
        actual_bytes: 2048,
    };
    match error {
        ActionError::PayloadTooLarge {
            max_bytes,
            actual_bytes,
        } => {
            assert_eq!(max_bytes, 1024);
            assert_eq!(actual_bytes, 2048);
        }
        other => assert_eq!(
            other,
            ActionError::PayloadTooLarge {
                max_bytes: 1024,
                actual_bytes: 2048,
            }
        ),
    }
}

#[test]
fn action_error_output_slot_out_of_bounds_reports_exact_boundary() {
    let error = ActionError::OutputSlotOutOfBounds {
        slot: 10,
        max_slots: 4,
    };
    match error {
        ActionError::OutputSlotOutOfBounds { slot, max_slots } => {
            assert_eq!(slot, 10);
            assert_eq!(max_slots, 4);
        }
        other => assert_eq!(
            other,
            ActionError::OutputSlotOutOfBounds {
                slot: 10,
                max_slots: 4,
            }
        ),
    }
}

#[test]
fn action_contract_with_zero_output_bytes_is_constructable() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    assert_eq!(contract.max_output_bytes, 0);
    assert_eq!(contract.output_slot_count, 0);
}

#[test]
fn action_contract_with_zero_timeout_is_constructable() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 0,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    assert_eq!(contract.timeout_ms, 0);
}

#[test]
fn action_output_ready_carries_secret_taint_without_downgrade() {
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Secret,
        encoded_len: 8,
    };
    assert_eq!(output.taint, Taint::Secret);
}

#[test]
fn action_failure_with_retryable_policy_is_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert_eq!(failure.retry_policy, RetryPolicy::Retryable);
}

#[test]
fn action_outcome_suspended_carries_ticket_identity() {
    let ticket = ActionTicket {
        run: RunId::new(42),
        step: StepIdx::new(3),
        seq: SeqNo::new(7),
        action: ActionId::new(1),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 999,
        capacity: 1,
        fencing_token: 0,
    };
    let outcome = ActionOutcome::Suspended(ticket);
    match outcome {
        ActionOutcome::Suspended(t) => {
            assert_eq!(t.run, RunId::new(42));
            assert_eq!(t.idempotency_key, 999);
        }
        other => assert_eq!(other, ActionOutcome::Suspended(ticket)),
    }
}

#[test]
fn action_failure_code_repr_values_are_distinct() {
    use std::collections::BTreeSet;
    let codes = [
        ActionFailureCode::Rejected,
        ActionFailureCode::Timeout,
        ActionFailureCode::RateLimited,
        ActionFailureCode::ResourceExhausted,
        ActionFailureCode::ExternalUnavailable,
        ActionFailureCode::InvalidInput,
        ActionFailureCode::PermissionDenied,
        ActionFailureCode::Conflict,
        ActionFailureCode::Unknown,
    ];
    let reprs: BTreeSet<u8> = codes.iter().map(|c| failure_code_repr(*c)).collect();
    assert_eq!(reprs.len(), codes.len());
}

fn failure_code_repr(code: ActionFailureCode) -> u8 {
    match code {
        ActionFailureCode::Rejected => 0,
        ActionFailureCode::Timeout => 1,
        ActionFailureCode::RateLimited => 2,
        ActionFailureCode::ResourceExhausted => 3,
        ActionFailureCode::ExternalUnavailable => 4,
        ActionFailureCode::InvalidInput => 5,
        ActionFailureCode::PermissionDenied => 6,
        ActionFailureCode::Conflict => 7,
        ActionFailureCode::Unknown => 255,
    }
}

// =========================================================================
// Phase 38 tests -- SideEffect, RetrySafety, IdempotencyViolation
// =========================================================================

#[test]
fn side_effect_repr_values_are_distinct() {
    let effects = [
        SideEffect::None,
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ];
    let mut reprs: [u8; 5] = [0; 5];
    let mut count = 0;
    for effect in &effects {
        let repr = side_effect_repr(*effect);
        reprs[count] = repr;
        count = match count.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    let mut i = 0;
    while i < count {
        let mut j = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
        while j < count {
            assert_ne!(reprs[i], reprs[j], "duplicate repr at {i} and {j}");
            j = match j.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    assert_eq!(count, 5);
}

fn side_effect_repr(effect: SideEffect) -> u8 {
    match effect {
        SideEffect::None => 0,
        SideEffect::Writes => 1,
        SideEffect::Sends => 2,
        SideEffect::Creates => 3,
        SideEffect::Destroys => 4,
    }
}

#[test]
fn retry_safety_repr_values_are_distinct() {
    let safeties = [
        RetrySafety::Safe,
        RetrySafety::KeyRequired,
        RetrySafety::Unsafe,
    ];
    let repr_a = retry_safety_repr(safeties[0]);
    let repr_b = retry_safety_repr(safeties[1]);
    let repr_c = retry_safety_repr(safeties[2]);
    assert_ne!(repr_a, repr_b);
    assert_ne!(repr_b, repr_c);
    assert_ne!(repr_a, repr_c);
}

fn retry_safety_repr(safety: RetrySafety) -> u8 {
    match safety {
        RetrySafety::Safe => 0,
        RetrySafety::KeyRequired => 1,
        RetrySafety::Unsafe => 2,
    }
}

#[test]
fn idempotency_violation_missing_key_carries_side_effect() {
    let violation = IdempotencyViolation::MissingKey(SideEffect::Writes);
    match violation {
        IdempotencyViolation::MissingKey(eff) => assert_eq!(eff, SideEffect::Writes),
        other => panic!("expected MissingKey, got {other:?}"),
    }
}

#[test]
fn idempotency_violation_secret_in_key_carries_slot() {
    let violation = IdempotencyViolation::SecretInKey(7);
    match violation {
        IdempotencyViolation::SecretInKey(slot) => assert_eq!(slot, 7),
        other => panic!("expected SecretInKey, got {other:?}"),
    }
}

#[test]
fn idempotency_violation_random_in_key_carries_slot() {
    let violation = IdempotencyViolation::RandomInKey(3);
    match violation {
        IdempotencyViolation::RandomInKey(slot) => assert_eq!(slot, 3),
        other => panic!("expected RandomInKey, got {other:?}"),
    }
}

#[test]
fn idempotency_violation_time_in_key_carries_slot() {
    let violation = IdempotencyViolation::TimeInKey(5);
    match violation {
        IdempotencyViolation::TimeInKey(slot) => assert_eq!(slot, 5),
        other => panic!("expected TimeInKey, got {other:?}"),
    }
}

#[test]
fn verify_idempotency_pure_action_always_passes() {
    let action = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 1,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_idempotency_safe_action_with_side_effect_passes() {
    let action = ActionContract {
        id: ActionId::new(2),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_idempotency_unsafe_action_rejected() {
    let action = ActionContract {
        id: ActionId::new(3),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Destroys,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::Destroys))
    );
}

#[test]
fn verify_idempotency_key_required_empty_keys_rejected() {
    let action = ActionContract {
        id: ActionId::new(4),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::Writes))
    );
}

#[test]
fn verify_idempotency_key_required_clean_keys_passes() {
    let action = ActionContract {
        id: ActionId::new(5),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let mut frame = frame.ok().expect("test setup");
    assert!(
        frame
            .write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean)
            .is_ok()
    );
    assert!(
        frame
            .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Clean)
            .is_ok()
    );
    let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
    let result = verify_idempotency(&action, &key_slots, &frame);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_idempotency_key_required_secret_key_rejected() {
    let action = ActionContract {
        id: ActionId::new(6),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let mut frame = frame.ok().expect("test setup");
    let write_result =
        frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret);
    assert!(write_result.is_ok());
    let key_slots = [SlotIdx::new(0)];
    let result = verify_idempotency(&action, &key_slots, &frame);
    assert_eq!(result, Err(IdempotencyViolation::SecretInKey(0)));
}
