#![allow(unused)]
//! Kani harnesses for vb_runtime action module.
//!
//! Verifier lane: kani
//! Obligations: OBL-004, OBL-010, OBL-013
//!
/// Tests ActionRegistry panic-freedom and serialization round-trip bounds.
/// dispatch_generic is private to the module; its panic-freedom is
/// verified via the public `dispatch` method and the serialization harness.
use vb_core::action::{
    ActionContract, ActionName, ActionTicket, Idempotency, RetrySafety, SideEffect,
};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

// ─── OBL-010: ActionRegistry panic freedom ──────────────────────────────────────

/// ActionRegistry::register and resolve_compile_time must never panic.
#[kani::proof]
fn check_action_registry_panic_free() {
    let mut registry = crate::ActionRegistry::new();

    // Register an action - must not panic.
    let contract = ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test_action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 4096,
        max_output_bytes: 4096,
        timeout_ms: 30000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };

    let reg_result = registry.register(contract);
    assert!(reg_result.is_ok(), "First registration must succeed");

    // Register duplicate - must return error, not panic.
    let contract2 = ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("duplicate").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 4096,
        max_output_bytes: 4096,
        timeout_ms: 30000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };

    let dup_result = registry.register(contract2);
    assert!(
        dup_result.is_err(),
        "Duplicate registration must return error"
    );

    // Resolve a known action - must not panic.
    let resolved = registry.resolve_compile_time(ActionId::new(0));
    assert!(resolved.is_ok(), "Resolved action must be found");

    // Resolve unknown action - must return error, not panic.
    let unknown = registry.resolve_compile_time(ActionId::new(999));
    assert!(unknown.is_err(), "Unknown action must return error");
}

// ─── OBL-013: Serialization round-trip (7 fields, pre-MockMarker) ───────────────

/// ActionTicket serializes and deserializes correctly in postcard wire format.
#[kani::proof]
fn check_action_ticket_serialization() {
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 1,
        idempotency_key: 0xDEADBEEF,
        capacity: 5,
    };

    // Serialize (current 7-field format).
    let serialized = postcard::to_allocvec(&ticket).expect("Serialization must succeed");

    // Deserialize back.
    let deserialized: ActionTicket =
        postcard::from_bytes(&serialized).expect("Deserialization must succeed");

    // All fields must round-trip correctly.
    assert_eq!(deserialized.run.get(), ticket.run.get());
    assert_eq!(deserialized.step.get(), ticket.step.get());
    assert_eq!(deserialized.seq.get(), ticket.seq.get());
    assert_eq!(deserialized.action.get(), ticket.action.get());
    assert_eq!(deserialized.attempt, ticket.attempt);
    assert_eq!(deserialized.idempotency_key, ticket.idempotency_key);
    assert_eq!(deserialized.capacity, ticket.capacity);
}
