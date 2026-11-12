#![forbid(unsafe_code)]

use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::recovery::{RecoveryCannotResumeState, RecoveryRuntimeSummary};

use super::boundary::{RecoveredRunBoundary, RecoveredRunBoundaryKind};

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
        Self::CannotResume(NonResumableRecoveryProduct {
            summary,
            state,
            reason: None,
        })
    }

    /// Terminal-lifecycle rejection: the frame seed is otherwise resumable, but the
    /// run's latest terminal evidence (`RunFailed`/`RunFinished`/`RunCancelled`/`RunKilled`)
    /// belongs to its current attempt, so automatic recovery must not re-drive it.
    #[must_use]
    pub(crate) const fn cannot_resume_terminal(summary: RecoveryRuntimeSummary) -> Self {
        Self::CannotResume(NonResumableRecoveryProduct {
            summary,
            state: RecoveryCannotResumeState::RESUMABLE,
            reason: Some("terminal_state"),
        })
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
    /// Explicit reason token. `None` derives the reason from `state`; `Some` overrides
    /// (used for terminal-lifecycle rejections where the frame seed is otherwise
    /// resumable but the run has reached a terminal state).
    reason: Option<&'static str>,
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
        match self.reason {
            Some(reason) => reason,
            None => self.state.unsupported_reason(),
        }
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
    retry_checkpoint: Option<crate::run_checkpoint::RunCheckpointV1>,
    open_gates: Vec<crate::shard::types::OpenGate>,
    /// Original historical admission validated against the durable artifact;
    /// must be restored verbatim rather than rebuilt from current policy.
    admission: crate::admission::RunAdmission,
    /// Restored cold value store for arena-handle resolution, present when
    /// the run retained owned inputs. Rebuilt into the shard `RunState`
    /// instead of the default empty store.
    restored_store: Option<vb_core::value_store::ValueStoreCheckpointV1>,
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
            retry_checkpoint: parts.retry_checkpoint,
            open_gates: parts.open_gates,
            admission: parts.admission,
            restored_store: parts.restored_store,
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
        let command = crate::shard::types::RecoverRunCommand::new(
            self.run,
            self.frame,
            self.artifact_digest,
            self.workflow_digest,
            self.next_seq,
            self.collect_states,
            self.boundary,
        )
        .with_open_gates(self.open_gates)
        .with_validated_admission(self.admission)
        .with_restored_store(self.restored_store);
        match self.retry_checkpoint {
            Some(checkpoint) => command.with_retry_checkpoint(checkpoint),
            None => command,
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
    pub(crate) retry_checkpoint: Option<crate::run_checkpoint::RunCheckpointV1>,
    pub(crate) open_gates: Vec<crate::shard::types::OpenGate>,
    /// Original historical admission validated against the durable artifact;
    /// must be restored verbatim rather than rebuilt from current policy.
    pub(crate) admission: crate::admission::RunAdmission,
    /// Restored cold value store for arena-handle resolution, present when
    /// the run retained owned inputs.
    pub(crate) restored_store: Option<vb_core::value_store::ValueStoreCheckpointV1>,
}
