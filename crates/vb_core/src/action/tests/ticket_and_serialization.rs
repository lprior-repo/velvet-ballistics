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
        run_attempt: 1,
        attempt: 3,
        idempotency_key: 0xDEADBEEF_u128,
        capacity: 10,
        fencing_token: 0,
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
        side_effect: SideEffect::None,
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
        side_effect: SideEffect::Writes,
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
    assert_eq!(contract.side_effect, SideEffect::Writes);
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
        side_effect: SideEffect::None,
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

        let key1 = compute_action_idempotency_key(run, 1, seq, action);
        let key2 = compute_action_idempotency_key(run, 1, seq, action);

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

        let canonical_key = compute_action_idempotency_key(run, 1, seq, action);

        // Canonical ticket should validate
        let canonical_ticket = ActionTicket {
            run,
            step: StepIdx::new(deterministic_rand_u16_bounded(4, iter, 13)),
            seq,
            action,
            run_attempt: 1,
            attempt: deterministic_rand_u16_bounded(3, iter, 14) + 1,
            idempotency_key: canonical_key,
            capacity: deterministic_rand_u16_bounded(5, iter, 15) + 1,
            fencing_token: 0,
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
// ActionTicket postcard roundtrip — vb-xsu4g regression.
// Pins the encoding contract that the `idempotency_key` field (and every
// other `ActionTicket` field) survives postcard serialize/deserialize.
// The P0 durable-action-events defect lost this field when the legacy
// `ActionCompletedEvent` variant was used in place of the full-ticket
// `ActionCompletedEnvelope`; this roundtrip test fails the build if a
// future change drops the field from the wire format.
// =========================================================================

#[test]
fn action_ticket_postcard_roundtrip_preserves_idempotency_key() {
    // Compute the canonical idempotency key for the chosen
    // (run, seq, action) so the test exercises a real, valid ticket
    // that recovery would actually accept on replay.
    let run = RunId::new(0x0123_4567_89AB_CDEF_u64);
    let step = StepIdx::new(0x00C3);
    let seq = SeqNo::new(0x0042_1337_9000_0001_u64);
    let action = ActionId::new(0x07B7);
    let idempotency_key = compute_action_idempotency_key(run, 1, seq, action);
    let original = ActionTicket {
        run,
        step,
        seq,
        action,
        run_attempt: 1,
        attempt: 2,
        idempotency_key,
        capacity: 5,
        fencing_token: 0,
    };

    let bytes = postcard::to_allocvec(&original).expect("postcard encode must succeed");
    let decoded: ActionTicket = postcard::from_bytes(&bytes).expect("postcard decode must succeed");

    // Whole-struct equality: catches any field-level drop, rename, or
    // hidden reordering. ActionTicket derives Eq + PartialEq so this is
    // a true byte-for-byte field comparison.
    assert_eq!(
        decoded, original,
        "ActionTicket postcard roundtrip must preserve every field"
    );

    // Defensive: explicitly pin `idempotency_key` so a future regression
    // that survives PartialEq by chance (e.g. swap with another u128
    // field) is still caught here with a clear diagnostic.
    assert_eq!(
        decoded.idempotency_key, original.idempotency_key,
        "ActionTicket.idempotency_key must survive postcard roundtrip"
    );

    // The canonical-key invariant must still hold on the decoded ticket.
    assert!(
        action_ticket_has_valid_key(decoded),
        "decoded ticket must still satisfy the canonical-key invariant"
    );
}

#[test]
fn action_ticket_postcard_roundtrip_at_idempotency_key_boundaries() {
    // Boundary coverage for the `idempotency_key` u128 wire value:
    // 0 (minimum), 1 (smallest non-zero), u128::MAX (largest).
    for &key in &[
        0u128,
        1u128,
        0xDEAD_BEEF_CAFE_F00D_1234_5678_9ABC_DEF0_u128,
        u128::MAX,
    ] {
        let original = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(1),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: key,
            capacity: 1,
            fencing_token: 0,
        };
        let bytes = postcard::to_allocvec(&original).expect("postcard encode must succeed");
        let decoded: ActionTicket =
            postcard::from_bytes(&bytes).expect("postcard decode must succeed");
        assert_eq!(
            decoded.idempotency_key, key,
            "idempotency_key boundary value 0x{key:032X} must survive roundtrip"
        );
        assert_eq!(
            decoded, original,
            "all fields must survive boundary roundtrip"
        );
    }
}
