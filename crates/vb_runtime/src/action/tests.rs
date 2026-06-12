use super::*;
use vb_core::action::{ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::{RunId, SeqNo, SlotIdx, StepIdx};

fn contract_fixture(id: u16) -> ActionContract {
    ActionContract {
        id: ActionId::new(id),
        name: ActionName::new(format!("test-action-{id}")).unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    }
}

fn input_fixture(action: u16) -> ActionInput {
    ActionInput {
        run: RunId::new(1),
        step: StepIdx::new(0),
        action: ActionId::new(action),
        input: SlotIdx::new(0),
        ticket: ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(action),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        },
    }
}

#[test]
fn register_and_resolve_action() {
    let mut registry = ActionRegistry::new();
    let contract = contract_fixture(10);
    assert_eq!(registry.register(contract), Ok(()));
    let resolved = registry.resolve_compile_time(ActionId::new(10));
    assert_eq!(resolved.map(|c| c.id), Ok(ActionId::new(10)));
}

#[test]
fn resolve_unknown_action_returns_error() {
    let registry = ActionRegistry::new();
    let result = registry.resolve_compile_time(ActionId::new(99));
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(99)
        })
    );
}

#[test]
fn dispatch_produces_suspended_outcome() {
    let mut registry = ActionRegistry::new();
    let contract = contract_fixture(5);
    assert_eq!(registry.register(contract), Ok(()));
    let input = input_fixture(5);
    let resolved = registry.resolve_compile_time(ActionId::new(5));
    assert_eq!(resolved.as_ref().map(|c| c.id), Ok(ActionId::new(5)));
    let contract = resolved.ok().cloned();
    assert_eq!(contract.as_ref().map(|c| c.id), Some(ActionId::new(5)));
    let Some(ref contract) = contract else { return };
    let result = registry.dispatch(&input, contract);
    match result {
        Ok(ActionOutcome::Suspended(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(5));
        }
        other => assert_eq!(
            other,
            Ok(ActionOutcome::Suspended(ActionTicket {
                run: RunId::new(1),
                step: StepIdx::new(0),
                seq: SeqNo::new(0),
                action: ActionId::new(5),
                attempt: 1,
                idempotency_key: 0,
                capacity: 1,
            }))
        ),
    }
}

#[test]
fn register_duplicate_returns_error() {
    let mut registry = ActionRegistry::new();
    let contract = contract_fixture(3);
    assert_eq!(registry.register(contract), Ok(()));
    let duplicate = contract_fixture(3);
    assert_eq!(
        registry.register(duplicate),
        Err(ActionError::DispatchFailed)
    );
}

#[test]
fn register_duplicate_name_returns_unknown_action() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(3)), Ok(()));

    let duplicate_name = ActionContract {
        id: ActionId::new(4),
        ..contract_fixture(3)
    };

    assert_eq!(
        registry.register(duplicate_name),
        Err(ActionError::UnknownAction {
            action: ActionId::new(4)
        })
    );
}

#[test]
fn default_registry_is_empty() {
    let registry = ActionRegistry::default();
    assert_eq!(registry.is_empty(), true);
}

#[test]
fn len_returns_zero_for_new_registry() {
    let registry = ActionRegistry::new();
    assert_eq!(registry.len(), 0);
}

#[test]
fn len_increases_after_register() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.len(), 0);
    assert_eq!(registry.register(contract_fixture(1)), Ok(()));
    assert_eq!(registry.register(contract_fixture(5)), Ok(()));
    assert_eq!(registry.len(), 6);
}

#[test]
fn validate_input_bytes_rejects_when_max_input_bytes_is_zero() {
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    assert_eq!(registry.register(contract), Ok(()));
    let input = input_fixture(1);
    let resolved = registry.resolve_compile_time(ActionId::new(1));
    assert_eq!(resolved.as_ref().map(|c| c.id), Ok(ActionId::new(1)));
    let contract = resolved.ok().cloned();
    let Some(ref contract) = contract else { return };
    let result = registry.dispatch(&input, contract);
    assert_eq!(
        result,
        Err(ActionError::PayloadTooLarge {
            max_bytes: 0,
            actual_bytes: 0
        })
    );
}

#[test]
fn action_registry_resolve_returns_correct_contract() {
    // Given a registry with one contract
    let mut registry = ActionRegistry::new();
    let contract = contract_fixture(5);
    assert_eq!(registry.register(contract), Ok(()));
    // When resolving the action
    let result = registry.resolve_compile_time(ActionId::new(5));
    // Then it returns the correct contract with matching id
    match result {
        Ok(c) => {
            assert_eq!(c.id, ActionId::new(5));
            assert_eq!(c.input_slot_count, 1);
            assert_eq!(c.output_slot_count, 1);
            assert_eq!(c.max_input_bytes, 1024);
            assert_eq!(c.max_output_bytes, 1024);
        }
        Err(_) => {
            assert!(false);
        }
    }
}

#[test]
fn action_registry_register_fills_gaps() {
    // Given a registry where action 10 is registered first
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(10)), Ok(()));
    // Then len is 11 (slots 0..10)
    assert_eq!(registry.len(), 11);
    // And action 10 resolves correctly
    let resolved = registry.resolve_compile_time(ActionId::new(10));
    assert_eq!(resolved.map(|c| c.id), Ok(ActionId::new(10)));
}

#[test]
fn action_registry_dispatch_rejects_mismatched_contract() {
    // Given a registry with action 5
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(5)), Ok(()));
    // When dispatching with input for action 5 but a different contract
    let input = input_fixture(5);
    let wrong_contract = ActionContract {
        id: ActionId::new(3),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    let result = registry.dispatch(&input, &wrong_contract);
    // Then it returns an UnknownAction error
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(5)
        })
    );
}

#[test]
fn action_registry_is_not_empty_after_register() {
    // Given a registry with one action
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(1)), Ok(()));
    // When checking is_empty
    // Then it is not empty
    assert_eq!(registry.is_empty(), false);
}

#[test]
fn action_registry_new_default_matches_new() {
    // Given a default registry
    let default = ActionRegistry::default();
    let new = ActionRegistry::new();
    // When comparing
    // Then both are empty with same len
    assert_eq!(default.is_empty(), true);
    assert_eq!(new.is_empty(), true);
    assert_eq!(default.len(), 0);
    assert_eq!(new.len(), 0);
}

#[test]
fn action_registry_register_multiple_actions() {
    // Given a registry
    let mut registry = ActionRegistry::new();
    // When registering actions 0, 1, 2
    assert_eq!(registry.register(contract_fixture(0)), Ok(()));
    assert_eq!(registry.register(contract_fixture(1)), Ok(()));
    assert_eq!(registry.register(contract_fixture(2)), Ok(()));
    // Then all resolve correctly
    assert_eq!(
        registry
            .resolve_compile_time(ActionId::new(0))
            .map(|c| c.id),
        Ok(ActionId::new(0))
    );
    assert_eq!(
        registry
            .resolve_compile_time(ActionId::new(1))
            .map(|c| c.id),
        Ok(ActionId::new(1))
    );
    assert_eq!(
        registry
            .resolve_compile_time(ActionId::new(2))
            .map(|c| c.id),
        Ok(ActionId::new(2))
    );
    assert_eq!(registry.len(), 3);
}

#[test]
fn action_registry_resolve_unregistered_action_fails() {
    // Given a registry with action 0
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(0)), Ok(()));
    // When resolving unregistered action 5
    let result = registry.resolve_compile_time(ActionId::new(5));
    // Then it returns UnknownAction
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(5)
        })
    );
}

#[test]
fn action_registry_dispatch_with_correct_contract_succeeds() {
    // Given a registry with action 0
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(0)), Ok(()));
    let input = input_fixture(0);
    let contract = contract_fixture(0);
    // When dispatching with matching contract
    let result = registry.dispatch(&input, &contract);
    // Then it succeeds with Suspended outcome
    match result {
        Ok(ActionOutcome::Suspended(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(0));
        }
        other => {
            assert_eq!(
                other,
                Ok(ActionOutcome::Suspended(ActionTicket {
                    run: RunId::new(1),
                    step: StepIdx::new(0),
                    seq: SeqNo::new(0),
                    action: ActionId::new(0),
                    attempt: 1,
                    idempotency_key: 0,
                    capacity: 1,
                }))
            );
        }
    }
}

#[test]
fn action_contract_fields_are_preserved() {
    // Given a contract with specific fields
    let contract = ActionContract {
        id: ActionId::new(42),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 3,
        output_slot_count: 2,
        max_input_bytes: 2048,
        max_output_bytes: 4096,
        timeout_ms: 10000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };
    // When registering and resolving
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract), Ok(()));
    let resolved = registry.resolve_compile_time(ActionId::new(42));
    // Then all fields are preserved
    match resolved {
        Ok(c) => {
            assert_eq!(c.id, ActionId::new(42));
            assert_eq!(c.input_slot_count, 3);
            assert_eq!(c.output_slot_count, 2);
            assert_eq!(c.max_input_bytes, 2048);
            assert_eq!(c.max_output_bytes, 4096);
            assert_eq!(c.timeout_ms, 10000);
            assert_eq!(c.idempotency, Idempotency::IdempotentExternal);
        }
        Err(_) => {
            assert!(false);
        }
    }
}

#[test]
fn action_registry_len_increases_with_gap() {
    // Given a registry with action 5
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(5)), Ok(()));
    // Then len is 6 (slots 0..5)
    assert_eq!(registry.len(), 6);
}

#[test]
fn action_registry_gap_slot_rejects_default_placeholder_id() {
    // Given a registry with action 5
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(5)), Ok(()));
    // When resolving action 0 (gap slot filled with an internal placeholder)
    let result = registry.resolve_compile_time(ActionId::new(0));
    // Then the placeholder is not exposed as a registered action.
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(0)
        })
    );
}

#[test]
fn action_registry_gap_slot_nondefault_id_fails() {
    // Given a registry with action 5
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(5)), Ok(()));
    // When resolving action 3 (gap slot with default id, not matching 3)
    let result = registry.resolve_compile_time(ActionId::new(3));
    // Then it returns UnknownAction
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(3)
        })
    );
}

// =======================================================================
// Adversarial BDD tests - action registry attack vectors
// =======================================================================

#[test]
fn action_registry_dispatch_unknown_action_returns_exact_error_variant() {
    // Given an empty registry
    let registry = ActionRegistry::new();
    let input = input_fixture(99);
    let contract = contract_fixture(99);
    // When dispatching an unknown action
    let result = registry.dispatch(&input, &contract);
    // Then it returns UnknownAction with the exact action id
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(99)
        })
    );
}

#[test]
fn action_registry_register_then_reregister_same_id_returns_dispatch_failed() {
    // Given a registry with action 1
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(1)), Ok(()));
    // When registering the same action id again
    let result = registry.register(contract_fixture(1));
    // Then it returns DispatchFailed (duplicate rejection)
    assert_eq!(result, Err(ActionError::DispatchFailed));
}

#[test]
fn action_registry_register_max_action_id_does_not_overflow() {
    // Given an empty registry
    let mut registry = ActionRegistry::new();
    // When registering action at max valid index (65534)
    let contract = ActionContract {
        id: ActionId::new(65534),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    let result = registry.register(contract);
    // Then it succeeds (65534 < 65535 = MAX_REGISTERED_ACTIONS)
    assert_eq!(result, Ok(()));
    assert_eq!(registry.len(), 65535);
}

#[test]
fn action_registry_validate_input_bytes_rejects_zero_with_slots() {
    // Given a contract with max_input_bytes=0 and input_slot_count=1
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    assert_eq!(registry.register(contract), Ok(()));
    let input = input_fixture(1);
    let resolved = registry.resolve_compile_time(ActionId::new(1));
    let contract = match resolved {
        Ok(c) => c.clone(),
        Err(_) => return,
    };
    // When dispatching
    let result = registry.dispatch(&input, &contract);
    // Then it returns PayloadTooLarge (zero bytes with slots)
    assert_eq!(
        result,
        Err(ActionError::PayloadTooLarge {
            max_bytes: 0,
            actual_bytes: 0,
        })
    );
}

#[test]
fn action_registry_dispatch_with_contract_zero_bytes_and_zero_slots_succeeds() {
    // Given a contract with max_input_bytes=0 and input_slot_count=0
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(2),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    assert_eq!(registry.register(contract), Ok(()));
    let input = ActionInput {
        run: RunId::new(1),
        step: StepIdx::new(0),
        action: ActionId::new(2),
        input: SlotIdx::new(0),
        ticket: ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(2),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        },
    };
    let contract = match registry.resolve_compile_time(ActionId::new(2)) {
        Ok(c) => c.clone(),
        Err(_) => return,
    };
    // When dispatching with zero bytes and zero slots
    let result = registry.dispatch(&input, &contract);
    // Then it succeeds (no payload to validate)
    match result {
        Ok(ActionOutcome::Suspended(_)) => {}
        other => {
            assert_eq!(
                other,
                Ok(ActionOutcome::Suspended(ActionTicket {
                    run: RunId::new(1),
                    step: StepIdx::new(0),
                    seq: SeqNo::new(0),
                    action: ActionId::new(2),
                    attempt: 1,
                    idempotency_key: 0,
                    capacity: 1,
                }))
            );
        }
    }
}

#[test]
fn action_registry_resolve_after_many_registrations_finds_correct_action() {
    // Given a registry with actions 0, 5, 10, 20
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(0)), Ok(()));
    assert_eq!(registry.register(contract_fixture(5)), Ok(()));
    assert_eq!(registry.register(contract_fixture(10)), Ok(()));
    assert_eq!(registry.register(contract_fixture(20)), Ok(()));
    // When resolving action 10
    let result = registry.resolve_compile_time(ActionId::new(10));
    // Then it returns the correct contract
    match result {
        Ok(c) => {
            assert_eq!(c.id, ActionId::new(10));
            assert_eq!(c.input_slot_count, 1);
        }
        Err(_) => {
            assert!(false);
        }
    }
}

#[test]
fn registered_contracts_returns_only_real_contracts_sorted_by_action_id() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(10)), Ok(()));
    assert_eq!(registry.register(contract_fixture(2)), Ok(()));
    assert_eq!(registry.register(contract_fixture(5)), Ok(()));

    let listed: Vec<ActionId> = registry
        .registered_contracts()
        .iter()
        .map(|contract| contract.id)
        .collect();

    assert_eq!(
        listed,
        vec![ActionId::new(2), ActionId::new(5), ActionId::new(10)]
    );
    assert_eq!(registry.len(), 11);
}

#[test]
fn action_registry_dispatch_returns_ticket_with_correct_action_from_input() {
    // Given a registry with action 3
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract_fixture(3)), Ok(()));
    let input = ActionInput {
        run: RunId::new(77),
        step: StepIdx::new(5),
        action: ActionId::new(3),
        input: SlotIdx::new(0),
        ticket: ActionTicket {
            run: RunId::new(77),
            step: StepIdx::new(5),
            seq: SeqNo::new(10),
            action: ActionId::new(3),
            attempt: 2,
            idempotency_key: 99,
            capacity: 1,
        },
    };
    let contract = match registry.resolve_compile_time(ActionId::new(3)) {
        Ok(c) => c.clone(),
        Err(_) => return,
    };
    // When dispatching
    let result = registry.dispatch(&input, &contract);
    // Then the returned ticket carries the input ticket's fields
    match result {
        Ok(ActionOutcome::Suspended(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(3));
            assert_eq!(ticket.run, RunId::new(77));
            assert_eq!(ticket.step, StepIdx::new(5));
            assert_eq!(ticket.seq, SeqNo::new(10));
            assert_eq!(ticket.attempt, 2);
            assert_eq!(ticket.idempotency_key, 99);
        }
        other => {
            assert_eq!(
                other,
                Ok(ActionOutcome::Suspended(ActionTicket {
                    run: RunId::new(0),
                    step: StepIdx::new(0),
                    seq: SeqNo::new(0),
                    action: ActionId::new(0),
                    attempt: 0,
                    idempotency_key: 0,
                    capacity: 1,
                }))
            );
        }
    }
}

// ========================================================================
// IdempotencyTracker tests
// ========================================================================

#[test]
fn idempotency_tracker_new_is_empty() {
    use crate::idempotency::IdempotencyTracker;
    let tracker = IdempotencyTracker::with_default_capacity();
    assert_eq!(tracker.is_empty(), true);
    assert_eq!(tracker.len(), 0);
}

#[test]
fn idempotency_tracker_record_completion_succeeds() {
    use crate::idempotency::IdempotencyTracker;
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let ticket = ActionTicket {
        run: RunId::new(0),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 0,
        idempotency_key: 42,
        capacity: 1,
    };
    assert_eq!(tracker.mark_completed(&ticket), Ok(()));
    assert_eq!(tracker.is_completed(&ticket), true);
    assert_eq!(tracker.len(), 1);
}

#[test]
fn idempotency_tracker_duplicate_completion_returns_error() {
    use crate::idempotency::IdempotencyTracker;
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let ticket = ActionTicket {
        run: RunId::new(0),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 0,
        idempotency_key: 99,
        capacity: 1,
    };
    assert_eq!(tracker.mark_completed(&ticket), Ok(()));
    assert_eq!(
        tracker.mark_completed(&ticket),
        Err(ActionError::CompletionAlreadyRecorded)
    );
}

#[test]
fn idempotency_tracker_different_keys_are_independent() {
    use crate::idempotency::IdempotencyTracker;
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let ticket_a = ActionTicket {
        run: RunId::new(0),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 0,
        idempotency_key: 1,
        capacity: 1,
    };
    let ticket_b = ActionTicket {
        run: RunId::new(0),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 0,
        idempotency_key: 2,
        capacity: 1,
    };
    let ticket_c = ActionTicket {
        run: RunId::new(0),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 0,
        idempotency_key: 3,
        capacity: 1,
    };
    assert_eq!(tracker.mark_completed(&ticket_a), Ok(()));
    assert_eq!(tracker.mark_completed(&ticket_b), Ok(()));
    assert_eq!(tracker.is_completed(&ticket_a), true);
    assert_eq!(tracker.is_completed(&ticket_b), true);
    assert_eq!(tracker.is_completed(&ticket_c), false);
    assert_eq!(tracker.len(), 2);
}

#[test]
fn idempotency_tracker_default_matches_new() {
    use crate::idempotency::IdempotencyTracker;
    let default = IdempotencyTracker::default();
    let new = IdempotencyTracker::with_default_capacity();
    assert_eq!(default.len(), new.len());
    assert_eq!(default.is_empty(), new.is_empty());
}

// =========================================================================
// vb-u09ai: 4-variant RetrySafety action dispatch tests (Tier 1).
// =========================================================================

/// Tier 1/2: `verify_idempotency` accepts `Idempotent` retry_safety for
/// a non-pure side-effect even with empty key_slots (C6 contract: Idempotent
/// is unconditionally safe to retry).
#[test]
fn action_dispatch_idempotent_retry_safety_recognized() {
    use vb_core::action::{IdempotencyViolation, RetrySafety, verify_idempotency};
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, RunId, StepIdx};
    let action = ActionContract {
        id: ActionId::new(8001),
        name: ActionName::new("test-4v-idempotent").unwrap(),
        input_slot_count: 0,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    let frame = match RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1) {
        Ok(f) => f,
        Err(_) => return,
    };
    assert_eq!(
        verify_idempotency(&action, &[], &frame),
        Ok(()),
        "Idempotent + LocalWrite with empty key slots must be Ok(())"
    );
    // Negative control: also assert that MissingKey variant is NOT produced.
    if let Err(IdempotencyViolation::MissingKey(_)) = verify_idempotency(&action, &[], &frame) {
        panic!("Idempotent must not produce MissingKey violation");
    }
}

/// Tier 1/2: `verify_idempotency` rejects `Unknown` retry_safety for a
/// non-pure side-effect with empty key_slots, producing `MissingKey(se)`
/// (C8 contract: Unknown collapses to Unsafe / missing-key semantics).
#[test]
fn action_dispatch_unknown_retry_safety_recognized() {
    use vb_core::action::{IdempotencyViolation, RetrySafety, verify_idempotency};
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, RunId, StepIdx};
    let action = ActionContract {
        id: ActionId::new(8002),
        name: ActionName::new("test-4v-unknown").unwrap(),
        input_slot_count: 0,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::Unknown,
        required_capabilities: Box::new([]),
    };
    let frame = match RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1) {
        Ok(f) => f,
        Err(_) => return,
    };
    let result = verify_idempotency(&action, &[], &frame);
    assert!(
        matches!(result, Err(IdempotencyViolation::MissingKey(_))),
        "Unknown + LocalWrite with empty key slots must be MissingKey, got {result:?}"
    );
}
