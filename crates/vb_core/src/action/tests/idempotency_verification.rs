#[test]
fn validate_key_ingredients_clean_slots_pass() {
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
    let result = validate_idempotency_key_ingredients(&key_slots, &frame);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_key_ingredients_uninitialized_slot_rejected() {
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let key_slots = [SlotIdx::new(0)];
    let result = validate_idempotency_key_ingredients(&key_slots, &frame);
    assert_eq!(result, Err(IdempotencyViolation::UnavailableKeySlot(0)));
}

#[test]
fn validate_key_ingredients_out_of_bounds_slot_rejected() {
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 1);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let key_slots = [SlotIdx::new(2)];
    let result = validate_idempotency_key_ingredients(&key_slots, &frame);
    assert_eq!(result, Err(IdempotencyViolation::UnavailableKeySlot(2)));
}

#[test]
fn validate_key_ingredients_derived_secret_rejected() {
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let mut frame = frame.ok().expect("test setup");
    let write_result = frame.write_slot_with_taint(
        SlotIdx::new(1),
        SlotValue::I64(99),
        Taint::DerivedFromSecret,
    );
    assert!(write_result.is_ok());
    let key_slots = [SlotIdx::new(1)];
    let result = validate_idempotency_key_ingredients(&key_slots, &frame);
    assert_eq!(result, Err(IdempotencyViolation::SecretInKey(1)));
}

#[test]
fn verify_idempotency_sends_side_effect_key_required_rejected_without_key() {
    let action = ActionContract {
        id: ActionId::new(7),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Sends,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::Sends))
    );
}

#[test]
fn verify_idempotency_creates_side_effect_unsafe_rejected() {
    let action = ActionContract {
        id: ActionId::new(8),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Creates,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::Creates))
    );
}

#[test]
fn action_contract_serializes_with_new_fields() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let bytes = postcard::to_allocvec(&contract);
    assert!(bytes.is_ok(), "postcard serialization should succeed");
    let bytes = bytes.ok().expect("test setup");
    let recovered: Result<ActionContract, _> = postcard::from_bytes(&bytes);
    assert!(recovered.is_ok(), "postcard deserialization should succeed");
    let recovered = recovered.ok().expect("test setup");
    assert_eq!(recovered.id, contract.id);
    assert_eq!(recovered.side_effect, contract.side_effect);
    assert_eq!(recovered.retry_safety, contract.retry_safety);
}

#[test]
fn side_effect_is_copy() {
    let a = SideEffect::Writes;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn retry_safety_is_copy() {
    let a = RetrySafety::KeyRequired;
    let b = a;
    assert_eq!(a, b);
}

// =========================================================================
// Phase 38 adversarial tests -- idempotency verification rejection paths
// =========================================================================

#[test]
fn verify_idempotency_writes_with_safe_passes() {
    let action = ActionContract {
        id: ActionId::new(100),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(50), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_idempotency_destroys_with_unsafe_rejected_even_with_keys() {
    let action = ActionContract {
        id: ActionId::new(101),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Destroys,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(51), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    // Even though we supply key slots, Unsafe is always rejected.
    let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
    let result = verify_idempotency(&action, &key_slots, &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::Destroys))
    );
}

#[test]
fn verify_idempotency_destroys_with_unsafe_rejected_without_keys() {
    let action = ActionContract {
        id: ActionId::new(102),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Destroys,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(52), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::Destroys))
    );
}

#[test]
fn verify_idempotency_key_required_rejects_secret_tainted_key_slot() {
    let action = ActionContract {
        id: ActionId::new(103),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(53), StepIdx::new(0), 4, 4);
    assert!(frame.is_ok());
    let mut frame = frame.ok().expect("test setup");
    // Slot 0 has a clean value.
    let write_clean =
        frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean);
    assert!(write_clean.is_ok());
    // Slot 1 has a secret-tainted value.
    let write_secret =
        frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret);
    assert!(write_secret.is_ok());
    // Slot 2 has a derived-from-secret value.
    let write_derived = frame.write_slot_with_taint(
        SlotIdx::new(2),
        SlotValue::I64(30),
        Taint::DerivedFromSecret,
    );
    assert!(write_derived.is_ok());

    // Clean key passes.
    let result_clean = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
    assert_eq!(result_clean, Ok(()));

    // Secret key is rejected.
    let result_secret = verify_idempotency(&action, &[SlotIdx::new(1)], &frame);
    assert_eq!(result_secret, Err(IdempotencyViolation::SecretInKey(1)));

    // DerivedFromSecret key is also rejected.
    let result_derived = verify_idempotency(&action, &[SlotIdx::new(2)], &frame);
    assert_eq!(result_derived, Err(IdempotencyViolation::SecretInKey(2)));
}

#[test]
fn verify_idempotency_key_required_rejects_when_first_slot_clean_but_second_secret() {
    let action = ActionContract {
        id: ActionId::new(104),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Creates,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(54), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let mut frame = frame.ok().expect("test setup");
    let write_clean =
        frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean);
    assert!(write_clean.is_ok());
    let write_secret =
        frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret);
    assert!(write_secret.is_ok());
    // Key slots: [clean, secret]. Should reject on the second slot.
    let result = verify_idempotency(&action, &[SlotIdx::new(0), SlotIdx::new(1)], &frame);
    assert_eq!(result, Err(IdempotencyViolation::SecretInKey(1)));
}

#[test]
fn verify_idempotency_none_side_effect_always_passes_even_unsafe() {
    // Actions with SideEffect::None always pass, regardless of retry_safety.
    let action = ActionContract {
        id: ActionId::new(105),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 1,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(55), StepIdx::new(0), 1, 1);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_idempotency_sends_side_effect_unsafe_rejected() {
    let action = ActionContract {
        id: ActionId::new(106),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Sends,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(56), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::Sends))
    );
}
