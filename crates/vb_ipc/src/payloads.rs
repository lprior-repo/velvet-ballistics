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
    /// Answer a suspended ask ticket.
    AnswerAsk {
        /// Target run identifier.
        run_id: RunId,
        /// Ask ticket identifier.
        ticket: u64,
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
    /// List active and recent runs.
    ListRuns {
        /// Maximum number of runs to return.
        limit: u32,
        /// Filter to runs matching this workflow digest (optional).
        workflow: Option<WorkflowDigest>,
    },
    /// Health probe.
    Health,
    /// Graceful shutdown request.
    Shutdown,
    /// Query runtime metrics.
    GetMetrics,
    /// Get taint report for a compiled workflow.
    GetTaintReport {
        /// Compiled workflow digest to analyze.
        digest: WorkflowDigest,
    },
    /// Retrieve the graph structure of a compiled workflow.
    GetWorkflowGraph {
        /// Compiled workflow digest to look up.
        digest: WorkflowDigest,
    },
    /// Verify a compiled workflow and return validation certificates.
    VerifyWorkflow {
        /// Compiled workflow digest to verify.
        digest: WorkflowDigest,
    },
}

/// Run state reported in list-run responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RunListState {
    /// Run is actively executing or suspended on this shard.
    Active,
    /// Run finished successfully.
    Finished,
    /// Run failed.
    Failed,
    /// Run was cancelled.
    Cancelled,
}

/// Summary of a run for list-run responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    /// Run identifier.
    pub run_id: RunId,
    /// Compiled workflow digest.
    pub workflow: WorkflowDigest,
    /// Current run state.
    pub state: RunListState,
    /// Submitted sequence number (0 for active in-memory runs).
    pub submitted_seq: u64,
    /// Finished sequence number, if the run reached a terminal state.
    pub finished_seq: Option<u64>,
    /// Number of steps in the workflow.
    pub step_count: u16,
    /// Steps that reached a terminal state (Succeeded, Failed, Skipped, or Cancelled).
    pub steps_completed: u16,
}

/// Verification outcome for a compiled workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Per-gate certificate details.
    pub certificates: Vec<CertificateWire>,
    /// Total number of gate checks performed.
    pub total_checks: u32,
    /// Number of gate checks that passed.
    pub pass_count: u32,
    /// Number of gate checks that failed.
    pub fail_count: u32,
}

/// One gate-check certificate in a verification result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateWire {
    /// Gate identifier, e.g. "gate_07_expression_stack_depth".
    pub kind: String,
    /// "Pass" or "Fail".
    pub status: String,
    /// Human-readable details, empty on pass.
    pub details: String,
}

/// One edge in a taint propagation path, serialized for IPC transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaintPathWire {
    /// Source step index.
    pub from: u16,
    /// Destination step index.
    pub to: u16,
    /// Whether this edge is dangerous or just a warning.
    pub status: String,
}

/// Lightweight descriptor for a single workflow node returned by GetWorkflowGraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDescriptor {
    /// Step index of this node.
    pub step_idx: u16,
    /// Kind of this node.
    pub kind: String,
    /// Fallthrough target step index, if any.
    pub next: Option<u16>,
    /// Human-readable step name.
    pub title: String,
}

/// Lightweight descriptor for a workflow edge returned by GetWorkflowGraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeDescriptor {
    /// Source step index.
    pub from: u16,
    /// Target step index.
    pub to: u16,
    /// Optional edge label.
    pub label: Option<String>,
    /// Edge type.
    pub edge_type: String,
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
}
