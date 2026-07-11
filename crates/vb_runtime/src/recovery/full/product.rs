#![forbid(unsafe_code)]

use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::recovery::{RecoveryCannotResumeState, RecoveryRuntimeSummary};

use super::RecoveredPendingActionTicket;

/// Public recovery product emitted by the durable runtime recovery boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeRecoveryProduct {
    /// Durable evidence is summary-only and cannot be turned into live state.
    SummaryOnly(SummaryRecoveryProduct),
    /// Durable evidence was parsed but cannot safely resume.
    CannotResume(NonResumableRecoveryProduct),
    /// Durable evidence is sufficient to rebuild a live run in the shard.
    Resumable(Box<ResumableRecoveryProduct>),
}

impl RuntimeRecoveryProduct {
    #[must_use]
    pub(crate) const fn cannot_resume(
        summary: RecoveryRuntimeSummary,
        state: RecoveryCannotResumeState,
    ) -> Self {
        Self::CannotResume(NonResumableRecoveryProduct { summary, state })
    }

    #[must_use]
    pub(crate) fn resumable(product: ResumableRecoveryProduct) -> Self {
        Self::Resumable(Box::new(product))
    }

    #[must_use]
    pub const fn summary(&self) -> RecoveryRuntimeSummary {
        match self {
            Self::SummaryOnly(product) => product.summary(),
            Self::CannotResume(product) => product.summary(),
            Self::Resumable(product) => product.summary(),
        }
    }

    #[must_use]
    pub const fn is_resumable(&self) -> bool {
        matches!(self, Self::Resumable(_))
    }

    #[must_use]
    pub const fn cannot_resume_reason(&self) -> Option<&'static str> {
        match self {
            Self::CannotResume(product) => Some(product.reason()),
            Self::SummaryOnly(_) | Self::Resumable(_) => None,
        }
    }
}

/// Summary-only product with private construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryRecoveryProduct {
    summary: RecoveryRuntimeSummary,
}

impl SummaryRecoveryProduct {
    #[must_use]
    pub const fn summary(&self) -> RecoveryRuntimeSummary {
        self.summary
    }
}

/// Cannot-resume product with an explicit typed reason witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonResumableRecoveryProduct {
    summary: RecoveryRuntimeSummary,
    state: RecoveryCannotResumeState,
}

impl NonResumableRecoveryProduct {
    #[must_use]
    pub const fn summary(&self) -> RecoveryRuntimeSummary {
        self.summary
    }

    #[must_use]
    pub const fn state(&self) -> RecoveryCannotResumeState {
        self.state
    }

    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.state.unsupported_reason()
    }
}

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
}

impl RecoveredOpenAsk {
    #[must_use]
    pub(crate) const fn new(step: StepIdx) -> Self {
        Self { step }
    }

    #[must_use]
    pub const fn step(self) -> StepIdx {
        self.step
    }
}

/// Resumable product with private construction and private live-frame fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumableRecoveryProduct {
    run: RunId,
    summary: RecoveryRuntimeSummary,
    frame: RunFrame,
    artifact_digest: WorkflowDigest,
    workflow_digest: WorkflowDigest,
    next_seq: EventSeq,
    collect_states: crate::primitives::collect::CollectStates,
    boundary: RecoveredRunBoundary,
}

impl ResumableRecoveryProduct {
    #[must_use]
    pub(crate) fn new(parts: ResumableRecoveryParts) -> Self {
        Self {
            run: parts.run,
            summary: parts.summary,
            frame: parts.frame,
            artifact_digest: parts.artifact_digest,
            workflow_digest: parts.workflow_digest,
            next_seq: parts.next_seq,
            collect_states: parts.collect_states,
            boundary: parts.boundary,
        }
    }

    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run
    }

    #[must_use]
    pub const fn summary(&self) -> RecoveryRuntimeSummary {
        self.summary
    }

    #[must_use]
    pub const fn boundary_kind(&self) -> RecoveredRunBoundaryKind {
        self.boundary.kind()
    }

    #[must_use]
    pub(crate) fn into_recover_command(self) -> crate::shard::types::RecoverRunCommand {
        crate::shard::types::RecoverRunCommand {
            run: self.run,
            frame: self.frame,
            artifact_digest: self.artifact_digest,
            workflow_digest: self.workflow_digest,
            next_seq: self.next_seq,
            collect_states: self.collect_states,
            boundary: self.boundary,
        }
    }
}

/// Constructor arguments for a resumable recovery product.
pub(crate) struct ResumableRecoveryParts {
    pub(crate) run: RunId,
    pub(crate) summary: RecoveryRuntimeSummary,
    pub(crate) frame: RunFrame,
    pub(crate) artifact_digest: WorkflowDigest,
    pub(crate) workflow_digest: WorkflowDigest,
    pub(crate) next_seq: EventSeq,
    pub(crate) collect_states: crate::primitives::collect::CollectStates,
    pub(crate) boundary: RecoveredRunBoundary,
}
