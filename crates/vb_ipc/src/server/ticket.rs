//! Ticket conversion helpers.

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};
use vb_core::RunId;

pub(crate) fn step_from_ticket(ticket: u64) -> Option<StepIdx> {
    match u16::try_from(ticket) {
        Ok(step) => Some(StepIdx::new(step)),
        Err(_) => None,
    }
}

pub fn action_ticket_from_wire(run_id: RunId, ticket: u64) -> Option<ActionTicket> {
    let step = step_from_ticket(ticket)?;
    Some(ActionTicket {
        run: run_id,
        step,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
    })
}

pub fn payload_len(len: usize) -> u32 {
    u32::try_from(len).map_or(u32::MAX, |value| value)
}
