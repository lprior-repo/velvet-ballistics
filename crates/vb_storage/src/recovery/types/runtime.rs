#![forbid(unsafe_code)]

use crate::EventSeq;
use vb_core::{CapabilitySet, RunId, RuntimePolicy, SlotIdx, WorkflowDigest};

/// Terminal status recovered from durable journal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryTerminalState {
    /// Run was cancelled before completion.
    Cancelled,
    /// Run was killed by the runtime.
    Killed,
    /// Run completed and selected a result slot.
    Finished {
        /// Result slot selected by the finish event.
        result: SlotIdx,
    },
    /// Run failed.
    Failed,
}

/// Runtime summary that can be recovered without reconstructing a live `RunFrame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryRuntimeSummary {
    /// Run identifier summarized by this recovery view.
    pub run: RunId,
    /// First sequence observed for the run.
    pub first_seq: EventSeq,
    /// Last sequence observed for the run.
    pub last_seq: EventSeq,
    /// Compiled workflow digest from the acceptance event, when present.
    pub workflow: Option<WorkflowDigest>,
    /// Number of step start events.
    pub steps_started: u64,
    /// Number of step success events.
    pub steps_succeeded: u64,
    /// Number of action schedule events.
    pub actions_scheduled: u64,
    /// Number of resolved action events.
    pub actions_resolved: u64,
    /// Number of boundary suspension events.
    pub suspensions: u64,
    /// Number of slot write events.
    pub slots_written: u64,
    /// Terminal status, when a terminal event exists.
    pub terminal: Option<RecoveryTerminalState>,
}

/// Admission metadata recovered from durable journal events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredRunAdmission {
    /// Canonical retained input identity, or explicit historical absence.
    pub input_digest: crate::RunInputBinding,
    /// Actual policy identity from the accepted artifact, or explicit historical absence.
    pub policy_pin: crate::RunPolicyPin,
    /// Digest of the accepted compiled artifact.
    pub artifact_digest: WorkflowDigest,
    /// Run identifier assigned at admission.
    pub run_id: RunId,
    /// Capabilities granted for this run.
    pub granted_capabilities: CapabilitySet,
    /// Admission policy that governed this admission decision.
    pub policy: RuntimePolicy,
    /// Aggregate budget admitted for this run, when the receipt carries one.
    pub budget: Option<vb_core::AggregateResourceBudget>,
}
