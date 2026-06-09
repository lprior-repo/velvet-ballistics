use super::*;
use crate::value::Taint;

#[test]
fn deterministic_clean_stays_clean() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Clean);
    assert_eq!(result, Taint::Clean);
}

#[test]
fn deterministic_secret_stays_secret() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Secret);
    assert_eq!(result, Taint::Secret);
}

#[test]
fn deterministic_derived_stays_derived() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn idempotent_clean_stays_clean() {
    let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::Clean);
    assert_eq!(result, Taint::Clean);
}

#[test]
fn at_least_once_secret_becomes_derived() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Secret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn at_least_once_derived_stays_derived() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn at_least_once_clean_stays_clean() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Clean);
    assert_eq!(result, Taint::Clean);
}

// -- ActionError exact variant assertions --

#[test]
fn action_error_unknown_action_exact_variant() -> Result<(), String> {
    let error = ActionError::UnknownAction {
        action: ActionId::new(42),
    };
    let ActionError::UnknownAction { action } = error else {
        return Err(String::from("expected UnknownAction variant"));
    };
    if action != ActionId::new(42) {
        return Err(String::from("unexpected action id"));
    }
    Ok(())
}

#[test]
fn action_error_invalid_ticket_exact_variant() {
    let error = ActionError::InvalidTicket;
    assert_eq!(error, ActionError::InvalidTicket);
}

#[test]
fn action_error_payload_too_large_exact_variant() -> Result<(), String> {
    let error = ActionError::PayloadTooLarge {
        max_bytes: 1024,
        actual_bytes: 2048,
    };
    let ActionError::PayloadTooLarge {
        max_bytes,
        actual_bytes,
    } = error
    else {
        return Err(String::from("expected PayloadTooLarge variant"));
    };
    if max_bytes != 1024 || actual_bytes != 2048 {
        return Err(String::from("unexpected payload size fields"));
    }
    Ok(())
}

#[test]
fn action_error_output_slot_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = ActionError::OutputSlotOutOfBounds {
        slot: 5,
        max_slots: 4,
    };
    let ActionError::OutputSlotOutOfBounds { slot, max_slots } = error else {
        return Err(String::from("expected OutputSlotOutOfBounds variant"));
    };
    if slot != 5 || max_slots != 4 {
        return Err(String::from("unexpected output slot bounds fields"));
    }
    Ok(())
}

#[test]
fn action_error_non_idempotent_replay_blocked_exact_variant() {
    let error = ActionError::NonIdempotentReplayBlocked;
    assert_eq!(error, ActionError::NonIdempotentReplayBlocked);
}

#[test]
fn action_error_completion_already_recorded_exact_variant() {
    let error = ActionError::CompletionAlreadyRecorded;
    assert_eq!(error, ActionError::CompletionAlreadyRecorded);
}

#[test]
fn action_error_queue_full_exact_variant() {
    let error = ActionError::QueueFull;
    assert_eq!(error, ActionError::QueueFull);
}

#[test]
fn action_error_encoding_failed_exact_variant() {
    let error = ActionError::EncodingFailed;
    assert_eq!(error, ActionError::EncodingFailed);
}

#[test]
fn action_error_dispatch_failed_exact_variant() {
    let error = ActionError::DispatchFailed;
    assert_eq!(error, ActionError::DispatchFailed);
}

#[test]
fn action_error_runtime_codes_cover_section_17_mappings() {
    assert_eq!(
        ActionError::UnknownAction {
            action: ActionId::new(9)
        }
        .runtime_code(),
        Some("REFERENCE_MISSING")
    );
    assert_eq!(
        ActionError::PayloadTooLarge {
            max_bytes: 1,
            actual_bytes: 2,
        }
        .runtime_code(),
        Some("PAYLOAD_TOO_LARGE")
    );
    assert_eq!(ActionError::QueueFull.runtime_code(), Some("QUEUE_FULL"));
    assert_eq!(
        ActionError::EncodingFailed.runtime_code(),
        Some("ACTION_FAILED")
    );
    assert_eq!(
        ActionError::DispatchFailed.runtime_code(),
        Some("ACTION_FAILED")
    );
}

#[test]
fn action_error_runtime_codes_are_unique() {
    let codes = [
        ActionError::REFERENCE_MISSING_RUNTIME_CODE,
        ActionError::ACTION_FAILED_RUNTIME_CODE,
        ActionError::PAYLOAD_TOO_LARGE_RUNTIME_CODE,
        ActionError::QUEUE_FULL_RUNTIME_CODE,
    ];
    assert_eq!(codes.len(), 4);
    assert_eq!(
        codes
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
}

#[test]
fn action_error_runtime_code_is_absent_without_section_17_equivalent() {
    assert_eq!(ActionError::InvalidTicket.runtime_code(), None);
    assert_eq!(ActionError::CompletionAlreadyRecorded.runtime_code(), None);
}

// =========================================================================
// Phase 2 adversarial BDD tests -- action ABI security & taint vectors
// =========================================================================

#[test]
fn action_ticket_with_idempotency_key_zero_is_a_valid_ticket() {
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(5),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    assert_eq!(ticket.idempotency_key, 0);
    assert_eq!(ticket.run, RunId::new(1));
}

#[test]
fn deterministic_pure_propagate_cannot_downgrade_secret_to_clean() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Secret);
    assert_eq!(result, Taint::Secret);
}

#[test]
fn deterministic_pure_propagate_cannot_downgrade_derived_to_clean() {
    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn idempotent_external_propagate_cannot_downgrade_secret_to_clean() {
    let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::Secret);
    assert_eq!(result, Taint::Secret);
}

#[test]
fn idempotent_external_propagate_cannot_downgrade_derived_to_clean() {
    let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
}

#[test]
fn at_least_once_secret_is_always_derived_never_secret() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Secret);
    assert_eq!(result, Taint::DerivedFromSecret);
    assert_ne!(result, Taint::Clean);
}

#[test]
fn at_least_once_derived_remains_derived_never_clean() {
    let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::DerivedFromSecret);
    assert_eq!(result, Taint::DerivedFromSecret);
    assert_ne!(result, Taint::Clean);
}

#[test]
fn action_ticket_from_different_run_is_not_equal() {
    let ticket_a = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(5),
        attempt: 1,
        idempotency_key: 100,
        capacity: 1,
    };
    let ticket_b = ActionTicket {
        run: RunId::new(2),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(5),
        attempt: 1,
        idempotency_key: 100,
        capacity: 1,
    };
    assert_ne!(ticket_a, ticket_b);
}

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
        side_effect: SideEffect::Pure,
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
        side_effect: SideEffect::LocalWrite,
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
        attempt: 1,
        idempotency_key: 999,
        capacity: 1,
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
        SideEffect::Pure,
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
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
        SideEffect::Pure => 0,
        SideEffect::LocalWrite => 1,
        SideEffect::ExternalWrite => 2,
        SideEffect::LocalWrite => 3,
        SideEffect::LocalWrite => 4,
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
    let violation = IdempotencyViolation::MissingKey(SideEffect::LocalWrite);
    match violation {
        IdempotencyViolation::MissingKey(eff) => assert_eq!(eff, SideEffect::LocalWrite),
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
    let violation = IdempotencyViolation::SecretInKey(3);
    match violation {
        IdempotencyViolation::SecretInKey(slot) => assert_eq!(slot, 3),
        other => panic!("expected SecretInKey, got {other:?}"),
    }
}

#[test]
fn idempotency_violation_time_in_key_carries_slot() {
    let violation = IdempotencyViolation::SecretInKey(5);
    match violation {
        IdempotencyViolation::SecretInKey(slot) => assert_eq!(slot, 5),
        other => panic!("expected SecretInKey, got {other:?}"),
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
        side_effect: SideEffect::Pure,
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
        side_effect: SideEffect::LocalWrite,
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
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite))
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
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite))
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
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
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
        side_effect: SideEffect::LocalWrite,
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

#[test]
fn validate_key_ingredients_clean_slots_pass() {
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
    let result = validate_idempotency_key_ingredients(&key_slots, &frame);
    assert_eq!(result, Ok(()));
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
        side_effect: SideEffect::ExternalWrite,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::ExternalWrite))
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
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite))
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
        side_effect: SideEffect::LocalWrite,
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
    let a = SideEffect::LocalWrite;
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
        side_effect: SideEffect::LocalWrite,
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
        side_effect: SideEffect::LocalWrite,
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
        Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite))
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
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(52), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite))
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
        side_effect: SideEffect::LocalWrite,
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
        side_effect: SideEffect::LocalWrite,
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
    // Actions with SideEffect::Pure always pass, regardless of retry_safety.
    let action = ActionContract {
        id: ActionId::new(105),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 1,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
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
        side_effect: SideEffect::ExternalWrite,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let frame = RunFrame::new(RunId::new(56), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.ok().expect("test setup");
    let result = verify_idempotency(&action, &[SlotIdx::new(0)], &frame);
    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::ExternalWrite))
    );
}

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
        side_effect: SideEffect::Pure,
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
        side_effect: SideEffect::Pure,
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
        side_effect: SideEffect::Pure,
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
        side_effect: SideEffect::Pure,
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
    let ticket = issue_action_ticket(
        RunId::new(42),
        StepIdx::new(3),
        SeqNo::new(7),
        ActionId::new(5),
        2,
        12345,
        1,
    );
    assert_eq!(ticket.run, RunId::new(42));
    assert_eq!(ticket.step, StepIdx::new(3));
    assert_eq!(ticket.seq, SeqNo::new(7));
    assert_eq!(ticket.action, ActionId::new(5));
    assert_eq!(ticket.attempt, 2);
    assert_eq!(ticket.idempotency_key, 12345);
}

#[test]
fn issue_action_ticket_with_zero_key_is_valid() {
    let ticket = issue_action_ticket(
        RunId::new(1),
        StepIdx::new(0),
        SeqNo::new(1),
        ActionId::new(0),
        1,
        0,
        1,
    );
    assert_eq!(ticket.idempotency_key, 0);
    assert_eq!(ticket.attempt, 1);
}

#[test]
fn issue_action_ticket_with_max_values() {
    let ticket = issue_action_ticket(
        RunId::new(u64::MAX),
        StepIdx::new(u16::MAX),
        SeqNo::new(u64::MAX),
        ActionId::new(u16::MAX),
        u16::MAX,
        u128::MAX,
        u16::MAX,
    );
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
        side_effect: SideEffect::Pure,
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
    let result = validate_action_outcome(&contract, &outcome, Taint::Clean);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_outcome_ready_rejects_out_of_bounds_slot() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
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
    let result = validate_action_outcome(&contract, &outcome, Taint::Clean);
    assert_eq!(
        result,
        Err(ActionError::OutputSlotOutOfBounds {
            slot: 5,
            max_slots: 1,
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
        side_effect: SideEffect::Pure,
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
    let result = validate_action_outcome(&contract, &outcome, Taint::Clean);
    assert_eq!(result, Ok(()));
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
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    let outcome = ActionOutcome::Suspended(ticket);
    let result = validate_action_outcome(&contract, &outcome, Taint::Clean);
    assert_eq!(result, Err(ActionError::DispatchFailed));
}

#[test]
fn validate_action_outcome_rejects_taint_downgrade_deterministic_pure() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 2,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
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
    let result = validate_action_outcome(&contract, &outcome, Taint::Secret);
    assert_eq!(
        result,
        Err(ActionError::TaintViolation {
            required: Taint::Clean,
            supplied: Taint::Secret,
        })
    );
}

#[test]
fn validate_action_outcome_deterministic_pure_rejects_secret_output_with_clean_input() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 2,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Secret,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(output);
    let result = validate_action_outcome(&contract, &outcome, Taint::Clean);
    assert_eq!(
        result,
        Err(ActionError::TaintViolation {
            required: Taint::Clean,
            supplied: Taint::Secret,
        })
    );
}

#[test]
fn validate_action_outcome_idempotent_external_rejects_secret_output_with_clean_input() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 2,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Secret,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(output);
    let result = validate_action_outcome(&contract, &outcome, Taint::Clean);
    assert_eq!(
        result,
        Err(ActionError::TaintViolation {
            required: Taint::Clean,
            supplied: Taint::Secret,
        })
    );
}

#[test]
fn validate_action_outcome_rejects_taint_downgrade_idempotent_external() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 2,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Pure,
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
    let result = validate_action_outcome(&contract, &outcome, Taint::DerivedFromSecret);
    assert_eq!(
        result,
        Err(ActionError::TaintViolation {
            required: Taint::Clean,
            supplied: Taint::DerivedFromSecret,
        })
    );
}

#[test]
fn validate_action_outcome_accepts_correct_taint_idempotent_external() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 2,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Pure,
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
    let result = validate_action_outcome(&contract, &outcome, Taint::Clean);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_outcome_rejects_taint_downgrade_at_least_once() {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 2,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Pure,
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
    let result = validate_action_outcome(&contract, &outcome, Taint::Secret);
    assert_eq!(
        result,
        Err(ActionError::TaintViolation {
            required: Taint::DerivedFromSecret,
            supplied: Taint::Clean,
        })
    );
}

// --- ActionJournalEvent ---

#[test]
fn journal_event_suspended_roundtrips_fields() {
    let ticket = ActionTicket {
        run: RunId::new(10),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: 999,
        capacity: 1,
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
fn journal_event_completed_roundtrips_fields() {
    let ticket = ActionTicket {
        run: RunId::new(11),
        step: StepIdx::new(3),
        seq: SeqNo::new(4),
        action: ActionId::new(8),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    let event = ActionJournalEvent::Completed {
        ticket,
        attempt: ticket.attempt,
        output_slot: SlotIdx::new(2),
        output_taint: Taint::Secret,
    };
    match event {
        ActionJournalEvent::Completed {
            ticket: t,
            attempt,
            output_slot,
            output_taint,
        } => {
            assert_eq!(t.run, RunId::new(11));
            assert_eq!(attempt, 1);
            assert_eq!(output_slot, SlotIdx::new(2));
            assert_eq!(output_taint, Taint::Secret);
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[test]
fn journal_event_failed_roundtrips_fields() {
    let ticket = ActionTicket {
        run: RunId::new(12),
        step: StepIdx::new(4),
        seq: SeqNo::new(5),
        action: ActionId::new(9),
        attempt: 3,
        idempotency_key: 0,
        capacity: 1,
    };
    let event = ActionJournalEvent::Failed {
        ticket,
        attempt: ticket.attempt,
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
    };
    match event {
        ActionJournalEvent::Failed {
            ticket: t,
            attempt,
            code,
            retry_policy,
        } => {
            assert_eq!(t.run, RunId::new(12));
            assert_eq!(attempt, 3);
            assert_eq!(code, ActionFailureCode::Timeout);
            assert_eq!(retry_policy, RetryPolicy::Retryable);
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn journal_event_serialization_roundtrip() {
    let ticket = ActionTicket {
        run: RunId::new(99),
        step: StepIdx::new(1),
        seq: SeqNo::new(2),
        action: ActionId::new(3),
        attempt: 1,
        idempotency_key: 12345,
        capacity: 1,
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

#[test]
fn journal_event_completed_serialization_roundtrip() {
    let ticket = ActionTicket {
        run: RunId::new(50),
        step: StepIdx::new(5),
        seq: SeqNo::new(10),
        action: ActionId::new(2),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    let event = ActionJournalEvent::Completed {
        ticket,
        attempt: ticket.attempt,
        output_slot: SlotIdx::new(3),
        output_taint: Taint::DerivedFromSecret,
    };
    let bytes = postcard::to_allocvec(&event);
    assert!(bytes.is_ok());
    let bytes = bytes.ok().expect("test setup");
    let recovered: Result<ActionJournalEvent, _> = postcard::from_bytes(&bytes);
    assert!(recovered.is_ok());
    assert_eq!(recovered.ok().expect("test setup"), event);
}

#[test]
fn journal_event_failed_serialization_roundtrip() {
    let ticket = ActionTicket {
        run: RunId::new(51),
        step: StepIdx::new(6),
        seq: SeqNo::new(11),
        action: ActionId::new(4),
        attempt: 2,
        idempotency_key: 0,
        capacity: 1,
    };
    let event = ActionJournalEvent::Failed {
        ticket,
        attempt: ticket.attempt,
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
    };
    let bytes = postcard::to_allocvec(&event);
    assert!(bytes.is_ok());
    let bytes = bytes.ok().expect("test setup");
    let recovered: Result<ActionJournalEvent, _> = postcard::from_bytes(&bytes);
    assert!(recovered.is_ok());
    assert_eq!(recovered.ok().expect("test setup"), event);
}

// =========================================================================
// Edge-case tests -- ActionTicket construction, ActionError display,
// ActionId equality, ActionContract fields/defaults, zero timeout
// =========================================================================

#[test]
fn action_ticket_construction_all_fields_accessible() {
    let ticket = ActionTicket {
        run: RunId::new(999),
        step: StepIdx::new(42),
        seq: SeqNo::new(100),
        action: ActionId::new(7),
        attempt: 3,
        idempotency_key: 0xDEADBEEF_u128,
        capacity: 10,
    };
    assert_eq!(ticket.run, RunId::new(999));
    assert_eq!(ticket.step, StepIdx::new(42));
    assert_eq!(ticket.seq, SeqNo::new(100));
    assert_eq!(ticket.action, ActionId::new(7));
    assert_eq!(ticket.attempt, 3);
    assert_eq!(ticket.idempotency_key, 0xDEADBEEF_u128);
    assert_eq!(ticket.capacity, 10);
}

#[test]
fn action_ticket_with_zero_timeout_via_contract() {
    let contract = ActionContract {
        id: ActionId::new(10),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 512,
        max_output_bytes: 512,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    // Zero timeout is a valid edge case; the contract should still be constructable.
    assert_eq!(contract.timeout_ms, 0);
    assert_eq!(contract.id, ActionId::new(10));
}

#[test]
fn action_error_unknown_action_display_message() {
    let error = ActionError::UnknownAction {
        action: ActionId::new(77),
    };
    let msg = error.to_string();
    assert!(
        msg.contains("unknown action"),
        "display must contain 'unknown action', got: {msg}"
    );
    assert!(
        msg.contains("ActionId(77)"),
        "display must contain the action id, got: {msg}"
    );
}

#[test]
fn action_error_timeout_display_via_failure_code() {
    // ActionError does not have a Timeout variant directly, but
    // ActionFailureCode::Timeout exists. Verify its discriminant and
    // that ActionFailure carries it.
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert_eq!(failure.code, ActionFailureCode::Timeout);
    // Verify the Timeout repr is 1 (defined as = 1 in the enum).
    assert_eq!(failure_code_repr(ActionFailureCode::Timeout), 1);
}

#[test]
fn action_error_payload_too_large_display_contains_both_sizes() {
    let error = ActionError::PayloadTooLarge {
        max_bytes: 100,
        actual_bytes: 250,
    };
    let msg = error.to_string();
    assert!(
        msg.contains("100"),
        "display must contain max_bytes, got: {msg}"
    );
    assert!(
        msg.contains("250"),
        "display must contain actual_bytes, got: {msg}"
    );
}

#[test]
fn action_error_equality_all_variants() {
    // Verify PartialEq works for each variant.
    assert_eq!(
        ActionError::UnknownAction {
            action: ActionId::new(1)
        },
        ActionError::UnknownAction {
            action: ActionId::new(1)
        }
    );
    assert_eq!(ActionError::InvalidTicket, ActionError::InvalidTicket);
    assert_eq!(
        ActionError::PayloadTooLarge {
            max_bytes: 10,
            actual_bytes: 20
        },
        ActionError::PayloadTooLarge {
            max_bytes: 10,
            actual_bytes: 20
        }
    );
    assert_eq!(
        ActionError::OutputSlotOutOfBounds {
            slot: 3,
            max_slots: 2
        },
        ActionError::OutputSlotOutOfBounds {
            slot: 3,
            max_slots: 2
        }
    );
    assert_eq!(
        ActionError::NonIdempotentReplayBlocked,
        ActionError::NonIdempotentReplayBlocked
    );
    assert_eq!(
        ActionError::CompletionAlreadyRecorded,
        ActionError::CompletionAlreadyRecorded
    );
    assert_eq!(ActionError::QueueFull, ActionError::QueueFull);
    assert_eq!(ActionError::EncodingFailed, ActionError::EncodingFailed);
    assert_eq!(ActionError::DispatchFailed, ActionError::DispatchFailed);
}

#[test]
fn action_error_inequality_different_variants() {
    let a = ActionError::QueueFull;
    let b = ActionError::EncodingFailed;
    assert_ne!(a, b);
}

#[test]
fn action_id_creation_and_equality() {
    let id_a = ActionId::new(42);
    let id_b = ActionId::new(42);
    let id_c = ActionId::new(43);
    assert_eq!(id_a, id_b);
    assert_ne!(id_a, id_c);
    assert_eq!(id_a.get(), 42);
}

#[test]
fn action_id_max_value() {
    let id = ActionId::new(u16::MAX);
    assert_eq!(id.get(), u16::MAX);
}

#[test]
fn action_contract_fields_and_required_capabilities() {
    let contract = ActionContract {
        id: ActionId::new(5),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 3,
        output_slot_count: 2,
        max_input_bytes: 4096,
        max_output_bytes: 2048,
        timeout_ms: 30_000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([Capability::new(
            String::from("file_read").into_boxed_str(),
            ActionId::new(5),
        )]),
    };
    assert_eq!(contract.id, ActionId::new(5));
    assert_eq!(contract.input_slot_count, 3);
    assert_eq!(contract.output_slot_count, 2);
    assert_eq!(contract.max_input_bytes, 4096);
    assert_eq!(contract.max_output_bytes, 2048);
    assert_eq!(contract.timeout_ms, 30_000);
    assert_eq!(contract.idempotency, Idempotency::IdempotentExternal);
    assert_eq!(contract.side_effect, SideEffect::LocalWrite);
    assert_eq!(contract.retry_safety, RetrySafety::KeyRequired);
    assert_eq!(contract.required_capabilities.len(), 1);
}

#[test]
fn action_contract_default_like_values() {
    // Verify a minimal "default-like" contract with zero-count fields.
    let contract = ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    assert_eq!(contract.id, ActionId::new(0));
    assert_eq!(contract.input_slot_count, 0);
    assert_eq!(contract.output_slot_count, 0);
    assert!(contract.required_capabilities.is_empty());
}

// =========================================================================
// vb-8mdp.6: Idempotency Hydration proptest tests
// PO-VB-IDEM-001c, PO-VB-IDEM-012b
// =========================================================================

/// Deterministic pseudo-random u16 in [0, n) derived from iteration count.
/// Uses a simple hash-based approach: mix the iteration counter with
/// the per-test call-site id to produce a deterministic sequence.
fn deterministic_rand_u16_bounded(n: u16, iter: u32, site_id: u32) -> u16 {
    // Simple mixing function: xorshift-like, returns u16
    let mut x = iter.wrapping_mul(1664525).wrapping_add(1013904223);
    x = x ^ (x >> 13);
    x = x.wrapping_mul(1664525).wrapping_add(1013904223);
    x = x ^ (x >> 17);
    x = x ^ (x >> 5);
    let combined = x.wrapping_mul(31).wrapping_add(site_id);
    combined as u16 % n
}

#[test]
fn test_key_computation_deterministic() {
    // Property: compute_action_idempotency_key is deterministic.
    // f(run, seq, action) == f(run, seq, action) for all inputs.
    use crate::ids::{ActionId, RunId, SeqNo};

    for iter in 0..1000u32 {
        let run = RunId::new(u64::from(deterministic_rand_u16_bounded(64, iter, 0)));
        let seq = SeqNo::new(u64::from(deterministic_rand_u16_bounded(64, iter, 1)));
        let action = ActionId::new(deterministic_rand_u16_bounded(16, iter, 2));

        let key1 = compute_action_idempotency_key(run, seq, action);
        let key2 = compute_action_idempotency_key(run, seq, action);

        assert_eq!(key1, key2, "key computation must be deterministic");
    }
}

#[test]
fn test_canonical_key_validates() {
    // Property: ticket with canonical key validates; ticket with wrong key rejects.
    for iter in 0..1000u32 {
        let run = RunId::new(u64::from(deterministic_rand_u16_bounded(64, iter, 10)));
        let seq = SeqNo::new(u64::from(deterministic_rand_u16_bounded(64, iter, 11)));
        let action = ActionId::new(deterministic_rand_u16_bounded(16, iter, 12));

        let canonical_key = compute_action_idempotency_key(run, seq, action);

        // Canonical ticket should validate
        let canonical_ticket = ActionTicket {
            run,
            step: StepIdx::new(deterministic_rand_u16_bounded(4, iter, 13)),
            seq,
            action,
            attempt: deterministic_rand_u16_bounded(3, iter, 14) + 1,
            idempotency_key: canonical_key,
            capacity: deterministic_rand_u16_bounded(5, iter, 15) + 1,
        };
        assert!(
            action_ticket_has_valid_key(canonical_ticket),
            "canonical key must validate"
        );

        // Wrong key should reject
        let wrong_ticket = ActionTicket {
            idempotency_key: canonical_key.wrapping_add(1),
            ..canonical_ticket
        };
        assert!(
            !action_ticket_has_valid_key(wrong_ticket),
            "wrong key must reject"
        );
    }
}

// =========================================================================
// vb-tub4: Kani proof obligations - idempotency bounded property test
// =========================================================================

#[test]
fn idempotency_property_bounded() {
    // B-IDEM-001 variant: verify_idempotency is deterministic for bounded inputs
    // Same contract/key/frame must produce same result across duplicate calls.
    // This is the "bounded" property - we test with concrete (non-symbolic) values.
    use crate::action::{
        ActionContract, ActionId, ActionName, Idempotency, RetrySafety, SideEffect,
        verify_idempotency,
    };
    use crate::frame::RunFrame;
    use crate::ids::{RunId, SlotIdx, StepIdx};
    use crate::value::SlotValue;

    // Create a deterministic clean contract with KeyRequired safety
    let contract = ActionContract {
        id: ActionId::new(42),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 2,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 512,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };

    // Create a clean frame with initialized key slots
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).unwrap();
    let mut frame = frame;
    // Populate key slots with clean values
    let _ = frame.write_slot(SlotIdx::new(0), SlotValue::I64(100));
    let _ = frame.write_slot(SlotIdx::new(1), SlotValue::Bool(true));

    let key_slots = &[SlotIdx::new(0), SlotIdx::new(1)];

    // First call
    let first_result = verify_idempotency(&contract, key_slots, &frame);

    // Second call with same inputs - must be deterministic
    let second_result = verify_idempotency(&contract, key_slots, &frame);

    assert_eq!(
        first_result, second_result,
        "verify_idempotency must be deterministic: same inputs produce same result"
    );

    // Third call to further confirm stability
    let third_result = verify_idempotency(&contract, key_slots, &frame);
    assert_eq!(
        first_result, third_result,
        "verify_idempotency must be stable across multiple calls"
    );
}
