/// Result type for runtime engine operations.
pub type RuntimeEngineResult<T> = Result<T, RuntimeEngineError>;

/// Errors from the runtime engine's action-aware execution.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RuntimeEngineError {
    /// Core engine error.
    #[error("{0}")]
    Core(EngineError),
    /// Action subsystem error.
    #[error("{0}")]
    Action(ActionError),
    /// Retry policy exhausted all attempts.
    #[error("retry exhausted for action {action:?} after {attempts} attempts")]
    RetryExhausted {
        /// Action that exhausted retries.
        action: ActionId,
        /// Number of attempts made.
        attempts: u16,
    },
    /// Taint propagation rejected a clean result from tainted input.
    #[error("taint violation at step {step:?}")]
    TaintViolation {
        /// Step where the violation occurred.
        step: StepIdx,
    },
    /// TogetherStart branch count exceeds the u16 representation limit.
    #[error("branch count {requested} exceeds maximum {max}")]
    BranchLimitExceeded {
        /// Maximum representable branch count.
        max: usize,
        /// Requested branch count.
        requested: usize,
    },
    /// Core engine returned an unknown variant that the runtime cannot
    /// translate (master §45 invalid_state_transition contract).
    #[error("unknown engine signal: {signal_debug}")]
    UnknownEngineSignal {
        /// `Debug` rendering of the offending core engine signal.
        signal_debug: String,
    },
    /// Drive loop completed but one or more `read_slot` evidence
    /// emissions failed and were recorded as gaps (RE-004 follow-up).
    /// The count and last failing slot are preserved so operators can
    /// act on the gap without losing the slot index (black-hat
    /// FINDING-003).
    #[error("evidence gaps recorded: count={count}, last_slot={last_slot:?}")]
    EvidenceGapsRecorded {
        /// Number of evidence gaps recorded during the drive.
        count: u64,
        /// Slot index of the most recent gap, or `None` when no gaps
        /// were recorded. The variant only carries `count > 0`, so
        /// this field is observably `Some(_)` in practice, but the
        /// `Option` keeps the error type honest if the report is ever
        /// reused with a zero count.
        last_slot: Option<SlotIdx>,
    },
}

impl From<EngineError> for RuntimeEngineError {
    fn from(error: EngineError) -> Self {
        Self::Core(error)
    }
}

impl From<ActionError> for RuntimeEngineError {
    fn from(error: ActionError) -> Self {
        Self::Action(error)
    }
}

impl RuntimeEngineError {
    /// Runtime code for exhausted retry policies.
    pub const RETRY_EXHAUSTED_RUNTIME_CODE: &str = "RETRY_EXHAUSTED";
    pub const BRANCH_LIMIT_EXCEEDED_RUNTIME_CODE: &str = "BRANCH_LIMIT_EXCEEDED";
    pub const UNKNOWN_ENGINE_SIGNAL_RUNTIME_CODE: &str = "UNKNOWN_ENGINE_SIGNAL";
    pub const EVIDENCE_GAPS_RECORDED_RUNTIME_CODE: &str = "EVIDENCE_GAPS_RECORDED";

    /// Returns the stable section 17 runtime code when this error has a direct mapping.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::Core(error) => error.runtime_code(),
            Self::Action(error) => error.runtime_code(),
            Self::RetryExhausted { .. } => Some(Self::RETRY_EXHAUSTED_RUNTIME_CODE),
            Self::TaintViolation { .. } => None,
            Self::BranchLimitExceeded { .. } => Some(Self::BRANCH_LIMIT_EXCEEDED_RUNTIME_CODE),
            Self::UnknownEngineSignal { .. } => Some(Self::UNKNOWN_ENGINE_SIGNAL_RUNTIME_CODE),
            Self::EvidenceGapsRecorded { .. } => Some(Self::EVIDENCE_GAPS_RECORDED_RUNTIME_CODE),
        }
    }
}
