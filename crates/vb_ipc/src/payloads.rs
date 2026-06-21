#![forbid(unsafe_code)]
//! IPC payload types.

use serde::{Deserialize, Serialize};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::Taint;
use vb_core::{RunId, WorkflowDigest};

/// Submit a compiled workflow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitRunPayload {
    /// Caller-selected run identifier.
    pub run_id: RunId,
    /// Compiled workflow digest.
    pub workflow: WorkflowDigest,
    /// Runtime input bytes owned by the IPC payload.
    pub input: Vec<u8>,
}

/// Payloads accepted by the binary IPC command surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IpcPayload {
    /// Submit a compiled workflow run.
    SubmitRun(SubmitRunPayload),
    /// Submit a compiled workflow run with inline inputs.
    SubmitRunInline(SubmitRunPayload),
    /// Cancel a run.
    CancelRun {
        /// Target run identifier.
        run_id: RunId,
        /// Optional caller-supplied reason recorded in the journal entry.
        ///
        /// When `None`, the cancellation is recorded with no reason string.
        /// When `Some`, the reason is passed through to
        /// [`Runtime::cancel_run_with_reason`] so it lands on the durable
        /// `RunCancelled` journal event (RQ-W0-11).
        reason: Option<Vec<u8>>,
    },
    /// Inspect a run.
    InspectRun {
        /// Target run identifier.
        run_id: RunId,
    },
    /// List run events from a sequence number.
    ListEvents {
        /// Target run identifier.
        run_id: RunId,
        /// First event sequence to return.
        from_sequence: u64,
    },
    /// Answer the pending ask for a run.
    AnswerAsk {
        /// Target run identifier.
        run_id: RunId,
        /// Destination slot that receives the decoded answer.
        answer_slot: SlotIdx,
        /// Postcard-compatible answer bytes.
        answer: Vec<u8>,
        /// Taint classification of the answer value.
        ///
        /// When None, the runtime treats the answer as clean for backward-compatible callers.
        taint: Option<Taint>,
    },
    /// Complete an external action ticket.
    CompleteAction {
        /// Target run identifier.
        run_id: RunId,
        /// Action ticket identifier.
        ticket: u64,
        /// Action output bytes.
        output: Vec<u8>,
    },
    /// Fail an external action ticket.
    FailAction {
        /// Target run identifier.
        run_id: RunId,
        /// Action ticket identifier.
        ticket: u64,
        /// Encoded failure payload.
        error: Vec<u8>,
    },
    /// Drain trace records for a run.
    DrainTrace {
        /// Target run identifier.
        run_id: RunId,
        /// Maximum records to return.
        max_records: u32,
    },
    /// Health probe.
    Health,
    /// Graceful shutdown request.
    Shutdown,
}

/// Typed trace event returned by `ListEvents`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcTraceEvent {
    /// Monotonic sequence assigned by the IPC snapshot response.
    pub sequence: u64,
    /// Event payload.
    pub kind: IpcTraceEventKind,
}

/// Stable IPC event payload independent of runtime internals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IpcTraceEventKind {
    /// A step began execution.
    StepStarted { run: RunId, step: StepIdx },
    /// A step completed execution.
    StepEnded { run: RunId, step: StepIdx },
    /// A slot was written.
    SlotWritten {
        run: RunId,
        slot: SlotIdx,
        /// Encoded slot value bytes.
        value: Vec<u8>,
    },
    /// An action was scheduled.
    ActionScheduled { run: RunId, step: StepIdx },
    /// An action completed.
    ActionCompleted { run: RunId, step: StepIdx },
    /// An action failed.
    ActionFailed {
        run: RunId,
        step: StepIdx,
        code: vb_core::action::ActionFailureCode,
    },
    /// An ask was answered.
    AskAnswered {
        run: RunId,
        step: StepIdx,
        slot: SlotIdx,
    },
    /// A run was submitted.
    RunSubmitted { run: RunId },
    /// A run finished.
    RunFinished { run: RunId },
    /// A run failed.
    RunFailed { run: RunId },
    /// A run was cancelled.
    RunCancelled { run: RunId },
    /// A run was killed (RQ-W0-09).
    RunKilled { run: RunId },
    /// An unknown event (for future compatibility).
    #[doc(hidden)]
    Unknown,
}

#[cfg(test)]
#[path = "payloads/tests.rs"]
mod tests;
