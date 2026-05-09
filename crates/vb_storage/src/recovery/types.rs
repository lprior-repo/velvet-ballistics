//! Recovery types for velvet-ballastics journal.
//!
//! Provides:
//! - Digest mismatch detection types
//! - Recovery state types
//! - Frame seed types for live-frame reconstruction

use crate::{EventSeq, JournalError};
use serde::{Deserialize, Serialize};
use vb_core::{
    ActionId, CapabilitySet, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint,
    WorkflowDigest,
};

/// Recovery failures with typed diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    /// Journal operation failed during recovery.
    #[error("journal error during recovery: {0}")]
    Journal(#[from] JournalError),
    /// Workflow source digest does not match the stored record.
    #[error("workflow source digest mismatch: expected {expected:?}, found {found:?}")]
    WorkflowSourceDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Compiled IR digest does not match the stored record.
    #[error("compiled IR digest mismatch: expected {expected:?}, found {found:?}")]
    CompiledIrDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Action ABI digest mismatch during recovery.
    #[error("action ABI digest mismatch for action {action_id:?}")]
    ActionAbiMismatch {
        /// Action with mismatched ABI.
        action_id: ActionId,
    },
    /// Policy digest mismatch during recovery.
    #[error("policy digest mismatch for step {step:?}")]
    PolicyDigestMismatch {
        /// Step where policy diverged.
        step: StepIdx,
    },
    /// A non-idempotent action was encountered during recovery and cannot be re-executed.
    #[error(
        "non-idempotent action {action:?} at step {step:?} cannot be re-executed during recovery"
    )]
    NonIdempotentActionBlocked {
        /// Action identifier.
        action: ActionId,
        /// Step where the action was scheduled.
        step: StepIdx,
    },
    /// Replay diverged from expected state machine trajectory.
    #[error("replay divergence at step {step:?}: {detail}")]
    ReplayDivergence {
        /// Step where divergence was detected.
        step: StepIdx,
        /// Divergence description.
        detail: String,
    },
    /// No snapshot or journal events found for run.
    #[error("no recovery data found for run {run:?}")]
    NoRecoveryData {
        /// Run identifier.
        run: RunId,
    },
    /// Snapshot is present but corrupt or unreadable.
    #[error("snapshot corrupt for run {run:?} at seq {seq:?}")]
    CorruptSnapshot {
        /// Run identifier.
        run: RunId,
        /// Snapshot sequence.
        seq: EventSeq,
    },
    /// Recovery produced a terminal state that does not match expectations.
    #[error("recovery terminal state mismatch: expected {expected:?}, found {found:?}")]
    TerminalStateMismatch {
        /// Expected terminal event kind.
        expected: String,
        /// Found terminal event kind.
        found: String,
    },
    /// Durable event indexes exceed the runtime frame dimensions that can be represented.
    #[error("recovery frame dimension overflow for run {run:?}")]
    FrameDimensionOverflow {
        /// Run identifier.
        run: RunId,
    },
}

/// Result alias for recovery operations.
pub type RecoveryResult<T> = Result<T, RecoveryError>;

/// Terminal status recovered from durable journal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryTerminalState {
    /// Run was cancelled before completion.
    Cancelled,
    /// Run completed and selected a result slot.
    Finished {
        /// Result slot selected by the finish event.
        result: SlotIdx,
    },
    /// Run failed.
    Failed,
}

/// Runtime summary that can be recovered without reconstructing a live `RunFrame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredRunAdmission {
    /// Digest of the accepted compiled artifact.
    pub artifact_digest: WorkflowDigest,
    /// Run identifier assigned at admission.
    pub run_id: RunId,
    /// Capabilities granted for this run.
    pub granted_capabilities: CapabilitySet,
    /// Admission policy that governed this admission decision.
    pub policy: RuntimePolicy,
}

/// Explicit recovery product. Supports summary-only or full live-frame seed
/// recovery from durable journal events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryHydration {
    /// Summary-only recovery product.
    Summary(RecoveryRuntimeSummary),
    /// Full live-frame seed recovered from durable events.
    FrameSeed(RecoveryFrameSeed),
}

impl RecoveryHydration {
    /// Returns the summary carried by this hydration product.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        match self {
            Self::Summary(summary) => *summary,
            Self::FrameSeed(seed) => seed.summary,
        }
    }
}

/// Step state recovered from durable lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveredStepState {
    /// Step has started or is waiting on action completion.
    Running,
    /// Step completed successfully.
    Succeeded,
    /// Step failed.
    Failed,
    /// Step is suspended on a wait primitive.
    Waiting,
    /// Step is suspended on an ask primitive.
    Asking,
}

/// One recovered step-state entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredStepEntry {
    /// Step index.
    pub step: StepIdx,
    /// Durable state inferred for this step.
    pub state: RecoveredStepState,
}

/// One slot value recovered by deterministic workflow replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredSlotEntry {
    /// Slot index.
    pub slot: SlotIdx,
    /// Reconstructed slot value.
    pub value: SlotValue,
    /// Reconstructed taint marker.
    pub taint: Taint,
}

/// One pending action reconstructed from unresolved action lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredPendingAction {
    /// Step that scheduled the action.
    pub step: StepIdx,
    /// Durable action identifier.
    pub action: ActionId,
}

/// State that durable headers/events still cannot reconstruct into a live frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedRecoveryState {
    /// Slot values are not present in current slot-written records.
    pub slot_values: bool,
    /// Slot taint is not present in current slot-written records.
    pub slot_taint: bool,
    /// Action payload/result bodies are not present in current action records.
    pub action_payloads: bool,
    /// Pending action resumability cannot be projected into the runtime frame yet.
    pub pending_actions: bool,
}

impl UnsupportedRecoveryState {
    /// Recovery state is fully supported by the runtime hydration boundary.
    pub const SUPPORTED: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    };

    /// Event-only slot values have no durable taint payload.
    #[must_use]
    pub const fn event_slot_taint_unsupported() -> Self {
        Self {
            slot_taint: true,
            ..Self::SUPPORTED
        }
    }

    /// Some slot value bodies were missing or corrupt in the durable record.
    #[must_use]
    pub const fn slot_values_unsupported() -> Self {
        Self {
            slot_values: true,
            slot_taint: true,
            ..Self::SUPPORTED
        }
    }

    /// Pending actions were recovered but cannot yet be resumed by `RunFrame`.
    #[must_use]
    pub const fn pending_actions_unsupported() -> Self {
        Self {
            pending_actions: true,
            ..Self::SUPPORTED
        }
    }

    /// Combines two support descriptors without permitting contradictory states.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            slot_values: self.slot_values || other.slot_values,
            slot_taint: self.slot_taint
                || other.slot_taint
                || self.slot_values
                || other.slot_values,
            action_payloads: self.action_payloads || other.action_payloads,
            pending_actions: self.pending_actions || other.pending_actions,
        }
    }
}

/// Minimal live-frame seed recovered from durable journal headers/events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryFrameSeed {
    /// Runtime summary for the same event set.
    pub summary: RecoveryRuntimeSummary,
    /// First program-counter step for the rebuilt frame.
    pub first_step: StepIdx,
    /// Minimum step-state capacity needed for observed events.
    pub step_count: u16,
    /// Minimum slot capacity needed for observed slot/result references.
    pub slot_count: u16,
    /// Program counter inferred from the latest observed step event.
    pub pc: StepIdx,
    /// Final step states inferred from durable lifecycle events.
    pub steps: Vec<RecoveredStepEntry>,
    /// Slot values reconstructed by deterministic replay.
    pub slots: Vec<RecoveredSlotEntry>,
    /// Actions scheduled but not completed or failed at the recovery point.
    pub pending_actions: Vec<RecoveredPendingAction>,
    /// Exact pieces of live runtime state not represented by durable events yet.
    pub unsupported: UnsupportedRecoveryState,
}

/// Snapshot of a run's runtime state at a specific event sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSnapshot {
    /// Run identifier.
    pub run: RunId,
    /// Sequence number at which this snapshot was taken.
    pub seq: EventSeq,
    /// Compiled workflow digest.
    pub workflow: WorkflowDigest,
    /// Slot values at snapshot time, compact binary form.
    pub slots: Vec<u8>,
    /// Slot taint markers at snapshot time, compact binary form.
    pub taint: Vec<u8>,
}

/// Tracks which actions have been completed during recovery to prevent
/// re-execution of non-idempotent actions.
#[derive(Debug, Clone)]
pub struct ActionReplayTracker {
    completed: std::collections::HashSet<(ActionId, StepIdx)>,
    failed: std::collections::HashSet<(ActionId, StepIdx)>,
}

impl ActionReplayTracker {
    /// Creates an empty action replay tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            completed: std::collections::HashSet::new(),
            failed: std::collections::HashSet::new(),
        }
    }

    /// Records that an action was completed during normal execution.
    /// During recovery, encountering this action again will block re-execution.
    pub fn mark_completed(&mut self, action: ActionId, step: StepIdx) {
        self.completed.insert((action, step));
    }

    /// Records that an action failed during normal execution.
    pub fn mark_failed(&mut self, action: ActionId, step: StepIdx) {
        self.failed.insert((action, step));
    }

    /// Checks whether an action has already been resolved (completed or failed)
    /// and must not be re-executed during recovery.
    #[must_use]
    pub fn is_resolved(&self, action: ActionId, step: StepIdx) -> bool {
        self.completed.contains(&(action, step)) || self.failed.contains(&(action, step))
    }
}

impl Default for ActionReplayTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Digest check level for recovery validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestCheck {
    /// Only verify workflow source digest.
    WorkflowSourceOnly,
    /// Verify workflow source and compiled IR digests.
    WorkflowAndIr,
    /// Verify all digests including action ABI and policy.
    Full,
}
