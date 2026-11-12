//! Kani harness: verify Err(MissingKey) when idempotency==IdempotentExternal
//! and key_slots is empty (KeyRequired).
//!
//! Obligation: KANI-RUNTIME-002
//! Requirement: POST-006

#![forbid(unsafe_code)]

use vb_core::action::{verify_idempotency, ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};

/// KANI-RUNTIME-002: Err(MissingKey) when key_slots is empty with KeyRequired.
///
/// Bounded: key_slots length = 0.
#[kani::proof]
#[kani::unwind(4)]
fn verify_idempotency_missing_key() {
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

    // Empty key_slots
    let key_slots: [SlotIdx; 0] = [];

    // RunFrame is still needed (though not accessed when key_slots is empty)
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    kani::assume(frame.is_ok());
    let frame = frame.ok().unwrap();

    let result = verify_idempotency(&contract, &key_slots, &frame);
    kani::assert(result.is_err(), "verify_idempotency must Err when key_slots is empty with KeyRequired");

    if let Err(err) = &result {
        match err {
            vb_core::action::IdempotencyViolation::MissingKey(_) => {
                // Expected variant
            }
            _ => {
                kani::assert(false, "Expected MissingKey error variant");
            }
        }
    }
}
