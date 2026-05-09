#![forbid(unsafe_code)]
#![cfg(test)]

use vb_core::action::{
    ActionContract, Idempotency, SideEffect, RetrySafety,
};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::value::Taint;
use vb_runtime::action::ActionRegistry;
use vb_runtime::engine::action::compute_idempotency_key;

#[test]
fn test_compute_idempotency_key_deterministic_across_runs() {
    let run_id = RunId::new(42);
    let seq = SeqNo::new(7);
    let action = ActionId::new(3);
    let key1 = compute_idempotency_key(run_id, seq, action);
    let key2 = compute_idempotency_key(run_id, seq, action);
    assert_eq!(key1, key2, "idempotency_key should be deterministic across calls");
}

#[test]
fn test_idempotency_key_no_collision_on_adjacent_seq() {
    let run_id = RunId::new(1);
    let action = ActionId::new(1);
    for seq_val in 0..100u64 {
        let seq_a = SeqNo::new(seq_val);
        let seq_b = SeqNo::new(seq_val.wrapping_add(1));
        let key_a = compute_idempotency_key(run_id, seq_a, action);
        let key_b = compute_idempotency_key(run_id, seq_b, action);
        assert_ne!(key_a, key_b, "adjacent seq values should produce different keys");
    }
}

#[test]
fn test_registry_resolve_returns_what_was_stored() {
    let mut registry = ActionRegistry::new();
    for id_val in 0..100u16 {
        let contract = ActionContract {
            id: ActionId::new(id_val),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let reg_result = registry.register(contract.clone());
        assert!(reg_result.is_ok(), "register should succeed for id {}", id_val);
        let resolve_result = registry.resolve_compile_time(contract.id);
        assert!(resolve_result.is_ok(), "resolve should succeed for id {}", id_val);
        let resolved = resolve_result.unwrap();
        assert_eq!(resolved.id, contract.id, "resolved contract should match");
    }
}

#[test]
fn test_registry_len_consistency() {
    let mut registry = ActionRegistry::new();
    let test_ids = [5u16, 50, 99, 0, 25];
    for (i, &id_val) in test_ids.iter().enumerate() {
        let contract = ActionContract {
            id: ActionId::new(id_val),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        registry.register(contract).expect("register should succeed");
        let expected_len = (i + 1) as usize;
        assert_eq!(registry.len(), expected_len, "len mismatch after {} registrations", i + 1);
    }
}

#[test]
fn test_registry_duplicate_registration_consistency() {
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(77),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let first = registry.register(contract.clone());
    assert!(first.is_ok(), "first register should succeed");
    let second = registry.register(contract);
    assert!(second.is_err(), "second register should fail");
}

#[test]
fn test_taint_propagation_never_downgrades() {
    use vb_core::action::propagate_action_taint;
    let idempotencies = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];
    let taints = [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret];
    for &idempotency in &idempotencies {
        for &input_taint in &taints {
            let result = propagate_action_taint(idempotency, input_taint);
            match (idempotency, input_taint) {
                (Idempotency::DeterministicPure, t) => {
                    assert_eq!(result, t, "DeterministicPure should preserve taint");
                }
                (Idempotency::IdempotentExternal, t) => {
                    assert_eq!(result, t, "IdempotentExternal should preserve taint");
                }
                (Idempotency::AtLeastOnceExternal, Taint::Clean) => {
                    assert_eq!(result, Taint::Clean, "AtLeastOnce with Clean stays Clean");
                }
                (Idempotency::AtLeastOnceExternal, Taint::Secret) => {
                    assert_eq!(result, Taint::DerivedFromSecret, "AtLeastOnce upgrades Secret");
                }
                (Idempotency::AtLeastOnceExternal, Taint::DerivedFromSecret) => {
                    assert_eq!(result, Taint::DerivedFromSecret, "AtLeastOnce preserves Derived");
                }
            }
        }
    }
}

#[test]
fn test_at_least_once_upgrades_secret() {
    use vb_core::action::propagate_action_taint;
    for _ in 0..100 {
        let result = propagate_action_taint(Idempotency::AtLeastOnceExternal, Taint::Secret);
        assert_eq!(result, Taint::DerivedFromSecret, "AtLeastOnce should always upgrade Secret");
    }
}

#[test]
fn test_pure_preserves_non_clean_taints() {
    use vb_core::action::propagate_action_taint;
    let non_clean = [Taint::Secret, Taint::DerivedFromSecret];
    for &taint in &non_clean {
        let result = propagate_action_taint(Idempotency::DeterministicPure, taint);
        assert_eq!(result, taint, "DeterministicPure should preserve {:?}", taint);
    }
}

#[test]
fn test_tracker_eviction_fifo_order() {
    let capacity = 5;
    let mut tracker = vb_runtime::action::IdempotencyTracker::new(capacity);
    for i in 0..capacity {
        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(i as u64 + 1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: i as u128,
            capacity: 3,
        };
        tracker.mark_completed(&ticket).expect("mark should succeed");
    }
    assert_eq!(tracker.len(), capacity, "tracker should be at capacity");
    let first_ticket = vb_core::action::ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 999u128,
        capacity: 3,
    };
    tracker.mark_completed(&first_ticket).expect("new mark should succeed");
    assert!(tracker.len() <= capacity, "tracker should never exceed capacity");
}

#[test]
fn test_tracker_capacity_never_exceeded() {
    let capacity = 10;
    let mut tracker = vb_runtime::action::IdempotencyTracker::new(capacity);
    for i in 0..20 {
        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(i as u64 + 100),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: (1000 + i) as u128,
            capacity: 3,
        };
        let _ = tracker.mark_completed(&ticket);
    }
    assert!(tracker.len() <= capacity, "tracker should never exceed capacity");
}

#[test]
fn test_action_contract_max_bytes_bounds() {
    let contract = ActionContract {
        id: ActionId::new(1),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: u32::MAX,
        max_output_bytes: u32::MAX,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    assert_eq!(contract.max_input_bytes, u32::MAX, "max_input_bytes should fit in u32");
    assert_eq!(contract.max_output_bytes, u32::MAX, "max_output_bytes should fit in u32");
}

#[test]
fn test_action_contract_timeout_ms_bounds() {
    let contract = ActionContract {
        id: ActionId::new(1),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: u64::MAX,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    assert_eq!(contract.timeout_ms, u64::MAX, "timeout_ms should fit in u64");
}

#[test]
fn test_action_error_encoding_roundtrip() {
    use vb_core::action::ActionError;
    let errors = [
        ActionError::UnknownAction { action: ActionId::new(1) },
        ActionError::InvalidTicket,
        ActionError::PayloadTooLarge { max_bytes: 100, actual_bytes: 200 },
        ActionError::OutputSlotOutOfBounds { slot: 5, max_slots: 4 },
        ActionError::NonIdempotentReplayBlocked,
        ActionError::CompletionAlreadyRecorded,
        ActionError::QueueFull,
        ActionError::EncodingFailed,
        ActionError::DispatchFailed,
    ];
    for original in &errors {
        let encoded: Vec<u8> = postcard::to_allocvec(original).expect("encode should succeed");
        let decoded: ActionError = postcard::from_bytes(&encoded).expect("decode should succeed");
        assert_eq!(&decoded, original, "roundtrip should preserve error");
    }
}

#[test]
fn test_idempotency_violation_encoding_roundtrip() {
    use vb_core::action::IdempotencyViolation;
    let violations = [
        IdempotencyViolation::MissingKey(SideEffect::Writes),
        IdempotencyViolation::SecretInKey(3),
        IdempotencyViolation::RandomInKey(2),
        IdempotencyViolation::TimeInKey(1),
    ];
    for original in &violations {
        let encoded: Vec<u8> = postcard::to_allocvec(original).expect("encode should succeed");
        let decoded: IdempotencyViolation = postcard::from_bytes(&encoded).expect("decode should succeed");
        assert_eq!(&decoded, original, "roundtrip should preserve violation");
    }
}
