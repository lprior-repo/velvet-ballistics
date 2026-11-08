#![forbid(unsafe_code)]

use vb_core::ids::StepIdx;

use super::RecoveredPendingActionTicket;

/// Kind tag for a recovered external boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveredRunBoundaryKind {
    /// No suspended external boundary remains.
    None,
    /// A durable action ticket was recovered.
    PendingAction,
    /// An ask with no timeout was recovered from the frame state.
    OpenAsk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveredRunBoundaryInner {
    None,
    PendingAction(RecoveredPendingActionTicket),
    OpenAsk(RecoveredOpenAsk),
}

/// Typed recovered external-boundary authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredRunBoundary {
    inner: RecoveredRunBoundaryInner,
}

impl RecoveredRunBoundary {
    #[must_use]
    pub(crate) const fn none() -> Self {
        Self {
            inner: RecoveredRunBoundaryInner::None,
        }
    }

    #[must_use]
    pub(crate) const fn from_pending_action(ticket: RecoveredPendingActionTicket) -> Self {
        Self {
            inner: RecoveredRunBoundaryInner::PendingAction(ticket),
        }
    }

    #[must_use]
    pub(crate) const fn from_open_ask(ask: RecoveredOpenAsk) -> Self {
        Self {
            inner: RecoveredRunBoundaryInner::OpenAsk(ask),
        }
    }

    #[must_use]
    pub const fn kind(self) -> RecoveredRunBoundaryKind {
        match self.inner {
            RecoveredRunBoundaryInner::None => RecoveredRunBoundaryKind::None,
            RecoveredRunBoundaryInner::PendingAction(_) => RecoveredRunBoundaryKind::PendingAction,
            RecoveredRunBoundaryInner::OpenAsk(_) => RecoveredRunBoundaryKind::OpenAsk,
        }
    }

    #[must_use]
    pub(crate) const fn pending_action_ticket(self) -> Option<RecoveredPendingActionTicket> {
        match self.inner {
            RecoveredRunBoundaryInner::PendingAction(ticket) => Some(ticket),
            RecoveredRunBoundaryInner::None | RecoveredRunBoundaryInner::OpenAsk(_) => None,
        }
    }

    #[must_use]
    pub const fn open_ask(self) -> Option<RecoveredOpenAsk> {
        match self.inner {
            RecoveredRunBoundaryInner::OpenAsk(ask) => Some(ask),
            RecoveredRunBoundaryInner::None | RecoveredRunBoundaryInner::PendingAction(_) => None,
        }
    }

    #[must_use]
    pub(crate) fn is_open_ask_step(self, step: StepIdx) -> bool {
        match self.inner {
            RecoveredRunBoundaryInner::OpenAsk(ask) => ask.step == step,
            RecoveredRunBoundaryInner::None | RecoveredRunBoundaryInner::PendingAction(_) => false,
        }
    }
}

/// Durable proof that a suspended ask had no timer authority to recover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredOpenAsk {
    step: StepIdx,
    boundary_seq: vb_storage::EventSeq,
}

impl RecoveredOpenAsk {
    #[must_use]
    pub(crate) const fn new(step: StepIdx, boundary_seq: vb_storage::EventSeq) -> Self {
        Self { step, boundary_seq }
    }

    #[must_use]
    pub const fn step(self) -> StepIdx {
        self.step
    }

    #[must_use]
    pub(crate) const fn boundary_seq(self) -> vb_storage::EventSeq {
        self.boundary_seq
    }
}
