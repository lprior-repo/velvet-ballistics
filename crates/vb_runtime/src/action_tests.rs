//! Tests for the action registry.
#![forbid(unsafe_code)]

use vb_core::action::{
    ActionContract, ActionError, ActionInput, ActionOutcome, ActionResult, ActionTicket,
    Idempotency, RetrySafety, SideEffect,
};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};

use crate::action::ActionRegistry;

fn test_contract(id: u16) -> ActionContract {
    ActionContract {
        id: ActionId::new(id),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

fn test_input(action: u16) -> ActionInput {
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
        },
    }
}

#[test]
fn register_and_resolve_action() {
    let mut registry = ActionRegistry::new();
    let contract = test_contract(10);
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
    let contract = test_contract(5);
    assert_eq!(registry.register(contract), Ok(()));
    let input = test_input(5);
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
            }))
        ),
    }
}

#[test]
fn register_duplicate_returns_error() {
    let mut registry = ActionRegistry::new();
    let contract = test_contract(3);
    assert_eq!(registry.register(contract), Ok(()));
    let duplicate = test_contract(3);
    assert_eq!(registry.register(duplicate), Err(ActionError::DispatchFailed));
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
    assert_eq!(registry.register(test_contract(1)), Ok(()));
    assert_eq!(registry.register(test_contract(5)), Ok(()));
    assert_eq!(registry.len(), 6);
}

#[test]
fn validate_input_bytes_rejects_when_max_input_bytes_is_zero() {
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(1),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
    };
    assert_eq!(registry.register(contract), Ok(()));
    let input = test_input(1);
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
    let mut registry = ActionRegistry::new();
    let contract = test_contract(5);
    assert_eq!(registry.register(contract), Ok(()));
    let result = registry.resolve_compile_time(ActionId::new(5));
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
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(10)), Ok(()));
    assert_eq!(registry.len(), 11);
    let resolved = registry.resolve_compile_time(ActionId::new(10));
    assert_eq!(resolved.map(|c| c.id), Ok(ActionId::new(10)));
}

#[test]
fn action_registry_dispatch_rejects_mismatched_contract() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(5)), Ok(()));
    let input = test_input(5);
    let wrong_contract = ActionContract {
        id: ActionId::new(3),
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
    let result = registry.dispatch(&input, &wrong_contract);
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(5)
        })
    );
}

#[test]
fn action_registry_is_not_empty_after_register() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(1)), Ok(()));
    assert_eq!(registry.is_empty(), false);
}

#[test]
fn action_registry_new_default_matches_new() {
    let default = ActionRegistry::default();
    let new = ActionRegistry::new();
    assert_eq!(default.is_empty(), true);
    assert_eq!(new.is_empty(), true);
    assert_eq!(default.len(), 0);
    assert_eq!(new.len(), 0);
}

#[test]
fn action_registry_register_multiple_actions() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(0)), Ok(()));
    assert_eq!(registry.register(test_contract(1)), Ok(()));
    assert_eq!(registry.register(test_contract(2)), Ok(()));
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
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(0)), Ok(()));
    let result = registry.resolve_compile_time(ActionId::new(5));
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(5)
        })
    );
}

#[test]
fn action_registry_dispatch_with_correct_contract_succeeds() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(0)), Ok(()));
    let input = test_input(0);
    let contract = test_contract(0);
    let result = registry.dispatch(&input, &contract);
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
                }))
            );
        }
    }
}

#[test]
fn action_contract_fields_are_preserved() {
    let contract = ActionContract {
        id: ActionId::new(42),
        input_slot_count: 3,
        output_slot_count: 2,
        max_input_bytes: 2048,
        max_output_bytes: 4096,
        timeout_ms: 10000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(contract), Ok(()));
    let resolved = registry.resolve_compile_time(ActionId::new(42));
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
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(5)), Ok(()));
    assert_eq!(registry.len(), 6);
}

#[test]
fn action_registry_gap_slot_resolves_for_default_id() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(5)), Ok(()));
    let result = registry.resolve_compile_time(ActionId::new(0));
    match result {
        Ok(c) => {
            assert_eq!(c.id, ActionId::new(0));
        }
        Err(_) => {
            assert!(false);
        }
    }
}

#[test]
fn action_registry_gap_slot_nondefault_id_fails() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(5)), Ok(()));
    let result = registry.resolve_compile_time(ActionId::new(3));
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(3)
        })
    );
}

#[test]
fn action_registry_dispatch_unknown_action_returns_exact_error_variant() {
    let registry = ActionRegistry::new();
    let input = test_input(99);
    let contract = test_contract(99);
    let result = registry.dispatch(&input, &contract);
    assert_eq!(
        result,
        Err(ActionError::UnknownAction {
            action: ActionId::new(99)
        })
    );
}

#[test]
fn action_registry_register_then_reregister_same_id_returns_dispatch_failed() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(1)), Ok(()));
    let result = registry.register(test_contract(1));
    assert_eq!(result, Err(ActionError::DispatchFailed));
}

#[test]
fn action_registry_register_max_action_id_does_not_overflow() {
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(65534),
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
    let result = registry.register(contract);
    assert_eq!(result, Ok(()));
    assert_eq!(registry.len(), 65535);
}

#[test]
fn action_registry_validate_input_bytes_rejects_zero_with_slots() {
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(1),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
    };
    assert_eq!(registry.register(contract), Ok(()));
    let input = test_input(1);
    let resolved = registry.resolve_compile_time(ActionId::new(1));
    let contract = match resolved {
        Ok(c) => c.clone(),
        Err(_) => return,
    };
    let result = registry.dispatch(&input, &contract);
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
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(2),
        input_slot_count: 0,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
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
        },
    };
    let contract = match registry.resolve_compile_time(ActionId::new(2)) {
        Ok(c) => c.clone(),
        Err(_) => return,
    };
    let result = registry.dispatch(&input, &contract);
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
                }))
            );
        }
    }
}

#[test]
fn action_registry_resolve_after_many_registrations_finds_correct_action() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(0)), Ok(()));
    assert_eq!(registry.register(test_contract(5)), Ok(()));
    assert_eq!(registry.register(test_contract(10)), Ok(()));
    assert_eq!(registry.register(test_contract(20)), Ok(()));
    let result = registry.resolve_compile_time(ActionId::new(10));
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
fn action_registry_dispatch_returns_ticket_with_correct_action_from_input() {
    let mut registry = ActionRegistry::new();
    assert_eq!(registry.register(test_contract(3)), Ok(()));
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
        },
    };
    let contract = match registry.resolve_compile_time(ActionId::new(3)) {
        Ok(c) => c.clone(),
        Err(_) => return,
    };
    let result = registry.dispatch(&input, &contract);
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
                }))
            );
        }
    }
}
