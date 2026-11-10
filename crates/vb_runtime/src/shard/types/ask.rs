/// Ticket identifying where an ask answer must resume execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskTicket {
    /// Owning run.
    pub(crate) run: RunId,
    /// Step that issued the ask and is currently marked asking.
    pub(crate) ask_step: StepIdx,
    /// Step that consumes the answer slot, usually an AskResume node.
    pub(crate) resume_step: StepIdx,
    /// Slot authorized by the compiled AskResume node.
    pub(crate) answer_slot: SlotIdx,
    /// Durable per-run sequence identifying this exact AskScheduled boundary.
    pub(crate) boundary_seq: EventSeq,
}

impl AskTicket {
    /// Returns the owning run.
    #[must_use]
    pub const fn run(self) -> RunId {
        self.run
    }

    /// Returns the step that issued the ask.
    #[must_use]
    pub const fn ask_step(self) -> StepIdx {
        self.ask_step
    }

    /// Returns the step that consumes the answer.
    #[must_use]
    pub const fn resume_step(self) -> StepIdx {
        self.resume_step
    }

    /// Returns the authorized answer slot.
    #[must_use]
    pub const fn answer_slot(self) -> SlotIdx {
        self.answer_slot
    }

    pub(crate) const fn issued(
        run: RunId,
        ask_step: StepIdx,
        resume_step: StepIdx,
        answer_slot: SlotIdx,
        boundary_seq: EventSeq,
    ) -> Self {
        Self {
            run,
            ask_step,
            resume_step,
            answer_slot,
            boundary_seq,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingAskAuthority {
    pub(crate) step: StepIdx,
    pub(crate) boundary_seq: EventSeq,
}

/// Operator-opened manual gate awaiting a decision (MASTER §77.15).
///
/// The gate definition, its opening time, the operator who requested the
/// gated action, and the current delegate after a delegation decision. The
/// opening is durably journaled as a `GateOpenedEvent` before the run is
/// suspended, so recovery can reconstruct the open gate; the outcome record
/// is journaled when the gate is decided or escalated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenGate {
    /// Gate definition as opened by the operator.
    pub(crate) gate: ManualGate,
    /// Opening time in milliseconds since the UNIX epoch.
    pub(crate) opened_at_ms: u64,
    /// Operator who requested the gated action.
    pub(crate) requester: ApproverId,
    /// Delegate target after a delegation decision; later submissions must
    /// come from this approver. `None` until the gate is delegated.
    pub(crate) delegated_to: Option<ApproverId>,
}

/// Explicit ask answer contract. The requested slot must match the AskTicket authority.
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
