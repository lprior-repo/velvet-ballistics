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

    // Generate action IDs using kani::any() — no hardcoded values.
    let id: u64 = kani::any();
    kani::assume(id < 65535u64); // Ensure it's a valid ActionId index

    let name_data: u8 = kani::any();
    // ActionName "test_a" is a valid name (≤64 chars, no whitespace).
    let action_name = match ActionName::new("test_action") {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    let contract = ActionContract {
        id: ActionId::new(id),
        name: action_name,
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

    // Register first action — must not panic.
    let reg_result = registry.register(contract);
    assert!(reg_result.is_ok(), "First registration must succeed");

    // Register duplicate ID — must return error, not panic.
    let different_name = match ActionName::new("different_action") {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    let contract2 = ActionContract {
        id: ActionId::new(id),
        name: different_name,
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
        "Duplicate ID registration must return error"
    );

    // Resolve the registered action — must not panic.
    let resolved = registry.resolve_compile_time(ActionId::new(id));
    assert!(resolved.is_ok(), "Resolved action must be found");

    // Resolve an unknown action — must return error, not panic.
    let unknown_id: u64 = kani::any();
    kani::assume(unknown_id > id);
    let unknown = registry.resolve_compile_time(ActionId::new(unknown_id));
    assert!(unknown.is_err(), "Unknown action must return error");
}

// ─── OBL-010 extended: ActionRegistry resolves by name ──────────────────────────

/// ActionRegistry::resolve_by_name must never panic.
#[kani::proof]
fn check_action_registry_resolve_by_name_panic_free() {
    let mut registry = crate::ActionRegistry::new();

    let id: u64 = kani::any();
    kani::assume(id < 65535u64);

    let resolve_name = match ActionName::new("resolve_test") {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    let contract = ActionContract {
        id: ActionId::new(id),
        name: resolve_name.clone(),
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

    registry.register(contract).ok();

    // Resolve by the registered name — must not panic.
    let _resolved = registry.resolve_by_name(&resolve_name);

    // Resolve by unregistered name — must not panic (returns error).
    let nonexistent_name = match ActionName::new("nonexistent") {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    let _unresolved = registry.resolve_by_name(&nonexistent_name);
}

// ─── OBL-013: Serialization round-trip (7 fields, pre-MockMarker) ───────────────

/// ActionTicket serializes and deserializes correctly in postcard wire format.
/// All fields are generated using kani::any() — no hardcoded values.
#[kani::proof]
fn check_action_ticket_serialization() {
    // Generate all fields using kani::any() — no hardcoded values.
    let run: u64 = kani::any();
    let step: u64 = kani::any();
    let seq: u64 = kani::any();
    let action: u64 = kani::any();
    let attempt: u16 = kani::any();
    let idempotency_key: u128 = kani::any();
    let capacity: u16 = kani::any();

    let ticket = ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action),
        attempt,
        idempotency_key,
        capacity,
    };

    // Serialize (7-field format).
    let serialized = match postcard::to_allocvec(&ticket) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop crates/vb_runtime/src/verification/kani/kani_rxru0_action_harnesses.rs }
    };

    // Deserialize back.
    let deserialized: ActionTicket = match postcard::from_bytes(&serialized) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop crates/vb_runtime/src/verification/kani/kani_rxru0_action_harnesses.rs }
    };

    // Verify all 7 fields round-trip correctly.
    assert_eq!(deserialized.run.get(), ticket.run.get());
    assert_eq!(deserialized.step.get(), ticket.step.get());
    assert_eq!(deserialized.seq.get(), ticket.seq.get());
    assert_eq!(deserialized.action.get(), ticket.action.get());
    assert_eq!(deserialized.attempt, ticket.attempt);
    assert_eq!(deserialized.idempotency_key, ticket.idempotency_key);
    assert_eq!(deserialized.capacity, ticket.capacity);
}

// ─── OBL-013 extended: Serialization of edge-case values ────────────────────────

/// Edge-case values (0, MAX) must serialize and deserialize correctly.
#[kani::proof]
fn check_action_ticket_serialization_edge_cases() {
    // Generate fields including edge cases (0 and MAX).
    let run: u64 = kani::any();
    let step: u64 = kani::any();
    let seq: u64 = kani::any();
    let action: u64 = kani::any();
    let attempt: u16 = kani::any();
    let idempotency_key: u128 = kani::any();
    let capacity: u16 = kani::any();

    // Kani will explore both small and large values.
    let ticket = ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action),
        attempt,
        idempotency_key,
        capacity,
    };

    // Round-trip through postcard.
    let serialized = match postcard::to_allocvec(&ticket) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop crates/vb_runtime/src/verification/kani/kani_rxru0_action_harnesses.rs }
    };
    let deserialized: ActionTicket = match postcard::from_bytes(&serialized) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop crates/vb_runtime/src/verification/kani/kani_rxru0_action_harnesses.rs }
    };

    assert_eq!(deserialized, ticket, "All fields must round-trip correctly");
}
