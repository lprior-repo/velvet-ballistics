//! Private answer command selector and wire encoding.
#![forbid(unsafe_code)]

use vb_core::ids::SlotIdx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnswerSelector {
    Slot(SlotIdx),
}

impl AnswerSelector {
    /// Slot tickets live above `u16::MAX`, disjoint from step-indexed tickets.
    pub(crate) fn wire_ticket(self) -> u64 {
        match self {
            Self::Slot(slot) => u64::from(slot.get()) | SLOT_TICKET_FLAG,
        }
    }
}

const SLOT_TICKET_FLAG: u64 = 1 << 16;

#[cfg(test)]
mod tests {
    use super::AnswerSelector;
    use vb_core::ids::SlotIdx;

    #[test]
    fn wire_ticket_uses_disjoint_ranges() {
        assert_eq!(AnswerSelector::Slot(SlotIdx::ZERO).wire_ticket(), 65_536);
        assert_eq!(
            AnswerSelector::Slot(SlotIdx::new(u16::MAX)).wire_ticket(),
            131_071
        );
    }
}
