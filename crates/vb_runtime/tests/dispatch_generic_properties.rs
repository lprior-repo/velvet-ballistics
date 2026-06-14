//! Properties of `dispatch_generic` that hold regardless of feature flags.
//!
//! Verifies that dispatch produces Suspended outcome, preserves fields,
//! sets capacity to 1, is idempotent, and rejects zero max_input_bytes.
//!
//! Tests go through `ActionRegistry::dispatch` (the public API), which
//! internally calls `dispatch_generic`.

#![forbid(unsafe_code)]

use vb_core::action::{
    ActionContract, ActionError, ActionInput, ActionName, ActionOutcome, ActionTicket,
};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_runtime::action::ActionRegistry;
use vb_core::action::{Idempotency, RetrySafety, SideEffect};

fn make_contract(id: u16, name: &str) -> ActionContract {
    ActionContract {
        id: ActionId::new(id),
        name: ActionName::new(name).unwrap(),
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

fn make_input(action_id: u16) -> ActionInput {
    ActionInput {
        run: RunId::new(1),
        step: StepIdx::new(0),
        action: ActionId::new(action_id),
        input: SlotIdx::new(0),
        ticket: ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(42),
            action: ActionId::new(action_id),
            attempt: 3,
            idempotency_key: 0xBEEF,
            capacity: 999,
        },
    }
}

fn make_input_with_capacity(action_id: u16, capacity: u16) -> ActionInput {
    let mut input = make_input(action_id);
    input.ticket.capacity = capacity;
    input
}

fn register_action(registry: &mut ActionRegistry, id: u16, name: &str) -> ActionContract {
    let contract = make_contract(id, name);
    registry.register(contract.clone()).expect("register must succeed");
    contract
}

// ---------------------------------------------------------------------------
// Suspended outcome
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_produces_suspended_outcome() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 5, "test.action");
    let input = make_input(5);

    let result = registry.dispatch(&input, &contract);
    match result {
        Ok(ActionOutcome::Suspended(ticket)) => {
            assert_eq!(
                ticket.action,
                ActionId::new(5),
                "dispatched ticket action must match input action"
            );
        }
        other => panic!("Expected Ok(Suspended), got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Field preservation
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_preserves_run() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 1, "test.action");
    let input = make_input(1);

    let result = registry.dispatch(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.run,
                input.run,
                "dispatch must preserve run from input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_step() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 1, "test.action");
    let input = make_input(1);

    let result = registry.dispatch(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.step,
                input.step,
                "dispatch must preserve step from input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_ticket_seq() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 1, "test.action");
    let input = make_input(1);

    let result = registry.dispatch(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.seq,
                input.ticket.seq,
                "dispatch must preserve ticket seq from input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_ticket_action() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 1, "test.action");
    let input = make_input(1);

    let result = registry.dispatch(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.action,
                input.action,
                "dispatch must preserve action from input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_ticket_attempt() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 1, "test.action");
    let input = make_input(1);

    let result = registry.dispatch(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.attempt,
                input.ticket.attempt,
                "dispatch must preserve attempt from input ticket"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_preserves_ticket_idempotency_key() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 1, "test.action");
    let input = make_input(1);

    let result = registry.dispatch(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.idempotency_key,
                input.ticket.idempotency_key,
                "dispatch must preserve idempotency_key from input ticket"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Capacity handling
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_capacity_set_to_one() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 1, "test.action");
    let input = make_input(1);

    let result = registry.dispatch(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.capacity,
                1,
                "dispatch must set capacity to 1 regardless of input"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

#[test]
fn test_dispatch_capacity_override_from_high_value() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 1, "test.action");
    let input = make_input_with_capacity(1, 9999);

    let result = registry.dispatch(&input, &contract).unwrap();
    match result {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.capacity,
                1,
                "dispatch must override input capacity to 1"
            );
            assert_ne!(
                ticket.capacity, 9999,
                "capacity must NOT be the input capacity"
            );
        }
        other => panic!("Expected Suspended, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Idempotence
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_is_idempotent() {
    let mut registry = ActionRegistry::new();
    let contract = register_action(&mut registry, 1, "test.action");
    let input = make_input(1);

    let result1 = registry.dispatch(&input, &contract).unwrap();
    let result2 = registry.dispatch(&input, &contract).unwrap();

    match (result1, result2) {
        (ActionOutcome::Suspended(t1), ActionOutcome::Suspended(t2)) => {
            assert_eq!(
                t1, t2,
                "dispatch must be idempotent: same input produces same outcome"
            );
        }
        (r1, r2) => panic!("Expected two Suspended outcomes, got {r1:?}, {r2:?}"),
    }
}

// ---------------------------------------------------------------------------
// Payload rejection
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_zero_max_input_bytes_rejects() {
    let mut registry = ActionRegistry::new();
    let input = make_input(1);
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test.action").unwrap(),
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

    registry.register(contract.clone()).expect("register must succeed");

    let result = registry.dispatch(&input, &contract);
    assert_eq!(
        result,
        Err(ActionError::PayloadTooLarge {
            max_bytes: 0,
            actual_bytes: 0
        }),
        "dispatch must reject when max_input_bytes is 0 and input_slot_count > 0"
    );
}

// ---------------------------------------------------------------------------
// Multiple action names
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_with_different_action_names() {
    let mut registry = ActionRegistry::new();

    // Explicit IDs to avoid collisions.
    let actions: &[(&str, u16)] = &[
        ("test.action", 100),
        ("github.issue.create", 101),
        ("ai.classify", 102),
        ("http.request", 103),
    ];

    for (name, id) in actions {
        let contract = register_action(&mut registry, *id, name);

        let input = ActionInput {
            run: RunId::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(*id),
            input: SlotIdx::new(0),
            ticket: ActionTicket {
                run: RunId::new(1),
                step: StepIdx::new(0),
                seq: SeqNo::new(1),
                action: ActionId::new(*id),
                attempt: 1,
                idempotency_key: 0,
                capacity: 1,
            },
        };

        let result = registry.dispatch(&input, &contract).unwrap();
        match result {
            ActionOutcome::Suspended(ticket) => {
                assert_eq!(
                    ticket.action,
                    ActionId::new(*id),
                    "dispatch must work for action name '{name}'"
                );
                assert_eq!(
                    ticket.capacity,
                    1,
                    "capacity must be 1 for action name '{name}'"
                );
            }
            other => panic!("Expected Suspended for '{name}', got {other:?}"),
        }
    }
}
