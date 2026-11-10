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
    /// A wait suspended on a durable timer schedule was recovered.
    WaitingTimer,
    /// An ask with timer authority was recovered from the frame state.
    TimedAsk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveredRunBoundaryInner {
    None,
    PendingAction(RecoveredPendingActionTicket),
    OpenAsk(RecoveredOpenAsk),
    WaitingTimer(RecoveredTimerSuspension),
    TimedAsk(RecoveredTimerSuspension),
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
    pub(crate) const fn from_waiting_timer(timer: RecoveredTimerSuspension) -> Self {
        Self {
            inner: RecoveredRunBoundaryInner::WaitingTimer(timer),
        }
    }

    #[must_use]
    pub(crate) const fn from_timed_ask(ask: RecoveredTimerSuspension) -> Self {
        Self {
            inner: RecoveredRunBoundaryInner::TimedAsk(ask),
        }
    }

    #[must_use]
    pub const fn kind(self) -> RecoveredRunBoundaryKind {
        match self.inner {
            RecoveredRunBoundaryInner::None => RecoveredRunBoundaryKind::None,
            RecoveredRunBoundaryInner::PendingAction(_) => RecoveredRunBoundaryKind::PendingAction,
            RecoveredRunBoundaryInner::OpenAsk(_) => RecoveredRunBoundaryKind::OpenAsk,
            RecoveredRunBoundaryInner::WaitingTimer(_) => RecoveredRunBoundaryKind::WaitingTimer,
            RecoveredRunBoundaryInner::TimedAsk(_) => RecoveredRunBoundaryKind::TimedAsk,
        }
    }

    #[must_use]
    pub(crate) const fn pending_action_ticket(self) -> Option<RecoveredPendingActionTicket> {
        match self.inner {
            RecoveredRunBoundaryInner::PendingAction(ticket) => Some(ticket),
            RecoveredRunBoundaryInner::None
            | RecoveredRunBoundaryInner::OpenAsk(_)
            | RecoveredRunBoundaryInner::WaitingTimer(_)
            | RecoveredRunBoundaryInner::TimedAsk(_) => None,
        }
    }

    #[must_use]
    pub const fn open_ask(self) -> Option<RecoveredOpenAsk> {
        match self.inner {
            RecoveredRunBoundaryInner::OpenAsk(ask) => Some(ask),
            RecoveredRunBoundaryInner::None
            | RecoveredRunBoundaryInner::PendingAction(_)
            | RecoveredRunBoundaryInner::WaitingTimer(_)
            | RecoveredRunBoundaryInner::TimedAsk(_) => None,
        }
    }

    #[must_use]
    pub const fn waiting_timer(self) -> Option<RecoveredTimerSuspension> {
        match self.inner {
            RecoveredRunBoundaryInner::WaitingTimer(timer) => Some(timer),
            RecoveredRunBoundaryInner::None
            | RecoveredRunBoundaryInner::PendingAction(_)
            | RecoveredRunBoundaryInner::OpenAsk(_)
            | RecoveredRunBoundaryInner::TimedAsk(_) => None,
        }
    }

    #[must_use]
    pub const fn timed_ask(self) -> Option<RecoveredTimerSuspension> {
        match self.inner {
            RecoveredRunBoundaryInner::TimedAsk(ask) => Some(ask),
            RecoveredRunBoundaryInner::None
            | RecoveredRunBoundaryInner::PendingAction(_)
            | RecoveredRunBoundaryInner::OpenAsk(_)
            | RecoveredRunBoundaryInner::WaitingTimer(_) => None,
        }
    }

    #[must_use]
    pub(crate) fn is_open_ask_step(self, step: StepIdx) -> bool {
        match self.inner {
            RecoveredRunBoundaryInner::OpenAsk(ask) => ask.step == step,
            RecoveredRunBoundaryInner::None
            | RecoveredRunBoundaryInner::PendingAction(_)
            | RecoveredRunBoundaryInner::WaitingTimer(_)
            | RecoveredRunBoundaryInner::TimedAsk(_) => false,
        }
    }

    #[must_use]
    pub(crate) fn is_waiting_timer_step(self, step: StepIdx) -> bool {
        match self.inner {
            RecoveredRunBoundaryInner::WaitingTimer(timer) => timer.step == step,
            RecoveredRunBoundaryInner::None
            | RecoveredRunBoundaryInner::PendingAction(_)
            | RecoveredRunBoundaryInner::OpenAsk(_)
            | RecoveredRunBoundaryInner::TimedAsk(_) => false,
        }
    }

    #[must_use]
    pub(crate) fn is_timed_ask_step(self, step: StepIdx) -> bool {
        match self.inner {
            RecoveredRunBoundaryInner::TimedAsk(ask) => ask.step == step,
            RecoveredRunBoundaryInner::None
            | RecoveredRunBoundaryInner::PendingAction(_)
            | RecoveredRunBoundaryInner::OpenAsk(_)
            | RecoveredRunBoundaryInner::WaitingTimer(_) => false,
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

/// Durable proof that a suspended step held timer authority to recover.
///
/// Covers both waiting waits (`WaitingTimer`: the wait node registered a
/// durable `WaitScheduledEvent`) and timed asks (`TimedAsk`: the ask node
/// carried a timeout slot, so its `AskScheduledEvent` also armed a timer).
/// The wall-clock deadline itself is not durable; the shard recomputes it
/// from the recovered timeout slot with the same helper the live drive path
/// uses, so a recovered timer is conservative (full timeout from recovery)
/// rather than expired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredTimerSuspension {
    step: StepIdx,
    boundary_seq: vb_storage::EventSeq,
}

impl RecoveredTimerSuspension {
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
