//! Kani harnesses for vb_core action module.
//!
//! Verifier lane: kani
//! Obligations: OBL-004, OBL-005, OBL-006, OBL-013
//!
//! Tests panic-freedom, serialization bounds.
//! All harnesses use kani::any() for structural inputs — no hardcoded data.

#![allow(unused)]

// ─── OBL-004: Panic freedom for compute_action_idempotency_key ──────────────────

/// compute_action_idempotency_key uses only wrapping arithmetic.
/// This harness proves it never panics for any input.
#[kani::proof]
fn check_compute_action_idempotency_key_panic_free() {
    // Use kani::any() for all inputs — no hardcoded values.
    let run: u64 = kani::any();
    let seq: u64 = kani::any();
    let action: u64 = kani::any();

    let key = crate::action::compute_action_idempotency_key(
        crate::ids::RunId::new(run),
        crate::ids::SeqNo::new(seq),
        crate::ids::ActionId::new(action),
    );

    // Verify the key is always a valid u128 (no overflow panics).
    // The wrapping arithmetic guarantees the result is in [0, u128::MAX].
    //! Kani harnesses for vb_core action module.
//!
//! Verifier lane: kani
//! Obligations: OBL-004, OBL-005, OBL-006, OBL-013
//!
//! Tests panic-freedom, serialization bounds.
//! All harnesses use kani::any() for structural inputs — no hardcoded data.

#![allow(unused)]

// ─── OBL-004: Panic freedom for compute_action_idempotency_key ──────────────────

/// compute_action_idempotency_key uses only wrapping arithmetic.
/// This harness proves it never panics for any input.
#[kani::proof]
fn check_compute_action_idempotency_key_panic_free() {
    // Use kani::any() for all inputs — no hardcoded values.
    let run: u64 = kani::any();
    let seq: u64 = kani::any();
    let action: u64 = kani::any();

    let key = crate::action::compute_action_idempotency_key(
        crate::ids::RunId::new(run),
        crate::ids::SeqNo::new(seq),
        crate::ids::ActionId::new(action),
    );

    // Verify the key is always a valid u128 (no overflow panics).
    // The wrapping arithmetic guarantees the result is in [0, u128::MAX].
    kani::assert(key >= 0, "Idempotency key must be non-negative");
    kani::assert(key <= u128::MAX, "Idempotency key must fit in u128");
}

// ─── OBL-005: Panic freedom for action_ticket_has_valid_key ─────────────────────

/// action_ticket_has_valid_key compares the stored key against the computed key.
/// This harness proves it never panics for any ticket with arbitrary field values.
#[kani::proof]
fn check_action_ticket_has_valid_key_panic_free() {
    // Generate all ticket fields using kani::any() — no hardcoded data.
    let run: u64 = kani::any();
    let step: u64 = kani::any();
    let seq: u64 = kani::any();
    let action: u64 = kani::any();
    let attempt: u16 = kani::any();
    let idempotency_key: u128 = kani::any();
    let capacity: u16 = kani::any();

    let action_ticket = crate::action::ActionTicket {
        run: crate::ids::RunId::new(run),
        step: crate::ids::StepIdx::new(step),
        seq: crate::ids::SeqNo::new(seq),
        action: crate::ids::ActionId::new(action),
        attempt,
        idempotency_key,
        capacity,
    };

    // This must not panic — the function only does equality comparison.
    let _has_valid = crate::action::action_ticket_has_valid_key(action_ticket);

    // Optional: verify the result is a valid boolean.
    assert(_has_valid == true || _has_valid == false, "Result must be boolean");
}

// ─── OBL-006: Panic freedom for is_retry_safe_with_key ──────────────────────────

/// is_retry_safe_with_key is a const fn with exhaustive match.
/// This harness proves it never panics for any retry safety enum value.
#[kani::proof]
fn check_is_retry_safe_with_key_panic_free() {
    // Generate a raw u8 and assume it's a valid RetrySafety discriminant.
    let safety: u8 = kani::any();
    let key_present: bool = kani::any();

    kani::assume(safety <= 3); // RetrySafety has variants 0-3

    let safety_enum = match safety {
        0 => crate::action::RetrySafety::Idempotent,
        1 => crate::action::RetrySafety::RequiresIdempotencyKey,
        2 => crate::action::RetrySafety::NotRetrySafe,
        3 => crate::action::RetrySafety::Unknown,
        _ => {
            kani::assume(false);
            loop {}
        }
    };

    // This must not panic — the function only does pattern matching.
    let result = crate::action::is_retry_safe_with_key(safety_enum, key_present);

    // Verify the result is a valid boolean.
    kani::assert(result == true || result == false, "Result must be boolean");
}

// ─── OBL-013: Serialization round-trip bounds (7 fields, pre-MockMarker) ────────

/// ActionTicket serializes correctly in postcard wire format.
/// Size is bounded by the 7 existing fields.
#[kani::proof]
fn check_action_ticket_serialization_size() {
    // Generate all fields using kani::any() — no hardcoded values.
    let run: u64 = kani::any();
    let step: u64 = kani::any();
    let seq: u64 = kani::any();
    let action: u64 = kani::any();
    let attempt: u16 = kani::any();
    let idempotency_key: u128 = kani::any();
    let capacity: u16 = kani::any();

    let action_ticket = crate::action::ActionTicket {
        run: crate::ids::RunId::new(run),
        step: crate::ids::StepIdx::new(step),
        seq: crate::ids::SeqNo::new(seq),
        action: crate::ids::ActionId::new(action),
        attempt,
        idempotency_key,
        capacity,
    };

    // Serialize to postcard bytes.
    let serialized = match postcard::to_allocvec(&action_ticket) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    // Lower bound: at least 1 byte for each field's varint encoding.
    // Minimum: 7 fields * 1 byte (minimum varint) = 7 bytes.
    // Upper bound: worst case for 7 fields.
    //   u64 fields (4): max 9 bytes each = 36
    //   u16 fields (2): max 3 bytes each = 6
    //   u128 (1): max 17 bytes
    //   Total: 59 bytes (with postcard overhead)
    kani::assert(serialized.len() >= 7, "Serialized ticket must contain at least 7 bytes (7 fields minimum)");
    kani::assert(serialized.len() <= 64, "Serialized ticket must fit within 64 bytes (7 fields with max varint overhead)");
}

// ─── OBL-005 extended: Valid key check — ticket with computed key is valid ──────

/// A ticket created with a key from compute_action_idempotency_key
/// always passes action_ticket_has_valid_key.
#[kani::proof]
fn check_action_ticket_has_valid_key_with_computed_key() {
    let run: u64 = kani::any();
    let seq: u64 = kani::any();
    let action: u64 = kani::any();
    let step: u64 = kani::any();
    let attempt: u16 = kani::any();
    let capacity: u16 = kani::any();

    let run_id = crate::ids::RunId::new(run);
    let seq_id = crate::ids::SeqNo::new(seq);
    let action_id = crate::ids::ActionId::new(action);

    // Compute the canonical key from run, seq, action.
    let computed_key = crate::action::compute_action_idempotency_key(run_id, seq_id, action_id);

    // Create a ticket with the computed key.
    let ticket = crate::action::ActionTicket {
        run: run_id,
        step: crate::ids::StepIdx::new(step),
        seq: seq_id,
        action: action_id,
        attempt,
        idempotency_key: computed_key,
        capacity,
    };

    // The validation check must return true.
    kani::assert(crate::action::action_ticket_has_valid_key(ticket), "Ticket with computed key must pass validation");
}
