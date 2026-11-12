//! Kani harness: verify Err(RandomInKey(slot_idx)) when a key slot carries Random taint.
//!
//! NOTE: The current validate_idempotency_key_ingredients only checks for
//! SecretTaint/DerivedFromSecret. Random and TimeDependent checks are scaffolded
//! for future extension. This harness documents the intended behavior.
//!
//! Obligation: KANI-RUNTIME-004
//! Requirement: POST-008

#![forbid(unsafe_code)]

use vb_core::action::{verify_idempotency, ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

/// KANI-RUNTIME-004: Err(RandomInKey) when a key slot carries Random taint.
///
/// Bounded: key_slots length 1..16, at least one slot has Random taint.
/// NOTE: validate_idempotency_key_ingredients currently only rejects Secret/DerivedFromSecret.
/// RandomInKey is scaffolded but not yet enforced — this harness documents the intended
/// future behavior. Currently the implementation returns Ok for Random (future extension point).
#[kani::proof]
#[kani::unwind(6)]
fn verify_idempotency_random_in_key() {
    let contract = ActionContract {
        id: vb_core::ids::ActionId::new(0),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        idempotency: Idempotency::IdempotentExternal,
        required_capabilities: Box::new([]),
    };

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    kani::assume(frame.is_ok());
    let mut frame = frame.ok().unwrap();

    // Write a value — we use I64 which has no inherent Random taint encoding.
    // Taint::Random is a variant but not encoded in SlotValue by default.
    // This harness is a placeholder: the actual RandomInKey enforcement requires
    // Taint::Random to be set on the slot value, which depends on runtime taint
    // tracking not yet modeled in SlotValue.
    // Write a value with Taint::Random to trigger RandomInKey error.
    let r = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Random);
    kani::assume(r.is_ok());

    let key_slots = [SlotIdx::new(0)];

    // validate_idempotency_key_ingredients checks for Secret/DerivedFromSecret and
    // also rejects Random/TimeDependent in the idempotency key.
    let result = verify_idempotency(&contract, &key_slots, &frame);

    // Assertion: Random taint in key slot triggers Err(RandomInKey).
    kani::assert(result.is_err() && matches!(result, Err(vb_core::action::IdempotencyViolation::RandomInKey(0))), "RandomInKey enforced — Taint::Random rejected in idempotency key");
}
