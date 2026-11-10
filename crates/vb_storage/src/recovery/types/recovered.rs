#![forbid(unsafe_code)]

use vb_core::{ActionId, SlotIdx, SlotValue, StepIdx, Taint};

/// Step state recovered from durable lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredStepEntry {
    /// Step index.
    pub step: StepIdx,
    /// Durable state inferred for this step.
    pub state: RecoveredStepState,
}

/// One slot value recovered by deterministic workflow replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredSlotEntry {
    /// Slot index.
    pub slot: SlotIdx,
    /// Reconstructed slot value.
    pub value: SlotValue,
    /// Reconstructed taint marker.
    pub taint: Taint,
}

/// One pending action reconstructed from unresolved action lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredPendingAction {
    /// Step that scheduled the action.
    pub step: StepIdx,
    /// Durable action identifier.
    pub action: ActionId,
}
