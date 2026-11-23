#![forbid(unsafe_code)]
//! Ticket conversion helpers.

use vb_core::ids::{SlotIdx, StepIdx};

/// Answer target carried by an `AnswerAsk` wire ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnswerTarget {
    Step(StepIdx),
    Slot(SlotIdx),
}

/// Tickets up to `u16::MAX` name an ask step; tickets in `65536..=131071`
/// name an answer slot (`ticket - 65536`). Anything else is rejected.
pub(crate) fn answer_target_from_ticket(ticket: u64) -> Option<AnswerTarget> {
    if let Ok(step) = u16::try_from(ticket) {
        return Some(AnswerTarget::Step(StepIdx::new(step)));
    }
    let slot = u16::try_from(ticket.checked_sub(SLOT_TICKET_BASE)?).ok()?;
    Some(AnswerTarget::Slot(SlotIdx::new(slot)))
}

const SLOT_TICKET_BASE: u64 = 1 << 16;

pub(crate) fn step_from_ticket(ticket: u64) -> Option<StepIdx> {
    match u16::try_from(ticket) {
        Ok(step) => Some(StepIdx::new(step)),
        Err(_) => None,
    }
}

/// Returns the step index carried in the lower 16 bits of a wire ticket.
///
/// This is the only field derivable from the wire ticket alone. The remaining
/// `ActionTicket` fields (`seq`, `action`, `attempt`, `idempotency_key`,
/// `capacity`) MUST be reconstructed from runtime state via
/// `Runtime::lookup_pending_action_ticket`. The IPC layer MUST NOT fabricate
/// these fields from defaults; doing so bypasses the idempotency-key check
/// and the per-action validation chain (see bead vb-xb62s).
#[must_use]
pub fn wire_ticket_step(ticket: u64) -> Option<StepIdx> {
    step_from_ticket(ticket)
}

pub fn payload_len(len: usize) -> u32 {
    u32::try_from(len).map_or(u32::MAX, |value| value)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── wire_ticket_step / step_from_ticket tests ──

    #[test]
    fn wire_ticket_step_returns_step_for_zero() {
        assert_eq!(wire_ticket_step(0), Some(StepIdx::new(0)));
    }

    #[test]
    fn wire_ticket_step_returns_step_for_valid_u16() {
        assert_eq!(wire_ticket_step(10), Some(StepIdx::new(10)));
    }

    #[test]
    fn wire_ticket_step_returns_step_for_max_u16() {
        assert_eq!(
            wire_ticket_step(u16::MAX as u64),
            Some(StepIdx::new(u16::MAX))
        );
    }

    #[test]
    fn wire_ticket_step_returns_none_for_u16_max_plus_one() {
        assert_eq!(wire_ticket_step(u16::MAX as u64 + 1), None);
    }

    #[test]
    fn wire_ticket_step_returns_none_for_u64_max() {
        assert_eq!(wire_ticket_step(u64::MAX), None);
    }

    #[test]
    fn wire_ticket_step_returns_none_for_large_value() {
        assert_eq!(wire_ticket_step(100_000), None);
    }

    #[test]
    fn wire_ticket_step_mid_range_u16_value() {
        assert_eq!(wire_ticket_step(1000), Some(StepIdx::new(1000)));
    }

    #[test]
    fn wire_ticket_step_allocates_sequential_steps() {
        for i in 0..5u16 {
            assert_eq!(wire_ticket_step(u64::from(i)), Some(StepIdx::new(i)));
        }
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
    fn payload_len_exact_u32_max() {
        let val = u32::MAX as usize;
        assert_eq!(payload_len(val), u32::MAX);
    }

    #[test]
    fn payload_len_over_u32_max_saturates() {
        let val = u32::MAX as usize + 1;
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

    #[test]
    fn answer_target_splits_step_and_slot_ranges() {
        assert_eq!(
            answer_target_from_ticket(7),
            Some(AnswerTarget::Step(StepIdx::new(7)))
        );
        assert_eq!(
            answer_target_from_ticket(65_536),
            Some(AnswerTarget::Slot(SlotIdx::new(0)))
        );
        assert_eq!(
            answer_target_from_ticket(131_071),
            Some(AnswerTarget::Slot(SlotIdx::new(u16::MAX)))
        );
        assert_eq!(answer_target_from_ticket(131_072), None);
        assert_eq!(answer_target_from_ticket(u64::MAX), None);
    }
}
