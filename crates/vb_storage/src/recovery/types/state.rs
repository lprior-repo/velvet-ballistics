#![forbid(unsafe_code)]
//! Recovery state types: terminal states, summaries, admission, hydration,
//! step/slot entries, frame seeds, and unsupported-state flags.

use serde::{Deserialize, Serialize};
use vb_core::{
    ActionId, CapabilitySet, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint,
    WorkflowDigest,
};

use crate::EventSeq;

// ---------------------------------------------------------------------------
// Terminal state
// ---------------------------------------------------------------------------

/// Terminal status recovered from durable journal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl RecoveryTerminalState {
    /// Returns the variant name as a static string.
    ///
    /// This is the canonical diagnostic form for terminal states in recovery
    /// errors: it names the kind of terminal event without exposing the
    /// payload, so the structural comparison (`PartialEq`) remains the
    /// authoritative source for variant-class and slot-value mismatches.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cancelled => "Cancelled",
            Self::Killed => "Killed",
            Self::Finished { .. } => "Finished",
            Self::Failed => "Failed",
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime summary
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Admission metadata
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Hydration product
// ---------------------------------------------------------------------------

/// Explicit recovery product. Supports summary-only or full live-frame seed
/// recovery from durable journal events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
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

// ---------------------------------------------------------------------------
// Step state
// ---------------------------------------------------------------------------

/// Step state recovered from durable lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
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

// ---------------------------------------------------------------------------
// Slot entries
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Pending actions
// ---------------------------------------------------------------------------

/// One pending action reconstructed from unresolved action lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecoveredPendingAction {
    /// Step that scheduled the action.
    pub step: StepIdx,
    /// Durable action identifier.
    pub action: ActionId,
}

// ---------------------------------------------------------------------------
// Unsupported state flags
// ---------------------------------------------------------------------------

/// State that durable headers/events still cannot reconstruct into a live frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedRecoveryState {
    /// Slot values are not present in current slot-written records.
    pub slot_values: bool,
    /// Slot taint is not present in current slot-written records.
    pub slot_taint: bool,
    /// Action payload/result bodies are not present in current action records.
    pub action_payloads: bool,
}

impl UnsupportedRecoveryState {
    /// Recovery state is fully supported by the runtime hydration boundary.
    pub const SUPPORTED: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
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
            ..Self::SUPPORTED
        }
    }

    /// Ticket-envelope events (scheduled-ticket or completed-envelope) carry
    /// action payload bodies that the runtime rehydration boundary cannot
    /// re-attach to a live frame, so the seed must explicitly mark these as
    /// unsupported.
    #[must_use]
    pub const fn action_payloads_unsupported() -> Self {
        Self {
            action_payloads: true,
            ..Self::SUPPORTED
        }
    }

    /// Pending actions were recovered but cannot yet be resumed by `RunFrame`.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            slot_values: self.slot_values || other.slot_values,
            slot_taint: self.slot_taint || other.slot_taint,
            action_payloads: self.action_payloads || other.action_payloads,
        }
    }

    /// Production proof surface for `SUPPORTED`: every unsupported flag is false.
    #[must_use]
    pub const fn is_fully_supported(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads
    }

    /// Production proof surface for flag-wise union correspondence.
    #[must_use]
    pub const fn union_matches_flags(self, other: Self, union: Self) -> bool {
        union.slot_values == (self.slot_values || other.slot_values)
            && union.slot_taint == (self.slot_taint || other.slot_taint)
            && union.action_payloads == (self.action_payloads || other.action_payloads)
    }
}

// ---------------------------------------------------------------------------
// Frame seed
// ---------------------------------------------------------------------------

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
    /// Exact pieces of live runtime state not represented by durable events yet.
    pub unsupported: UnsupportedRecoveryState,
}
