#![forbid(unsafe_code)]
//! IPC payload types.

use serde::{Deserialize, Serialize};
use thiserror::Error;
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

/// Gate kind identifiers for verification certificates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    #[serde(rename = "gate_07_expression_stack_depth")]
    Gate07ExpressionStackDepth,
    #[serde(rename = "gate_08_accessor_path_segments")]
    Gate08AccessorPathSegments,
    #[serde(rename = "gate_09_slot_references")]
    Gate09SlotReferences,
    #[serde(rename = "gate_10_node_kind_specific")]
    Gate10NodeKindSpecific,
    #[serde(rename = "gate_11_loop_body_graph")]
    Gate11LoopBodyGraph,
    #[serde(rename = "gate_12_action_contract_completeness")]
    Gate12ActionContractCompleteness,
    #[serde(rename = "gate_13_no_slot_cycles")]
    Gate13NoSlotCycles,
    #[serde(rename = "gate_14_slot_type_consistency")]
    Gate14SlotTypeConsistency,
    #[serde(rename = "gate_15_determinism_proof")]
    Gate15DeterminismProof,
}

/// Failure to parse a verification gate kind from its wire name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("unknown verification gate kind")]
pub struct ParseGateKindError;

impl GateKind {
    /// Returns the string representation used on the wire.
    pub const fn as_str(&self) -> &'static str {
        match self {
            GateKind::Gate07ExpressionStackDepth => "gate_07_expression_stack_depth",
            GateKind::Gate08AccessorPathSegments => "gate_08_accessor_path_segments",
            GateKind::Gate09SlotReferences => "gate_09_slot_references",
            GateKind::Gate10NodeKindSpecific => "gate_10_node_kind_specific",
            GateKind::Gate11LoopBodyGraph => "gate_11_loop_body_graph",
            GateKind::Gate12ActionContractCompleteness => "gate_12_action_contract_completeness",
            GateKind::Gate13NoSlotCycles => "gate_13_no_slot_cycles",
            GateKind::Gate14SlotTypeConsistency => "gate_14_slot_type_consistency",
            GateKind::Gate15DeterminismProof => "gate_15_determinism_proof",
        }
    }
}

impl TryFrom<&str> for GateKind {
    type Error = ParseGateKindError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "gate_07_expression_stack_depth" => Ok(GateKind::Gate07ExpressionStackDepth),
            "gate_08_accessor_path_segments" => Ok(GateKind::Gate08AccessorPathSegments),
            "gate_09_slot_references" => Ok(GateKind::Gate09SlotReferences),
            "gate_10_node_kind_specific" => Ok(GateKind::Gate10NodeKindSpecific),
            "gate_11_loop_body_graph" => Ok(GateKind::Gate11LoopBodyGraph),
            "gate_12_action_contract_completeness" => {
                Ok(GateKind::Gate12ActionContractCompleteness)
            }
            "gate_13_no_slot_cycles" => Ok(GateKind::Gate13NoSlotCycles),
            "gate_14_slot_type_consistency" => Ok(GateKind::Gate14SlotTypeConsistency),
            "gate_15_determinism_proof" => Ok(GateKind::Gate15DeterminismProof),
            _ => Err(ParseGateKindError),
        }
    }
}

/// Pass/fail status for verification results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PassFail {
    Pass,
    Fail,
}

/// Status of a taint propagation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaintPathStatus {
    Dangerous,
    Warning,
}

/// Kind of a workflow node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum NodeKind {
    Nop,
    SetConst,
    Copy,
    EvalExpr,
    BuildObject,
    BuildList,
    Do,
    Choose,
    ChooseSlot,
    ForEachStart,
    ForEachNext,
    ForEachJoin,
    TogetherStart,
    TogetherBranch,
    TogetherJoin,
    CollectStart,
    CollectPage,
    CollectNext,
    CollectFinish,
    ReduceStart,
    ReduceNext,
    ReduceFinish,
    RepeatStart,
    RepeatAttempt,
    RepeatCheck,
    RepeatFinish,
    WaitUntil,
    WaitEvent,
    Ask,
    AskResume,
    RetryCheck,
    ErrorHandler,
    Jump,
    Finish,
}

impl NodeKind {
    /// Returns the string representation used on the wire.
    pub const fn as_str(&self) -> &'static str {
        match self {
            NodeKind::Nop => "Nop",
            NodeKind::SetConst => "SetConst",
            NodeKind::Copy => "Copy",
            NodeKind::EvalExpr => "EvalExpr",
            NodeKind::BuildObject => "BuildObject",
            NodeKind::BuildList => "BuildList",
            NodeKind::Do => "Do",
            NodeKind::Choose => "Choose",
            NodeKind::ChooseSlot => "ChooseSlot",
            NodeKind::ForEachStart => "ForEachStart",
            NodeKind::ForEachNext => "ForEachNext",
            NodeKind::ForEachJoin => "ForEachJoin",
            NodeKind::TogetherStart => "TogetherStart",
            NodeKind::TogetherBranch => "TogetherBranch",
            NodeKind::TogetherJoin => "TogetherJoin",
            NodeKind::CollectStart => "CollectStart",
            NodeKind::CollectPage => "CollectPage",
            NodeKind::CollectNext => "CollectNext",
            NodeKind::CollectFinish => "CollectFinish",
            NodeKind::ReduceStart => "ReduceStart",
            NodeKind::ReduceNext => "ReduceNext",
            NodeKind::ReduceFinish => "ReduceFinish",
            NodeKind::RepeatStart => "RepeatStart",
            NodeKind::RepeatAttempt => "RepeatAttempt",
            NodeKind::RepeatCheck => "RepeatCheck",
            NodeKind::RepeatFinish => "RepeatFinish",
            NodeKind::WaitUntil => "WaitUntil",
            NodeKind::WaitEvent => "WaitEvent",
            NodeKind::Ask => "Ask",
            NodeKind::AskResume => "AskResume",
            NodeKind::RetryCheck => "RetryCheck",
            NodeKind::ErrorHandler => "ErrorHandler",
            NodeKind::Jump => "Jump",
            NodeKind::Finish => "Finish",
        }
    }
}

impl From<&str> for NodeKind {
    fn from(s: &str) -> Self {
        match s {
            "Nop" => NodeKind::Nop,
            "SetConst" => NodeKind::SetConst,
            "Copy" => NodeKind::Copy,
            "EvalExpr" => NodeKind::EvalExpr,
            "BuildObject" => NodeKind::BuildObject,
            "BuildList" => NodeKind::BuildList,
            "Do" => NodeKind::Do,
            "Choose" => NodeKind::Choose,
            "ChooseSlot" => NodeKind::ChooseSlot,
            "ForEachStart" => NodeKind::ForEachStart,
            "ForEachNext" => NodeKind::ForEachNext,
            "ForEachJoin" => NodeKind::ForEachJoin,
            "TogetherStart" => NodeKind::TogetherStart,
            "TogetherBranch" => NodeKind::TogetherBranch,
            "TogetherJoin" => NodeKind::TogetherJoin,
            "CollectStart" => NodeKind::CollectStart,
            "CollectPage" => NodeKind::CollectPage,
            "CollectNext" => NodeKind::CollectNext,
            "CollectFinish" => NodeKind::CollectFinish,
            "ReduceStart" => NodeKind::ReduceStart,
            "ReduceNext" => NodeKind::ReduceNext,
            "ReduceFinish" => NodeKind::ReduceFinish,
            "RepeatStart" => NodeKind::RepeatStart,
            "RepeatAttempt" => NodeKind::RepeatAttempt,
            "RepeatCheck" => NodeKind::RepeatCheck,
            "RepeatFinish" => NodeKind::RepeatFinish,
            "WaitUntil" => NodeKind::WaitUntil,
            "WaitEvent" => NodeKind::WaitEvent,
            "Ask" => NodeKind::Ask,
            "AskResume" => NodeKind::AskResume,
            "RetryCheck" => NodeKind::RetryCheck,
            "ErrorHandler" => NodeKind::ErrorHandler,
            "Jump" => NodeKind::Jump,
            "Finish" => NodeKind::Finish,
            _ => NodeKind::Nop,
        }
    }
}

/// Type of a workflow edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Branch,
    LoopBody,
    LoopExit,
    ParallelBranch,
    ParallelJoin,
    Fallthrough,
    ErrorHandler,
    Jump,
}

impl EdgeType {
    /// Returns the string representation used on the wire.
    pub const fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Branch => "branch",
            EdgeType::LoopBody => "loop_body",
            EdgeType::LoopExit => "loop_exit",
            EdgeType::ParallelBranch => "parallel_branch",
            EdgeType::ParallelJoin => "parallel_join",
            EdgeType::Fallthrough => "fallthrough",
            EdgeType::ErrorHandler => "error_handler",
            EdgeType::Jump => "jump",
        }
    }
}

impl From<&str> for EdgeType {
    fn from(s: &str) -> Self {
        match s {
            "branch" => EdgeType::Branch,
            "loop_body" => EdgeType::LoopBody,
            "loop_exit" => EdgeType::LoopExit,
            "parallel_branch" => EdgeType::ParallelBranch,
            "parallel_join" => EdgeType::ParallelJoin,
            "fallthrough" => EdgeType::Fallthrough,
            "error_handler" => EdgeType::ErrorHandler,
            "jump" => EdgeType::Jump,
            _ => EdgeType::Fallthrough,
        }
    }
}

/// One gate-check certificate in a verification result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateWire {
    /// Gate identifier.
    pub kind: GateKind,
    /// "Pass" or "Fail".
    pub status: PassFail,
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
    pub status: TaintPathStatus,
}

/// Lightweight descriptor for a single workflow node returned by GetWorkflowGraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDescriptor {
    /// Step index of this node.
    pub step_idx: u16,
    /// Kind of this node.
    pub kind: NodeKind,
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
    pub edge_type: EdgeType,
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
    /// An unknown event (for future compatibility).
    #[doc(hidden)]
    Unknown,
}

#[cfg(test)]
#[path = "payloads/tests.rs"]
mod tests;
