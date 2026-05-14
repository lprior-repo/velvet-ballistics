#![forbid(unsafe_code)]
#![cfg(test)]

use vb_core::action::{
    ActionContract, ActionError, Idempotency, SideEffect, RetrySafety,
};
use vb_core::ids::ActionId;
use vb_runtime::action::ActionRegistry;

fn make_contract(id: ActionId) -> ActionContract {
    ActionContract {
        id,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

#[test]
fn test_registry_register_single_contract() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(1));
    let result = registry.register(contract.clone());
    assert!(result.is_ok(), "register should succeed for new contract");
    let resolved = registry.resolve_compile_time(contract.id);
    assert!(resolved.is_ok(), "resolve should return Ok after register");
    assert_eq!(resolved.unwrap().id, contract.id, "contract id should match");
}

#[test]
fn test_registry_register_returns_registered_contract() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(2));
    registry.register(contract.clone()).expect("register should succeed");
    let resolved = registry.resolve_compile_time(contract.id);
    let resolved_contract = resolved.expect("resolve should return Ok");
    assert_eq!(resolved_contract.id, contract.id, "contract id should match exactly");
    assert_eq!(resolved_contract.idempotency, contract.idempotency, "idempotency should match");
}

#[test]
fn test_registry_register_idempotent_empty_slot() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(3));
    let first = registry.register(contract.clone());
    assert!(first.is_ok(), "first register should succeed");
}

#[test]
fn test_registry_register_duplicate_on_occupied_slot_fails() {
    let mut registry = ActionRegistry::new();
    let contract1 = make_contract(ActionId::new(4));
    let contract2 = ActionContract {
        id: ActionId::new(4),
        input_slot_count: 2,
        output_slot_count: 2,
        max_input_bytes: 2048,
        max_output_bytes: 2048,
        timeout_ms: 2000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    registry.register(contract1).expect("first register should succeed");
    let second = registry.register(contract2);
    assert!(second.is_err(), "re-registering occupied slot should fail");
    let err = second.unwrap_err();
    assert_eq!(err, ActionError::DispatchFailed, "should return DispatchFailed");
}

#[test]
fn test_registry_register_action_id_at_max_u16_boundary() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(65534));
    let result = registry.register(contract);
    assert!(result.is_ok(), "register with ActionId(65534) should succeed");
}

#[test]
fn test_registry_register_action_id_at_max_u16_plus_one_fails() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(65535));
    let result = registry.register(contract);
    assert!(result.is_err(), "register with ActionId(65535) should fail");
}

#[test]
fn test_registry_len_after_single_register() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(1));
    registry.register(contract).expect("register should succeed");
    assert_eq!(registry.len(), 1, "len should be 1 after single register");
}

#[test]
fn test_registry_len_with_sparse_ids() {
    let mut registry = ActionRegistry::new();
    registry.register(make_contract(ActionId::new(0))).expect("register 0 succeeds");
    registry.register(make_contract(ActionId::new(100))).expect("register 100 succeeds");
    assert_eq!(registry.len(), 101, "len should be 101 for sparse ids");
}

#[test]
fn test_registry_registered_contracts_returns_ascending_order() {
    let mut registry = ActionRegistry::new();
    registry.register(make_contract(ActionId::new(50))).expect("register 50 succeeds");
    registry.register(make_contract(ActionId::new(10))).expect("register 10 succeeds");
    registry.register(make_contract(ActionId::new(30))).expect("register 30 succeeds");
    let contracts = registry.registered_contracts();
    let ids: Vec<_> = contracts.iter().map(|c| c.id).collect();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort();
    assert_eq!(ids, sorted_ids, "contracts should be returned in ascending order");
}

#[test]
fn test_registry_resolve_unknown_action_returns_error() {
    let mut registry = ActionRegistry::new();
    let result = registry.resolve_compile_time(ActionId::new(999));
    assert!(result.is_err(), "resolve on unknown action should fail");
    let err = result.unwrap_err();
    assert_eq!(err, ActionError::UnknownAction { action: ActionId::new(999) });
}

#[test]
fn test_registry_resolve_registered_returns_ok() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(7));
    registry.register(contract.clone()).expect("register should succeed");
    let result = registry.resolve_compile_time(contract.id);
    assert!(result.is_ok(), "resolve on registered action should return Ok");
}

#[test]
fn test_registry_dispatch_unknown_action_returns_error() {
    let mut registry = ActionRegistry::new();
    use vb_core::action::{ActionInput, ActionTicket};
    use vb_core::ids::{RunId, SeqNo, SlotIdx, StepIdx};
    let dummy_contract = make_contract(ActionId::new(42));
    let input = ActionInput {
        run: RunId::new(1),
        step: StepIdx::new(0),
        action: ActionId::new(42),
        input: SlotIdx::new(0),
        ticket: ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(42),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        },
    };
    let result = registry.dispatch(&input, &dummy_contract);
    assert!(result.is_err(), "dispatch on unknown action should fail");
    let err = result.unwrap_err();
    assert_eq!(err, ActionError::UnknownAction { action: ActionId::new(42) });
}

#[test]
fn test_registry_dispatch_with_zero_max_bytes_and_nonzero_slots_fails() {
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(8),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 0,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    registry.register(contract.clone()).expect("register should succeed");
    use vb_core::action::{ActionInput, ActionTicket};
    use vb_core::ids::{RunId, SeqNo, SlotIdx, StepIdx};
    let input = ActionInput {
        run: RunId::new(1),
        step: StepIdx::new(0),
        action: ActionId::new(8),
        input: SlotIdx::new(0),
        ticket: ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(8),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        },
    };
    let result = registry.dispatch(&input, &contract);
    assert!(result.is_err(), "dispatch with zero max_bytes should fail");
    match result {
        Err(ActionError::PayloadTooLarge { max_bytes, actual_bytes: _ }) => {
            assert_eq!(max_bytes, 0, "max_bytes should be 0");
        }
        _ => panic!("expected PayloadTooLarge error"),
    }
}
