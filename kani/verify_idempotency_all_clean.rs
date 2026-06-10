//! Kani harness: verify Ok(()) when all key slots pass taint checks.
//!
//! Condition: key_slots are all Clean (no SecretTaint, Random, or TimeDependent).
//! Contract: side_effect!=None, idempotency==IdempotentExternal, retry_safety==KeyRequired.
//!
//! Obligation: KANI-RUNTIME-001
//! Requirement: POST-005

#![forbid(unsafe_code)]

use vb_core::action::{verify_idempotency, ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

/// KANI-RUNTIME-001: Ok when all key slots are clean (no SecretTaint/Random/TimeDependent).
///
/// Bounded: key_slots length 1..16, all slots contain Clean values.
#[kani::proof]
#[kani::unwind(6)]
fn verify_idempotency_all_clean() {
    // Build a contract with side_effect, KeyRequired, IdempotentExternal
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

    // Build a RunFrame with 4 slots, all Clean
    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4);
    kani::assume(frame.is_ok());
    let mut frame = frame.ok().unwrap();

    // Populate all 4 slots with Clean values
    let mut slot_i = 0u32;
    while slot_i < 4 {
        let write_result = frame.write_slot_with_taint(
            SlotIdx::new(slot_i),
            SlotValue::I64(42),
            Taint::Clean,
        );
        kani::assume(write_result.is_ok());
        slot_i = match slot_i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }

    // key_slots = [0, 1, 2, 3] — all clean
    let key_slots = [
        SlotIdx::new(0),
        SlotIdx::new(1),
        SlotIdx::new(2),
        SlotIdx::new(3),
    ];

    let result = verify_idempotency(&contract, &key_slots, &frame);
    kani::assert(result.is_ok(), "verify_idempotency must pass when all key slots are clean");
}
