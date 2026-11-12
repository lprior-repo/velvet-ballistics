use super::*;
use crate::value::Taint;

fn input_len_contract(action: ActionId, max_input_bytes: u32) -> Result<ActionContract, String> {
    let name = ActionName::new("input-len-contract").map_err(|error| error.to_string())?;
    Ok(ActionContract {
        id: action,
        name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

fn input_len_ticket(run: RunId, step: StepIdx, action: ActionId) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq: SeqNo::ZERO,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    }
}

include!("tests/taint_and_error_variants.rs");
include!("tests/contracts_and_idempotency.rs");
include!("tests/idempotency_verification.rs");
include!("tests/dispatch_and_outcome.rs");
include!("tests/ticket_and_serialization.rs");
