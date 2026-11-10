/// Lifecycle state of a run tracked by the runtime for resume eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeState {
    /// Run was created but has not yet been started.
    Initial,
    /// Run is actively executing.
    Running,
    /// Run suspended and can be resumed.
    Resumable,
    /// Resume is in flight for this run.
    Resuming,
    /// Run terminated with a failure.
    Failed,
}

impl RuntimeState {
    /// Returns true if this state is a valid target for resume.
    #[must_use]
    pub fn is_resumable(&self) -> bool {
        matches!(self, Self::Resumable)
    }
}

/// Runtime events that drive state transitions in the RuntimeStateMachine.
/// Each variant corresponds to a distinct operational event in the shard lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeEvent {
    /// A new run has been submitted and inserted into the shard.
    Submit,
    /// An existing run is being resumed from a suspended state.
    Resume,
    /// A pre-commit resume or drive append failed; revert to retryable state.
    ResumeRollback,
    /// A run's deterministic execution is continuing after a drive tick.
    DriveContinue,
    /// A run has reached a terminal finished state.
    DriveFinished,
    /// A run is awaiting an external action response.
    AwaitAction,
    /// A run is awaiting a timer (wait or ask timeout).
    AwaitTimer,
    /// A run has reached a terminal failed state.
    Fail,
    /// Remove run from runtime_states tracking (terminal).
    TerminalRemove,
}

impl RuntimeEvent {
    /// Returns true if this event produces a terminal state (run is removed from runtime_states).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Fail | Self::TerminalRemove | Self::DriveFinished
        )
    }

    /// Returns true if this event sets a Resumable state.
    #[must_use]
    pub fn is_resumable(&self) -> bool {
        matches!(
            self,
            Self::AwaitAction | Self::AwaitTimer | Self::ResumeRollback
        )
    }
}

/// Status of a resume operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResumeStatus {
    /// Resume was accepted and the run was driven once.
    ///
    /// The post-drive lifecycle may be `Running`, `Resumable`, or terminal,
    /// depending on the deterministic engine signal emitted by that drive.
    Resumed,
    /// Run was already running when resume was attempted.
    AlreadyRunning,
}

/// Result of a successful resume operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeResult {
    /// The run identifier that was resumed.
    pub run_id: RunId,
    /// The status of the resume operation.
    pub status: ResumeStatus,
    /// Monotonic timestamp when the resume occurred.
    pub timestamp: u64,
}

/// Errors that can occur during a resume operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResumeError {
    /// The run identifier was not found in the journal.
    RunIdNotFound {
        /// The run identifier that was not found.
        run_id: RunId,
    },
    /// The run is not in a resumable state.
    NotResumable {
        /// The run identifier.
        run_id: RunId,
        /// The current state of the run.
        current_state: RuntimeState,
    },
    /// Journal hydration is incomplete for this run.
    IncompleteHydration {
        /// The run identifier.
        run_id: RunId,
    },
    /// An open manual gate blocks resuming the run; decide the gate first.
    GateOpen {
        /// The run identifier that cannot resume yet.
        run_id: RunId,
        /// The open gate blocking the resume.
        gate: vb_core::manual_gate::GateId,
    },
    /// Failed to append the Resumed event to the journal.
    JournalAppendFailed,
    /// Failed to append the Resumed event with a preserved runtime source.
    JournalAppendFailedWithSource {
        /// Runtime failure that caused the journal append failure.
        source: Box<crate::RuntimeError>,
    },
    /// Failed to produce structured output.
    StructuredOutputFailed,
}

impl ResumeError {
    pub(crate) fn journal_append_failed_with_source(source: crate::RuntimeError) -> Self {
        Self::JournalAppendFailedWithSource {
            source: Box::new(source),
        }
    }

    /// Returns the runtime source bound to this resume journal failure on this thread.
    #[must_use]
    pub fn source_runtime_error(&self) -> Option<crate::RuntimeError> {
        match self {
            Self::JournalAppendFailedWithSource { source } => Some(source.as_ref().clone()),
            // NOTE: #[non_exhaustive] - new ResumeError variants return None for source_runtime_error.
            // Implementations should add explicit variant handling.
            _ => None,
        }
    }
}
