#![forbid(unsafe_code)]
//! IPC command identifiers.

use serde::{Deserialize, Serialize};

use crate::error::IpcError;

/// Binary IPC command identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
#[non_exhaustive]
pub enum IpcCommand {
    /// Submit a run using a previously compiled workflow artifact.
    SubmitRun = 1,
    /// Submit a run with inline validated runtime inputs.
    SubmitRunInline = 2,
    /// Cancel an active or queued run.
    CancelRun = 3,
    /// Inspect run state.
    InspectRun = 4,
    /// List persisted events for a run.
    ListEvents = 5,
    /// Answer a suspended ask.
    AnswerAsk = 6,
    /// Complete an external action ticket.
    CompleteAction = 7,
    /// Fail an external action ticket.
    FailAction = 8,
    /// Drain bounded trace records.
    DrainTrace = 9,
    /// List active runs.
    ListRuns = 12,
    /// Request runtime metrics.
    GetMetrics = 13,
    /// Verify a compiled workflow against validation gates.
    VerifyWorkflow = 14,
    /// Get the workflow graph structure.
    GetWorkflowGraph = 15,
    /// Get taint report for a workflow.
    GetTaintReport = 16,
    /// Probe runtime health.
    Health = 17,
    /// Request graceful shutdown.
    Shutdown = 18,
}

impl IpcCommand {
    /// Parses a wire command identifier.
    pub fn from_u16(value: u16) -> Result<Self, IpcError> {
        match value {
            1 => Ok(Self::SubmitRun),
            2 => Ok(Self::SubmitRunInline),
            3 => Ok(Self::CancelRun),
            4 => Ok(Self::InspectRun),
            5 => Ok(Self::ListEvents),
            6 => Ok(Self::AnswerAsk),
            7 => Ok(Self::CompleteAction),
            8 => Ok(Self::FailAction),
            9 => Ok(Self::DrainTrace),
            12 => Ok(Self::ListRuns),
            13 => Ok(Self::GetMetrics),
            14 => Ok(Self::VerifyWorkflow),
            15 => Ok(Self::GetWorkflowGraph),
            16 => Ok(Self::GetTaintReport),
            17 => Ok(Self::Health),
            18 => Ok(Self::Shutdown),
            other => Err(IpcError::UnknownCommand(other)),
        }
    }

    /// Returns the wire command identifier.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        match self {
            Self::SubmitRun => 1,
            Self::SubmitRunInline => 2,
            Self::CancelRun => 3,
            Self::InspectRun => 4,
            Self::ListEvents => 5,
            Self::AnswerAsk => 6,
            Self::CompleteAction => 7,
            Self::FailAction => 8,
            Self::DrainTrace => 9,
            Self::ListRuns => 12,
            Self::GetMetrics => 13,
            Self::VerifyWorkflow => 14,
            Self::GetWorkflowGraph => 15,
            Self::GetTaintReport => 16,
            Self::Health => 17,
            Self::Shutdown => 18,
        }
    }
}
