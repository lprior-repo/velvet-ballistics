#![forbid(unsafe_code)]

use vb_core::action::ActionFailureCode;
use vb_core::ids::{RunId, SlotIdx, StepIdx};

/// A single observable runtime event recorded by a shard trace ring.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TraceEvent {
    /// A step began execution.
    StepStarted {
        /// Run identifier.
        run: RunId,
        /// Step index.
        step: StepIdx,
    },
    /// A step completed execution.
    StepEnded {
        /// Run identifier.
        run: RunId,
        /// Step index.
        step: StepIdx,
    },
    /// A slot was written.
    SlotWritten {
        /// Run identifier.
        run: RunId,
        /// Slot index.
        slot: SlotIdx,
        /// Encoded slot value bytes (postcard-encoded `SlotValue`).
        value: Vec<u8>,
    },
    /// An action was scheduled.
    ActionScheduled {
        /// Run identifier.
        run: RunId,
        /// Step that scheduled the action.
        step: StepIdx,
    },
    /// An action completed.
    ActionCompleted {
        /// Run identifier.
        run: RunId,
        /// Step that received the completion.
        step: StepIdx,
    },
    /// An action failed.
    ActionFailed {
        /// Run identifier.
        run: RunId,
        /// Step that received the failure.
        step: StepIdx,
        /// Failure code.
        code: ActionFailureCode,
    },
    /// An ask was answered.
    AskAnswered {
        /// Run identifier.
        run: RunId,
        /// Step that scheduled the ask.
        step: StepIdx,
        /// Slot that received the answer.
        slot: SlotIdx,
    },
    /// A run was submitted.
    RunSubmitted {
        /// Run identifier.
        run: RunId,
    },
    /// A run finished.
    RunFinished {
        /// Run identifier.
        run: RunId,
    },
    /// A run failed.
    RunFailed {
        /// Run identifier.
        run: RunId,
    },
    /// A run was cancelled.
    RunCancelled {
        /// Run identifier.
        run: RunId,
    },
    /// A run was killed.
    RunKilled {
        /// Run identifier.
        run: RunId,
    },
}

impl TraceEvent {
    /// Returns the run associated with this trace event.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        match self {
            Self::StepStarted { run, .. }
            | Self::StepEnded { run, .. }
            | Self::SlotWritten { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompleted { run, .. }
            | Self::ActionFailed { run, .. }
            | Self::AskAnswered { run, .. }
            | Self::RunSubmitted { run }
            | Self::RunFinished { run }
            | Self::RunFailed { run }
            | Self::RunCancelled { run }
            | Self::RunKilled { run } => *run,
        }
    }

    /// Returns true when this event is terminal evidence for the given run.
    #[must_use]
    pub fn is_terminal_for_run(&self, target: RunId) -> bool {
        match self {
            Self::RunFinished { run }
            | Self::RunFailed { run }
            | Self::RunCancelled { run }
            | Self::RunKilled { run } => *run == target,
            Self::StepStarted { .. }
            | Self::StepEnded { .. }
            | Self::SlotWritten { .. }
            | Self::ActionScheduled { .. }
            | Self::ActionCompleted { .. }
            | Self::ActionFailed { .. }
            | Self::AskAnswered { .. }
            | Self::RunSubmitted { .. } => false,
        }
    }
}
