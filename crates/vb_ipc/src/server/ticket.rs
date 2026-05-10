#![forbid(unsafe_code)]
//! Ticket conversion helpers.

use vb_core::RunId;
use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};

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
        capacity: 1,
    })
}

pub fn payload_len(len: usize) -> u32 {
    u32::try_from(len).map_or(u32::MAX, |value| value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::SeqNo;

    // ── step_from_ticket tests ──

    #[test]
    fn step_from_ticket_returns_step_for_valid_u16() {
        let result = step_from_ticket(0);
        assert!(result.is_some(), "ticket 0 should produce a valid step");
        let Some(step) = result else { return };
        assert_eq!(step, StepIdx::new(0));
    }

    #[test]
    fn step_from_ticket_returns_step_for_max_u16() {
        let result = step_from_ticket(u64::from(u16::MAX));
        assert!(
            result.is_some(),
            "ticket u16::MAX should produce a valid step"
        );
        let Some(step) = result else { return };
        assert_eq!(step, StepIdx::new(u16::MAX));
    }

    #[test]
    fn step_from_ticket_returns_none_for_u16_max_plus_one() {
        let result = step_from_ticket(u64::from(u16::MAX) + 1);
        assert!(result.is_none(), "ticket u16::MAX+1 should be out of range");
    }

    #[test]
    fn step_from_ticket_returns_none_for_u64_max() {
        let result = step_from_ticket(u64::MAX);
        assert!(result.is_none(), "ticket u64::MAX should be out of range");
    }

    #[test]
    fn step_from_ticket_returns_none_for_large_value() {
        let result = step_from_ticket(100_000);
        assert!(result.is_none(), "ticket 100000 exceeds u16 range");
    }

    #[test]
    fn step_from_ticket_mid_range_u16_value() {
        let result = step_from_ticket(1000);
        assert!(result.is_some());
        let Some(step) = result else { return };
        assert_eq!(step, StepIdx::new(1000));
    }

    // ── action_ticket_from_wire tests ──

    #[test]
    fn action_ticket_from_wire_returns_ticket_for_valid_step() {
        let run_id = RunId::new(42);
        let result = action_ticket_from_wire(run_id, 10);
        assert!(result.is_some(), "valid ticket should produce ActionTicket");
        let ticket = result;
        let Some(ticket) = ticket else { return };
        assert_eq!(ticket.run, run_id);
        assert_eq!(ticket.step, StepIdx::new(10));
        assert_eq!(ticket.seq, SeqNo::ZERO);
        assert_eq!(ticket.action, vb_core::ids::ActionId::new(0));
        assert_eq!(ticket.attempt, 1);
        assert_eq!(ticket.idempotency_key, 0);
    }

    #[test]
    fn action_ticket_from_wire_returns_ticket_for_zero() {
        let run_id = RunId::new(1);
        let result = action_ticket_from_wire(run_id, 0);
        assert!(result.is_some(), "ticket 0 should produce ActionTicket");
        let Some(ticket) = result else { return };
        assert_eq!(ticket.step, StepIdx::new(0));
    }

    #[test]
    fn action_ticket_from_wire_returns_none_for_overflow() {
        let run_id = RunId::new(1);
        let result = action_ticket_from_wire(run_id, u64::from(u16::MAX) + 1);
        assert!(result.is_none(), "ticket exceeding u16 should return None");
    }

    #[test]
    fn action_ticket_from_wire_returns_none_for_u64_max() {
        let run_id = RunId::new(99);
        let result = action_ticket_from_wire(run_id, u64::MAX);
        assert!(result.is_none());
    }

    #[test]
    fn action_ticket_from_wire_preserves_run_id() {
        let run_id = RunId::new(u64::MAX);
        let result = action_ticket_from_wire(run_id, 5);
        assert!(result.is_some());
        let Some(ticket) = result else { return };
        assert_eq!(ticket.run, run_id);
    }

    // ── payload_len tests ──

    #[test]
    fn payload_len_zero() {
        assert_eq!(payload_len(0), 0);
    }

    #[test]
    fn payload_len_small_value() {
        assert_eq!(payload_len(100), 100);
    }

    #[test]
    #[allow(clippy::as_conversions)]
    fn payload_len_exact_u32_max() {
        // u32::MAX always fits in usize on all supported platforms
        let val = u32::MAX as usize;
        assert_eq!(payload_len(val), u32::MAX);
    }

    #[test]
    #[allow(clippy::as_conversions)]
    fn payload_len_over_u32_max_saturates() {
        // u32::MAX always fits in usize on all supported platforms; overflow test
        let val = (u32::MAX as usize).saturating_add(1);
        assert_eq!(
            payload_len(val),
            u32::MAX,
            "overflow should saturate to u32::MAX"
        );
    }

    #[test]
    fn payload_len_large_value_saturates() {
        assert_eq!(payload_len(usize::MAX), u32::MAX);
    }
}
