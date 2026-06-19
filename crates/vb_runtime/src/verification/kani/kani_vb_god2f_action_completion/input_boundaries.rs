#![forbid(unsafe_code)]

use super::model::{generated_action_input, generated_contract};
use vb_core::action::{ActionError, ActionOutcome};
use vb_core::ids::StepIdx;

#[kani::proof]
#[kani::unwind(8)]
fn vb_god2f_validate_input_bytes_contract_boundaries() {
    let input_slot_count: u16 = kani::any();
    let max_input_bytes: u32 = kani::any();
    let contract = match generated_contract(input_slot_count, max_input_bytes) {
        Some(value) => value,
        None => {
            kani::assert(false, "HVR-PO-RUNTIME-001 static action name must be valid");
            return;
        }
    };
    let input = generated_action_input(contract.id, StepIdx::ZERO);
    let result = crate::action::dispatch_generic(&input, &contract);
    let should_reject = max_input_bytes == 0 && input_slot_count > 0;

    kani::cover!(
        should_reject,
        "HVR-PO-RUNTIME-001 covers zero-byte rejection"
    );
    kani::cover!(
        !should_reject,
        "HVR-PO-RUNTIME-001 covers accepted metadata"
    );

    match (should_reject, result) {
        (
            true,
            Err(ActionError::PayloadTooLarge {
                max_bytes,
                actual_bytes,
            }),
        ) => {
            kani::assert(max_bytes == 0, "PayloadTooLarge max is exact");
            kani::assert(
                actual_bytes == 0,
                "PayloadTooLarge actual is structural zero",
            );
        }
        (true, _) => {
            kani::assert(
                false,
                "positive input slots with zero max bytes reject exactly",
            );
        }
        (false, Ok(ActionOutcome::Suspended(ticket))) => {
            kani::assert(ticket.run == input.run, "dispatch preserves run id");
            kani::assert(ticket.step == input.step, "dispatch preserves step id");
            kani::assert(
                ticket.seq == input.ticket.seq,
                "dispatch preserves ticket seq",
            );
            kani::assert(
                ticket.action == input.action,
                "dispatch preserves action id",
            );
            kani::assert(
                ticket.attempt == input.ticket.attempt,
                "dispatch preserves attempt",
            );
            kani::assert(
                ticket.capacity == 1,
                "dispatch assigns structural capacity one",
            );
        }
        (false, _) => {
            kani::assert(
                false,
                "accepted metadata suspends through production dispatch",
            );
        }
    }
}
