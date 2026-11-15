//! Private answer command selector and wire encoding.
#![forbid(unsafe_code)]

use vb_core::ids::{SlotIdx, StepIdx};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnswerSelector {
    Step(StepIdx),
    Slot(SlotIdx),
}

impl AnswerSelector {
    pub(crate) fn wire_ticket(self) -> Option<u64> {
        match self {
            Self::Step(step) => Some(u64::from(step.get())),
            Self::Slot(slot) => u64::from(slot.get()).checked_add(SLOT_TICKET_OFFSET),
        }
    }
}

const SLOT_TICKET_OFFSET: u64 = 65_536;

#[cfg(test)]
mod tests {
    use super::AnswerSelector;
    use vb_core::ids::{SlotIdx, StepIdx};

    #[test]
    fn wire_ticket_uses_disjoint_checked_ranges() {
        assert_eq!(AnswerSelector::Step(StepIdx::ZERO).wire_ticket(), Some(0));
        assert_eq!(
            AnswerSelector::Step(StepIdx::new(u16::MAX)).wire_ticket(),
            Some(65_535)
        );
        assert_eq!(
            AnswerSelector::Slot(SlotIdx::ZERO).wire_ticket(),
            Some(65_536)
        );
        assert_eq!(
            AnswerSelector::Slot(SlotIdx::new(u16::MAX)).wire_ticket(),
            Some(131_071)
        );
    }
}
