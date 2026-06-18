//! Delta data structures for comparing before/after state.
//!
//! Each struct captures a single delta observation between two snapshots.

use vb_core::frame::StepState;
use vb_core::ids::StepIdx;
use vb_core::value::{SlotValue, Taint};

/// A delta for a single slot's value between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlotDelta {
    /// Index of the changed slot.
    pub(crate) slot: u16,
    /// Slot value before the change (may be `None` if slot was empty).
    pub(crate) before: Option<SlotValue>,
    /// Slot value after the change (may be `None` if slot became empty).
    pub(crate) after: Option<SlotValue>,
}

/// A delta for a single slot's taint between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaintDelta {
    /// Index of the changed slot.
    pub(crate) slot: u16,
    /// Taint before the change.
    pub(crate) before: Taint,
    /// Taint after the change.
    pub(crate) after: Taint,
}

/// A delta for a single step's state between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateDelta {
    /// Index of the changed step.
    pub(crate) step: u16,
    /// Step state before the change.
    pub(crate) before: StepState,
    /// Step state after the change.
    pub(crate) after: StepState,
}

/// A delta for the program counter (step index) between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PcDelta {
    /// Program counter before the change.
    pub(crate) before: StepIdx,
    /// Program counter after the change.
    pub(crate) after: StepIdx,
}
