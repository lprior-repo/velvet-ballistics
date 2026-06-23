#![forbid(unsafe_code)]
//! Run state types for active runs.

use vb_core::action::ActionContract;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::primitives::collect::CollectStates;

// ============================================================================
// RunState and Inspect types
// ============================================================================

/// Mutable run state owned directly by the shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunState {
    /// Active run frame.
    pub frame: RunFrame,
    /// Compiled workflow for this run.
    pub workflow: CompiledWorkflow,
    /// Cold value store for list, object, and blob handles.
    pub store: ValueStore,
    /// Per-Do-step attempt counters owned with the live frame.
    pub action_attempts: Box<[u16]>,
    /// Admission record for this run, if admission gating was performed.
    pub admission: Option<crate::admission::RunAdmission>,
    /// Per-run collect pagination state side table.
    pub collect_states: CollectStates,
    /// Validated action contracts used by Do execution.
    pub action_contracts: Box<[ActionContract]>,
    /// Program-counter steps executed at the last successful snapshot.
    ///
    /// Used by the snapshot writer to determine whether enough steps have
    /// elapsed since the last snapshot to justify another one.
    pub last_snapshot_executed: u64,
}

/// Diagnostic snapshot returned by the Inspect command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectSnapshot {
    /// Run identifier.
    pub run: RunId,
    /// Caller correlation identifier.
    pub correlation: u64,
    /// Current program counter.
    pub pc: StepIdx,
    /// Number of executed transitions.
    pub executed: u64,
}

/// Bounded response produced by an inspect command.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InspectResponse {
    /// The run was active and a snapshot was captured.
    Found(InspectSnapshot),
    /// The run was not active on this shard.
    NotFound {
        /// Run identifier.
        run: RunId,
        /// Caller correlation identifier.
        correlation: u64,
    },
    /// The run is in the terminal set but no longer in the active runs map.
    ///
    /// Returned for cancelled, killed, completed, or failed runs that have been
    /// moved to the terminal set. The recorded `outcome` distinguishes how the
    /// run reached the terminal state.
    Terminal {
        /// Run identifier.
        run: RunId,
        /// Caller correlation identifier.
        correlation: u64,
        /// How the run reached the terminal state.
        outcome: TerminalOutcome,
    },
    /// The run was found in the terminal set but its outcome record was missing.
    ///
    /// This indicates a transient shard state inconsistency (terminal_runs contains
    /// the run but terminal_outcomes does not). Callers must NOT treat this as a
    /// normal `Failed` outcome; the absence of an outcome is observable and must
    /// be reported explicitly. Re-running inspect once the shard settles typically
    /// resolves the inconsistency.
    Tombstoned {
        /// Run identifier.
        run: RunId,
        /// Caller correlation identifier.
        correlation: u64,
    },
}

/// Recorded terminal state for a run that has been moved to the terminal set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TerminalOutcome {
    /// The run was cancelled by a `ShardCommand::Cancel`.
    Cancelled,
    /// The run was killed by a `ShardCommand::Kill`.
    Killed,
    /// The run reached its natural `Finished` signal.
    Completed,
    /// The run failed during deterministic execution.
    Failed,
}

// ============================================================================
// RuntimeState and RuntimeEvent
// ============================================================================

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
    /// Resume journal append failed, revert to Resumable state.
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

// ============================================================================
// Resume types
// ============================================================================

/// Status of a resume operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResumeStatus {
    /// Resume was accepted and the run was driven once.
    ///
    /// The post-drive lifecycle may be `Running`, `Resumable`, or terminal,
    /// depending on the deterministic engine signal emitted by that drive.
    Resumed,
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
