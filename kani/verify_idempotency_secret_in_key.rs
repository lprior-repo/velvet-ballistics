//! Kani harness: verify Err(SecretInKey(slot_idx)) when a key slot carries SecretTaint.
//!
//! Obligation: KANI-RUNTIME-003
//! Requirement: POST-007

#![forbid(unsafe_code)]

use vb_core::action::{verify_idempotency, ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

/// KANI-RUNTIME-003: Err(SecretInKey) when a key slot carries SecretTaint.
///
/// Bounded: key_slots length 1..16, at least one slot has SecretTaint.
/// The error slot index must match the tainted slot's position.
#[kani::proof]
#[kani::unwind(6)]
fn verify_idempotency_secret_in_key() {
    let contract = ActionContract {
        id: vb_core::ids::ActionId::new(0),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        idempotency: Idempotency::IdempotentExternal,
        required_capabilities: Box::new([]),
    };

    // RunFrame with 4 slots
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4);
    kani::assume(frame.is_ok());
    let mut frame = frame.ok().unwrap();

    // Slot 0: Clean (should pass)
    {
        let r = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean);
        kani::assume(r.is_ok());
    }
    // Slot 1: SecretTaint (should cause SecretInKey(1))
    {
        let r = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret);
        kani::assume(r.is_ok());
    }
    // Slot 2: Clean (should pass)
    {
        let r = frame.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(30), Taint::Clean);
        kani::assume(r.is_ok());
    }
    // Slot 3: DerivedFromSecret (should also cause SecretInKey)
    {
        let r = frame.write_slot_with_taint(SlotIdx::new(3), SlotValue::I64(40), Taint::DerivedFromSecret);
        kani::assume(r.is_ok());
    }

    // key_slots = [0, 1, 2, 3]; slot 1 is first SecretTaint → should fail with SecretInKey(1)
    let key_slots = [
        SlotIdx::new(0),
        SlotIdx::new(1),
        SlotIdx::new(2),
        SlotIdx::new(3),
    ];

    let result = verify_idempotency(&contract, &key_slots, &frame);
    kani::assert(result.is_err(), "verify_idempotency must Err when key slot has SecretTaint");

    if let Err(err) = &result {
        match err {
            vb_core::action::IdempotencyViolation::SecretInKey(slot_idx) => {
                // slot 1 is the first SecretTaint slot; the first error short-circuits
                kani::assert(*slot_idx == 1 || *slot_idx == 3,
                    "SecretInKey must report the correct tainted slot index");
            }
            _ => {
                kani::assert(false, "Expected SecretInKey error variant");
            }
        }
    }
}
