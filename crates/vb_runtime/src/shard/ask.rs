#![forbid(unsafe_code)]
//! Ask types for external answer handling.

use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

// ============================================================================
// AskTicket and AskAnswer
// ============================================================================

/// Ticket identifying where an ask answer must resume execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskTicket {
    /// Owning run.
    pub run: RunId,
    /// Step that issued the ask and is currently marked asking.
    pub ask_step: StepIdx,
    /// Step that consumes the answer slot, usually an AskResume node.
    pub resume_step: StepIdx,
}

/// Explicit ask answer contract. The caller supplies both payload and destination slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskAnswer {
    /// Ask ticket proving the intended resume point.
    pub ticket: AskTicket,
    /// Slot that receives the answer before resuming.
    pub answer_slot: SlotIdx,
    /// Answer payload.
    pub value: SlotValue,
    /// Answer taint marker.
    pub taint: Taint,
    /// Encoded length of the answer payload in bytes.
    pub encoded_len: u32,
}

impl AskAnswer {
    /// Creates an answer when the caller has not precomputed encoded size.
    #[must_use]
    pub fn new(ticket: AskTicket, answer_slot: SlotIdx, value: SlotValue, taint: Taint) -> Self {
        Self {
            ticket,
            answer_slot,
            value,
            taint,
            encoded_len: 0,
        }
    }

    /// Creates an answer with explicit encoded payload length.
    #[must_use]
    pub fn with_encoded_len(
        ticket: AskTicket,
        answer_slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
        encoded_len: u32,
    ) -> Self {
        Self {
            ticket,
            answer_slot,
            value,
            taint,
            encoded_len,
        }
    }
}
