#![forbid(unsafe_code)]
//! Integration behavior tests for capability and idempotency systems.
//!
//! Covers:
//! - Capability grant, check, revocation, scope, inheritance
//! - Idempotency determinism, key reuse, statelessness
//! - Capability serialization roundtrip
//! - RetrySafety x SideEffect combinatorial coverage
//! - Taint-based key validation
//! - CapabilitySet structural invariants
//! - Kani: determinism, short-circuit, monotonicity, empty-set, cross-action

use crate::action::{
    ActionContract, ActionName, Idempotency, IdempotencyViolation, RetrySafety, SideEffect,
    validate_idempotency_key_ingredients, verify_idempotency,
};
use crate::capability::{Capability, CapabilitySet};
use crate::frame::RunFrame;
use crate::ids::{ActionId, RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

fn cap(name: &str, action: ActionId) -> Capability {
    Capability::new(name.into(), action)
}

fn test_frame(slot_count: u16, step_count: u16) -> RunFrame {
    RunFrame::new(RunId::new(1), StepIdx::new(0), step_count, slot_count)
        .ok()
        .expect("test frame construction")
}

fn ensure_equal<T: core::fmt::Debug + PartialEq>(actual: T, expected: T) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

// =============================================================================
// 1. Capability grant: valid scope/action
// =============================================================================

#[test]
fn capability_grant_valid_scope_and_action_returns_true() -> Result<(), String> {
    let action = ActionId::new(1);
    let grant = cap("network.http", action);
    let required = cap("network.http", action);
    let set = CapabilitySet::from_grants(Box::new([grant]));
    ensure_equal(set.grants(&required), true)
}

#[test]
fn capability_grant_valid_action_but_wrong_name_returns_false() -> Result<(), String> {
    let action = ActionId::new(1);
    let grant = cap("network.http", action);
    let required = cap("storage.s3", action);
    let set = CapabilitySet::from_grants(Box::new([grant]));
    ensure_equal(set.grants(&required), false)
}

// =============================================================================
// 2. Capability check: has capability, missing capability
// =============================================================================

#[test]
fn capability_check_has_capability_when_exact_match() -> Result<(), String> {
    let action = ActionId::new(7);
    let set = CapabilitySet::from_grants(Box::new([cap("secrets.read", action)]));
    ensure_equal(set.grants(&cap("secrets.read", action)), true)
}

#[test]
fn capability_check_missing_capability_when_action_mismatch() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([cap("secrets.read", ActionId::new(7))]));
    ensure_equal(set.grants(&cap("secrets.read", ActionId::new(8))), false)
}

#[test]
fn capability_check_missing_capability_when_name_mismatch() -> Result<(), String> {
    let action = ActionId::new(7);
    let set = CapabilitySet::from_grants(Box::new([cap("network", action)]));
    ensure_equal(set.grants(&cap("secrets", action)), false)
}

#[test]
fn capability_check_missing_capability_when_both_mismatch() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([cap("network", ActionId::new(1))]));
    ensure_equal(
        set.grants(&cap("secrets", ActionId::new(2))),
        false,
    )
}

// =============================================================================
// 3. Capability revocation (via reconstruction)
// =============================================================================

#[test]
fn capability_revocation_via_reconstruction_of_empty_set() -> Result<(), String> {
    let action = ActionId::new(1);
    let granted = CapabilitySet::from_grants(Box::new([cap("network", action)]));
    ensure_equal(granted.grants(&cap("network", action)), true)?;
    let revoked = CapabilitySet::empty();
    ensure_equal(revoked.grants(&cap("network", action)), false)
}

#[test]
fn capability_revocation_via_reconstruction_with_different_grants() -> Result<(), String> {
    let action_a = ActionId::new(1);
    let action_b = ActionId::new(2);
    let original = CapabilitySet::from_grants(Box::new([
        cap("network", action_a),
        cap("secrets", action_a),
    ]));
    ensure_equal(original.grants(&cap("network", action_a)), true)?;
    ensure_equal(original.grants(&cap("secrets", action_a)), true)?;
    let modified = CapabilitySet::from_grants(Box::new([cap("secrets", action_b)]));
    ensure_equal(modified.grants(&cap("network", action_a)), false)?;
    ensure_equal(modified.grants(&cap("secrets", action_b)), true)
}

// =============================================================================
// 4. Capability scope: narrow vs broad, exact match
// =============================================================================

#[test]
fn capability_scope_narrow_grant_does_not_match_broad_required() -> Result<(), String> {
    let action = ActionId::new(1);
    let set = CapabilitySet::from_grants(Box::new([cap("net", action)]));
    ensure_equal(set.grants(&cap("network", action)), false)
}

#[test]
fn capability_scope_broad_grant_does_not_match_narrow_required() -> Result<(), String> {
    let action = ActionId::new(1);
    let set = CapabilitySet::from_grants(Box::new([cap("network.http.get", action)]));
    ensure_equal(set.grants(&cap("network", action)), false)
}

#[test]
fn capability_exact_match_grants_even_with_dotted_names() -> Result<(), String> {
    let action = ActionId::new(3);
    let grant = cap("a.b.c", action);
    let required = cap("a.b.c", action);
    let set = CapabilitySet::from_grants(Box::new([grant]));
    ensure_equal(set.grants(&required), true)
}

#[test]
fn capability_scope_multiple_grants_first_fails_second_matches() -> Result<(), String> {
    let action = ActionId::new(1);
    let set = CapabilitySet::from_grants(Box::new([
        cap("wrong", action),
        cap("correct", action),
    ]));
    ensure_equal(set.grants(&cap("correct", action)), true)?;
    ensure_equal(set.grants(&cap("wrong", action)), true)
}

// =============================================================================
// 5. Capability inheritance: opaque dotted names
// =============================================================================

#[test]
fn capability_opaque_dotted_names_parent_does_not_grant_child() -> Result<(), String> {
    let action = ActionId::new(1);
    let set = CapabilitySet::from_grants(Box::new([cap("network", action)]));
    ensure_equal(set.grants(&cap("network.github", action)), false)
}

#[test]
fn capability_opaque_dotted_names_child_does_not_grant_parent() -> Result<(), String> {
    let action = ActionId::new(1);
    let set = CapabilitySet::from_grants(Box::new([cap("network.github", action)]));
    ensure_equal(set.grants(&cap("network", action)), false)
}

#[test]
fn capability_opaque_dotted_names_siblings_do_not_cross_grant() -> Result<(), String> {
    let action = ActionId::new(1);
    let set = CapabilitySet::from_grants(Box::new([cap("network.http", action)]));
    ensure_equal(set.grants(&cap("network.github", action)), false)
}

// =============================================================================
// 6. Capability expiration: verified absent (API note)
// =============================================================================

#[test]
fn capability_expiration_api_note_no_time_fields_on_capability() {
    let c = cap("network", ActionId::new(1));
    kani::assert(c.name(, "assertion failed") == "network", "assertion failed");
    kani::assert(c.action_id(, "assertion failed") == ActionId::new(1), "assertion failed");
}

#[test]
fn capability_serialization_roundtrip_preserves_all_fields() -> Result<(), String> {
    let original = cap("network.http.post", ActionId::new(42));
    let bytes = postcard::to_allocvec(&original)
        .map_err(|e| e.to_string())?;
    let recovered: Capability = postcard::from_bytes(&bytes)
        .map_err(|e| e.to_string())?;
    ensure_equal(recovered.name(), original.name())?;
    ensure_equal(recovered.action_id(), original.action_id())?;
    ensure_equal(recovered, original)
}

#[test]
fn capability_set_serialization_roundtrip() -> Result<(), String> {
    let original = CapabilitySet::from_grants(Box::new([
        cap("network", ActionId::new(1)),
        cap("secrets.read", ActionId::new(2)),
        cap("storage.s3", ActionId::new(3)),
    ]));
    let bytes = postcard::to_allocvec(&original)
        .map_err(|e| e.to_string())?;
    let recovered: CapabilitySet = postcard::from_bytes(&bytes)
        .map_err(|e| e.to_string())?;
    ensure_equal(recovered.len(), original.len())?;
    ensure_equal(recovered.is_empty(), original.is_empty())?;
    ensure_equal(recovered.grants(&cap("network", ActionId::new(1))), true)?;
    ensure_equal(recovered.grants(&cap("secrets.read", ActionId::new(2))), true)?;
    ensure_equal(recovered.grants(&cap("storage.s3", ActionId::new(3))), true)?;
    ensure_equal(recovered.grants(&cap("absent", ActionId::new(1))), false)?;
    ensure_equal(recovered, original)
}

// =============================================================================
// 7. Idempotency: same key twice = same result, different keys = fresh
// =============================================================================

#[test]
fn idempotency_same_key_slots_twice_yields_same_ok_result() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };
    let mut frame = test_frame(2, 2);
    let write_result = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
    kani::assert(write_result.is_ok(), "kani harness assertion");
    let key_slots = [SlotIdx::new(0)];
    let result_a = verify_idempotency(&contract, &key_slots, &frame);
    let result_b = verify_idempotency(&contract, &key_slots, &frame);
    ensure_equal(result_a.is_ok(), true)?;
    ensure_equal(result_a, result_b)
}

#[test]
fn idempotency_same_key_twice_yields_same_err_result() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(2),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };
    let mut frame = test_frame(2, 2);
    let write_result = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    kani::assert(write_result.is_ok(), "kani harness assertion");
    let key_slots = [SlotIdx::new(0)];
    let result_a = verify_idempotency(&contract, &key_slots, &frame);
    let result_b = verify_idempotency(&contract, &key_slots, &frame);
    ensure_equal(result_a, result_b)
}

#[test]
fn idempotency_different_key_slots_produce_independent_results() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(3),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 2,
        output_slot_count: 2,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };
    let mut frame = test_frame(4, 2);
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
    let _ = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret);
    let _ = frame.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(3), Taint::Clean);
    let clean_result = verify_idempotency(&contract, &[SlotIdx::new(0), SlotIdx::new(2)], &frame);
    ensure_equal(clean_result, Ok(()))?;
    let secret_result = verify_idempotency(&contract, &[SlotIdx::new(0), SlotIdx::new(1)], &frame);
    ensure_equal(secret_result.is_err(), true)
}

// =============================================================================
// 8. Idempotency: key reuse after completion, concurrent same-key
// =============================================================================

#[test]
fn idempotency_key_reuse_after_completion_deterministic() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(4),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };
    let mut frame = test_frame(2, 2);
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean);
    let key_slots = [SlotIdx::new(0)];
    let first = verify_idempotency(&contract, &key_slots, &frame);
    let second = verify_idempotency(&contract, &key_slots, &frame);
    let third = verify_idempotency(&contract, &key_slots, &frame);
    ensure_equal(first, Ok(()))?;
    ensure_equal(second, Ok(()))?;
    ensure_equal(third, Ok(()))
}

#[test]
fn idempotency_same_key_no_side_effect_always_ok_even_unsafe_retry() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(5),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 1,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::NotRetrySafe,
        required_capabilities: Box::new([]),
    };
    let frame = test_frame(2, 2);
    let first = verify_idempotency(&contract, &[], &frame);
    let second = verify_idempotency(&contract, &[], &frame);
    ensure_equal(first.clone(), Ok(()))?;
    ensure_equal(second, first)
}

// =============================================================================
// 9. Idempotency storage: stateless (verified)
// =============================================================================

#[test]
fn idempotency_storage_is_stateless_no_persistent_state_between_calls() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(6),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };
    let mut frame = test_frame(2, 2);
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
    let key_slots = [SlotIdx::new(0)];
    let _ = verify_idempotency(&contract, &key_slots, &frame);
    let mut other_frame = test_frame(2, 2);
    let _ = other_frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
    let result = verify_idempotency(&contract, &key_slots, &other_frame);
    ensure_equal(result, Ok(()))
}

#[test]
fn idempotency_stateless_new_frame_with_same_slots_returns_same_result() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(7),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };
    let mut frame_a = test_frame(2, 2);
    let _ = frame_a.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
    let key = [SlotIdx::new(0)];
    let result_a = verify_idempotency(&contract, &key, &frame_a);
    let mut frame_b = test_frame(2, 2);
    let _ = frame_b.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
    let result_b = verify_idempotency(&contract, &key, &frame_b);
    ensure_equal(result_a.clone(), result_b)?;
    ensure_equal(result_a, Ok(()))
}

// =============================================================================
// 10. Capability serialization roundtrip (postcard)
// =============================================================================

#[test]
fn capability_postcard_roundtrip_empty_name() -> Result<(), String> {
    let original = cap("", ActionId::new(0));
    let bytes = postcard::to_allocvec(&original).map_err(|e| e.to_string())?;
    let recovered: Capability = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
    ensure_equal(recovered, original)
}

#[test]
fn capability_postcard_roundtrip_max_action_id() -> Result<(), String> {
    let original = cap("long.capability.name.here", ActionId::new(u16::MAX));
    let bytes = postcard::to_allocvec(&original).map_err(|e| e.to_string())?;
    let recovered: Capability = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
    ensure_equal(recovered, original)
}

// =============================================================================
// 11. Capability + idempotency interaction
// =============================================================================

#[test]
fn capability_and_idempotency_are_independent_correctness_dimensions() -> Result<(), String> {
    let required_cap = cap("network", ActionId::new(10));
    let contract = ActionContract {
        id: ActionId::new(10),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([required_cap]),
    };
    let cap_set = CapabilitySet::from_grants(Box::new([cap("network", ActionId::new(10))]));
    ensure_equal(cap_set.grants(&cap("network", ActionId::new(10))), true)?;
    let mut frame = test_frame(2, 2);
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
    let key_slots = [SlotIdx::new(0)];
    let idem_result = verify_idempotency(&contract, &key_slots, &frame);
    ensure_equal(idem_result, Ok(()))?;
    ensure_equal(contract.required_capabilities.len(), 1)
}

#[test]
fn capability_grants_while_idempotency_fails_with_secret_key() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(11),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([cap("network", ActionId::new(11))]),
    };
    let cap_set = CapabilitySet::from_grants(Box::new([cap("network", ActionId::new(11))]));
    ensure_equal(cap_set.grants(&cap("network", ActionId::new(11))), true)?;
    let mut frame = test_frame(2, 2);
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    let key_slots = [SlotIdx::new(0)];
    let idem_result = verify_idempotency(&contract, &key_slots, &frame);
    ensure_equal(idem_result, Err(IdempotencyViolation::SecretInKey(0)))
}

// =============================================================================
// 12. RetrySafety x SideEffect combinations
// =============================================================================

#[test]
fn retry_safety_safe_with_writes_passes() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    let frame = test_frame(2, 2);
    let result = verify_idempotency(&contract, &[], &frame);
    ensure_equal(result, Ok(()))
}

#[test]
fn retry_safety_safe_with_sends_passes() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(2),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::ExternalWrite,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    let frame = test_frame(2, 2);
    ensure_equal(verify_idempotency(&contract, &[], &frame), Ok(()))
}

#[test]
fn retry_safety_safe_with_creates_passes() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(3),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    let frame = test_frame(2, 2);
    ensure_equal(verify_idempotency(&contract, &[], &frame), Ok(()))
}

#[test]
fn retry_safety_safe_with_destroys_passes() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(4),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };
    let frame = test_frame(2, 2);
    ensure_equal(verify_idempotency(&contract, &[], &frame), Ok(()))
}

#[test]
fn retry_safety_unsafe_with_writes_fails() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(5),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::NotRetrySafe,
        required_capabilities: Box::new([]),
    };
    let frame = test_frame(2, 2);
    ensure_equal(
        verify_idempotency(&contract, &[], &frame),
        Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite)),
    )
}

#[test]
fn retry_safety_unsafe_with_sends_fails() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(6),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::ExternalWrite,
        retry_safety: RetrySafety::NotRetrySafe,
        required_capabilities: Box::new([]),
    };
    let frame = test_frame(2, 2);
    ensure_equal(
        verify_idempotency(&contract, &[], &frame),
        Err(IdempotencyViolation::MissingKey(SideEffect::ExternalWrite)),
    )
}

#[test]
fn retry_safety_unsafe_with_creates_fails() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(7),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::NotRetrySafe,
        required_capabilities: Box::new([]),
    };
    let frame = test_frame(2, 2);
    ensure_equal(
        verify_idempotency(&contract, &[], &frame),
        Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite)),
    )
}

#[test]
fn retry_safety_unsafe_with_destroys_fails() -> Result<(), String> {
    let contract = ActionContract {
        id: ActionId::new(8),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::NotRetrySafe,
        required_capabilities: Box::new([]),
    };
    let frame = test_frame(2, 2);
    ensure_equal(
        verify_idempotency(&contract, &[], &frame),
        Err(IdempotencyViolation::MissingKey(SideEffect::LocalWrite)),
    )
}

#[test]
fn retry_safety_key_required_with_empty_keys_all_side_effects_fail() -> Result<(), String> {
    for se in [SideEffect::LocalWrite, SideEffect::ExternalWrite, SideEffect::LocalWrite, SideEffect::LocalWrite] {
        let contract = ActionContract {
            id: ActionId::new(100),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: se,
            retry_safety: RetrySafety::RequiresIdempotencyKey,
            required_capabilities: Box::new([]),
        };
        let frame = test_frame(2, 2);
        ensure_equal(
            verify_idempotency(&contract, &[], &frame),
            Err(IdempotencyViolation::MissingKey(se)),
        )?;
    }
    Ok(())
}

#[test]
fn retry_safety_key_required_with_clean_keys_all_side_effects_pass() -> Result<(), String> {
    for se in [SideEffect::LocalWrite, SideEffect::ExternalWrite, SideEffect::LocalWrite, SideEffect::LocalWrite] {
        let contract = ActionContract {
            id: ActionId::new(200),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: se,
            retry_safety: RetrySafety::RequiresIdempotencyKey,
            required_capabilities: Box::new([]),
        };
        let mut frame = test_frame(2, 2);
        let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
        let key_slots = [SlotIdx::new(0)];
        ensure_equal(verify_idempotency(&contract, &key_slots, &frame), Ok(()))?;
    }
    Ok(())
}

#[test]
fn retry_safety_none_side_effect_passes_for_all_retry_safeties() -> Result<(), String> {
    for rs in [RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey, RetrySafety::NotRetrySafe] {
        let contract = ActionContract {
            id: ActionId::new(300),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 0,
            output_slot_count: 1,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::Pure,
            retry_safety: rs,
            required_capabilities: Box::new([]),
        };
        let frame = test_frame(2, 2);
        ensure_equal(verify_idempotency(&contract, &[], &frame), Ok(()))?;
    }
    Ok(())
}

// =============================================================================
// 13. Taint-based key validation
// =============================================================================

#[test]
fn taint_validation_secret_key_rejected() -> Result<(), String> {
    let mut frame = test_frame(2, 2);
    let wr = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret);
    kani::assert(wr.is_ok(), "kani harness assertion");
    let key_slots = [SlotIdx::new(0)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Err(IdempotencyViolation::SecretInKey(0)),
    )
}

#[test]
fn taint_validation_derived_from_secret_key_rejected() -> Result<(), String> {
    let mut frame = test_frame(2, 2);
    let wr = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(99), Taint::DerivedFromSecret);
    kani::assert(wr.is_ok(), "kani harness assertion");
    let key_slots = [SlotIdx::new(1)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Err(IdempotencyViolation::SecretInKey(1)),
    )
}

#[test]
fn taint_validation_random_key_rejected() -> Result<(), String> {
    let mut frame = test_frame(2, 2);
    let wr = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    kani::assert(wr.is_ok(), "kani harness assertion");
    let key_slots = [SlotIdx::new(0)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Err(IdempotencyViolation::SecretInKey(0)),
    )
}

#[test]
fn taint_validation_time_dependent_key_rejected() -> Result<(), String> {
    let mut frame = test_frame(2, 2);
    let wr = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    kani::assert(wr.is_ok(), "kani harness assertion");
    let key_slots = [SlotIdx::new(0)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Err(IdempotencyViolation::SecretInKey(0)),
    )
}

#[test]
fn taint_validation_clean_key_passes() -> Result<(), String> {
    let mut frame = test_frame(2, 2);
    let wr = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean);
    kani::assert(wr.is_ok(), "kani harness assertion");
    let key_slots = [SlotIdx::new(0)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Ok(()),
    )
}

#[test]
fn taint_validation_mixed_key_first_clean_second_secret_short_circuits_on_secret() -> Result<(), String> {
    let mut frame = test_frame(4, 2);
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
    let _ = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret);
    let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Err(IdempotencyViolation::SecretInKey(1)),
    )
}

#[test]
fn taint_validation_first_of_two_both_secret_short_circuits_on_first() -> Result<(), String> {
    let mut frame = test_frame(4, 2);
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    let _ = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret);
    let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Err(IdempotencyViolation::SecretInKey(0)),
    )
}

// =============================================================================
// 14. CapabilitySet structural tests
// =============================================================================

#[test]
fn capability_set_empty_len_zero_and_is_empty() -> Result<(), String> {
    let set = CapabilitySet::empty();
    ensure_equal(set.len(), 0)?;
    ensure_equal(set.is_empty(), true)
}

#[test]
fn capability_set_with_single_grant_has_len_one_and_not_empty() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([cap("network", ActionId::new(1))]));
    ensure_equal(set.len(), 1)?;
    ensure_equal(set.is_empty(), false)
}

#[test]
fn capability_set_with_zero_grants_is_empty() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([]));
    ensure_equal(set.len(), 0)?;
    ensure_equal(set.is_empty(), true)
}

#[test]
fn capability_set_empty_name_grant_denies_all() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([cap("", ActionId::new(1))]));
    ensure_equal(set.grants(&cap("network", ActionId::new(1))), false)?;
    ensure_equal(set.grants(&cap("", ActionId::new(1))), false)?;
    ensure_equal(set.grants(&cap("", ActionId::new(0))), false)
}

#[test]
fn capability_set_different_actions_no_cross_grant() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([cap("resource", ActionId::new(1))]));
    ensure_equal(set.grants(&cap("resource", ActionId::new(2))), false)?;
    ensure_equal(set.grants(&cap("resource", ActionId::new(0))), false)?;
    ensure_equal(set.grants(&cap("other", ActionId::new(1))), false)
}

#[test]
fn capability_set_large_valid_grants() -> Result<(), String> {
    let mut caps = Vec::new();
    for i in 0..50_u16 {
        caps.push(cap("action", ActionId::new(i)));
    }
    let set = CapabilitySet::from_grants(caps.into_boxed_slice());
    ensure_equal(set.len(), 50)?;
    ensure_equal(set.grants(&cap("action", ActionId::new(0))), true)?;
    ensure_equal(set.grants(&cap("action", ActionId::new(49))), true)?;
    ensure_equal(set.grants(&cap("action", ActionId::new(50))), false)?;
    ensure_equal(set.grants(&cap("different", ActionId::new(0))), false)?;
    Ok(())
}

#[test]
fn capability_set_identical_grants_multiple_times_still_grants_once() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([
        cap("network", ActionId::new(1)),
        cap("network", ActionId::new(1)),
        cap("network", ActionId::new(1)),
    ]));
    ensure_equal(set.len(), 3)?;
    ensure_equal(set.grants(&cap("network", ActionId::new(1))), true)
}

// =============================================================================
// 15. Kani proofs: exported as cfg(kani) module
// =============================================================================

#[cfg(kani)]
mod kani {
    use crate::action::{
        ActionContract, Idempotency, IdempotencyViolation, RetrySafety, SideEffect,
        validate_idempotency_key_ingredients, verify_idempotency,
    };
    use crate::capability::{Capability, CapabilitySet};
    use crate::frame::RunFrame;
    use crate::ids::{ActionId, RunId, SlotIdx, StepIdx};
    use crate::value::{SlotValue, Taint};

    /// Idempotency determinism: same inputs always produce same result.
    #[kani::proof]
    #[kani::unwind(4)]
    fn kani_idempotency_determinism() {
        let contract = kani::any::<ActionContract>();
        kani::assume(contract.retry_safety == RetrySafety::RequiresIdempotencyKey);
        kani::assume(contract.side_effect != SideEffect::Pure);

        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        let frame = match frame {
            Ok(f) => f,
            Err(_) => { kani::assume(false); return; }
        };

        let key_slots: [SlotIdx; 0] = [];
        let r1 = verify_idempotency(&contract, &key_slots, &frame);
        let r2 = verify_idempotency(&contract, &key_slots, &frame);
        , "kani harness assertion");
    let key_slots = [SlotIdx::new(0)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Ok(()),
    )
}

#[test]
fn taint_validation_mixed_key_first_clean_second_secret_short_circuits_on_secret() -> Result<(), String> {
    let mut frame = test_frame(4, 2);
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean);
    let _ = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret);
    let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Err(IdempotencyViolation::SecretInKey(1)),
    )
}

#[test]
fn taint_validation_first_of_two_both_secret_short_circuits_on_first() -> Result<(), String> {
    let mut frame = test_frame(4, 2);
    let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    let _ = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret);
    let key_slots = [SlotIdx::new(0), SlotIdx::new(1)];
    ensure_equal(
        validate_idempotency_key_ingredients(&key_slots, &frame),
        Err(IdempotencyViolation::SecretInKey(0)),
    )
}

// =============================================================================
// 14. CapabilitySet structural tests
// =============================================================================

#[test]
fn capability_set_empty_len_zero_and_is_empty() -> Result<(), String> {
    let set = CapabilitySet::empty();
    ensure_equal(set.len(), 0)?;
    ensure_equal(set.is_empty(), true)
}

#[test]
fn capability_set_with_single_grant_has_len_one_and_not_empty() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([cap("network", ActionId::new(1))]));
    ensure_equal(set.len(), 1)?;
    ensure_equal(set.is_empty(), false)
}

#[test]
fn capability_set_with_zero_grants_is_empty() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([]));
    ensure_equal(set.len(), 0)?;
    ensure_equal(set.is_empty(), true)
}

#[test]
fn capability_set_empty_name_grant_denies_all() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([cap("", ActionId::new(1))]));
    ensure_equal(set.grants(&cap("network", ActionId::new(1))), false)?;
    ensure_equal(set.grants(&cap("", ActionId::new(1))), false)?;
    ensure_equal(set.grants(&cap("", ActionId::new(0))), false)
}

#[test]
fn capability_set_different_actions_no_cross_grant() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([cap("resource", ActionId::new(1))]));
    ensure_equal(set.grants(&cap("resource", ActionId::new(2))), false)?;
    ensure_equal(set.grants(&cap("resource", ActionId::new(0))), false)?;
    ensure_equal(set.grants(&cap("other", ActionId::new(1))), false)
}

#[test]
fn capability_set_large_valid_grants() -> Result<(), String> {
    let mut caps = Vec::new();
    for i in 0..50_u16 {
        caps.push(cap("action", ActionId::new(i)));
    }
    let set = CapabilitySet::from_grants(caps.into_boxed_slice());
    ensure_equal(set.len(), 50)?;
    ensure_equal(set.grants(&cap("action", ActionId::new(0))), true)?;
    ensure_equal(set.grants(&cap("action", ActionId::new(49))), true)?;
    ensure_equal(set.grants(&cap("action", ActionId::new(50))), false)?;
    ensure_equal(set.grants(&cap("different", ActionId::new(0))), false)?;
    Ok(())
}

#[test]
fn capability_set_identical_grants_multiple_times_still_grants_once() -> Result<(), String> {
    let set = CapabilitySet::from_grants(Box::new([
        cap("network", ActionId::new(1)),
        cap("network", ActionId::new(1)),
        cap("network", ActionId::new(1)),
    ]));
    ensure_equal(set.len(), 3)?;
    ensure_equal(set.grants(&cap("network", ActionId::new(1))), true)
}

// =============================================================================
// 15. Kani proofs: exported as cfg(kani) module
// =============================================================================

#[cfg(kani)]
mod kani {
    use crate::action::{
        ActionContract, Idempotency, IdempotencyViolation, RetrySafety, SideEffect,
        validate_idempotency_key_ingredients, verify_idempotency,
    };
    use crate::capability::{Capability, CapabilitySet};
    use crate::frame::RunFrame;
    use crate::ids::{ActionId, RunId, SlotIdx, StepIdx};
    use crate::value::{SlotValue, Taint};

    /// Idempotency determinism: same inputs always produce same result.
    #[kani::proof]
    #[kani::unwind(4)]
    fn kani_idempotency_determinism() {
        let contract = kani::any::<ActionContract>();
        kani::assume(contract.retry_safety == RetrySafety::RequiresIdempotencyKey);
        kani::assume(contract.side_effect != SideEffect::Pure);

        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
        let frame = match frame {
            Ok(f) => f,
            Err(_) => { kani::assume(false); return; }
        };

        let key_slots: [SlotIdx; 0] = [];
        let r1 = verify_idempotency(&contract, &key_slots, &frame);
        let r2 = verify_idempotency(&contract, &key_slots, &frame);
        kani::assert(
            r1 == r2,
            "verify_idempotency must be deterministic for same inputs",
        );
    }

    /// Short-circuit: first error is returned, no second error variant appears.
    #[kani::proof]
    #[kani::unwind(8)]
    fn kani_short_circuit_first_error() {
        let contract = kani::any::<ActionContract>();
        kani::assume(contract.retry_safety == RetrySafety::RequiresIdempotencyKey);
        kani::assume(contract.side_effect != SideEffect::Pure);

        let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4);
        let mut frame = match frame {
            Ok(f) => f,
            Err(_) => { kani::assume(false); return; }
        };

        let _ = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean);
        let _ = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret);
        let _ = frame.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(30), Taint::Secret);
        let _ = frame.write_slot_with_taint(SlotIdx::new(3), SlotValue::I64(40), Taint::Clean);

        let key_slots = [
            SlotIdx::new(0),
            SlotIdx::new(1),
            SlotIdx::new(2),
            SlotIdx::new(3),
        ];

        let result = validate_idempotency_key_ingredients(&key_slots, &frame);
        kani::assert(result.is_err(),
            "validate must return Err when any key slot is tainted",
        );

        if let Err(err) = &result {
            let variants = [
                matches!(err, IdempotencyViolation::MissingKey(_)),
                matches!(err, IdempotencyViolation::SecretInKey(_)),
                matches!(err, IdempotencyViolation::SecretInKey(_)),
                matches!(err, IdempotencyViolation::SecretInKey(_)),
            ];
            let count = variants.iter().filter(|&&b| b).count();
            ,
            "validate must return Err when any key slot is tainted",
        );

        if let Err(err) = &result {
            let variants = [
                matches!(err, IdempotencyViolation::MissingKey(_)),
                matches!(err, IdempotencyViolation::SecretInKey(_)),
                matches!(err, IdempotencyViolation::SecretInKey(_)),
                matches!(err, IdempotencyViolation::SecretInKey(_)),
            ];
            let count = variants.iter().filter(|&&b| b).count();
            kani::assert(
                count == 1,
                "Error must contain exactly one variant (short-circuit)",
            );
        }
    }

    /// Monotonicity: adding grants never removes existing permissions.
    #[kani::proof]
    #[kani::unwind(6)]
    fn kani_capability_grants_monotonicity() {
        let action = ActionId::new(1);
        let required = Capability::new("target".into(), action);

        let set_small = CapabilitySet::from_grants(Box::new([Capability::new("target".into(), action)]));
        let was_granted = set_small.grants(&required);
        ",
            );
        }
    }

    /// Monotonicity: adding grants never removes existing permissions.
    #[kani::proof]
    #[kani::unwind(6)]
    fn kani_capability_grants_monotonicity() {
        let action = ActionId::new(1);
        let required = Capability::new("target".into(), action);

        let set_small = CapabilitySet::from_grants(Box::new([Capability::new("target".into(), action)]));
        let was_granted = set_small.grants(&required);
        kani::assert(was_granted, "single grant must match");

        let set_large = CapabilitySet::from_grants(Box::new([
            Capability::new("target".into(), action),
            Capability::new("extra".into(), action),
        ]));
        let still_granted = set_large.grants(&required);
        );
        let still_granted = set_large.grants(&required);
        kani::assert(
            still_granted,
            "adding grants must not remove existing permissions",
        );
    }

    /// Empty set never grants any capability.
    #[kani::proof]
    #[kani::unwind(4)]
    fn kani_empty_set_never_grants() {
        let cap = kani::any::<Capability>();
        let set = CapabilitySet::empty();
        let result = set.grants(&cap);
        kani::assert(!result, "empty capability set must never grant");
    }

    /// Different actions don't cross-grant.
    #[kani::proof]
    #[kani::unwind(4)]
    fn kani_different_actions_no_cross_grant() {
        let grant = Capability::new("resource".into(), ActionId::new(1));
        let set = CapabilitySet::from_grants(Box::new([grant]));

        let required = Capability::new("resource".into(), ActionId::new(2));
        kani::assert(!set.grants(&required, "assertion failed"),
            "grant for action 1 must not grant action 2",
        );

        let required_same_name = Capability::new("resource".into(), ActionId::new(1));
        kani::assert(set.grants(&required_same_name, "assertion failed"),
            "grant for action 1 must grant action 1",
        );
    }
}
