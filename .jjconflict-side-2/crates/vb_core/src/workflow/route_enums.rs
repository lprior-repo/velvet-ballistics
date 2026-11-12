#![forbid(unsafe_code)]

//! Total routing enums mirroring [`NodeContinuation`](super::NodeContinuation)
//! and [`NodeOutput`](super::NodeOutput) for node-kind optional targets.
//!
//! Each enum keeps the legacy [`Option`] postcard bytes: the absent variant
//! encodes as discriminant 0 and the present variant as discriminant 1, so
//! canonical digests and artifact bytes are unchanged. Reads are total.

use crate::ids::{SlotIdx, StepIdx};
use serde::{Deserialize, Serialize};

/// Fallthrough target of a branch when no condition matches.
///
/// Mirrors [`NodeContinuation`](super::NodeContinuation): `End` is the
/// absent case, `Target` the present case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchContinuation {
    /// No fallthrough target; stepping off raises `MissingNextStep`.
    End,
    /// Fall through to this step.
    Target(StepIdx),
}

impl BranchContinuation {
    /// Fallthrough target, or `None` when the branch ends the path.
    #[must_use]
    pub const fn step(self) -> Option<StepIdx> {
        match self {
            Self::End => None,
            Self::Target(step) => Some(step),
        }
    }
}

impl From<Option<StepIdx>> for BranchContinuation {
    fn from(target: Option<StepIdx>) -> Self {
        match target {
            Some(step) => Self::Target(step),
            None => Self::End,
        }
    }
}

/// Optional deadline slot for timed waits.
///
/// Mirrors [`NodeOutput`](super::NodeOutput): `Unset` writes nothing,
/// `Slot` names the receiving slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Deadline {
    /// No deadline is configured.
    Unset,
    /// Deadline is read from this slot.
    Slot(SlotIdx),
}

impl Deadline {
    /// Deadline slot, when configured.
    #[must_use]
    pub const fn slot(self) -> Option<SlotIdx> {
        match self {
            Self::Unset => None,
            Self::Slot(slot) => Some(slot),
        }
    }
}

impl From<Option<SlotIdx>> for Deadline {
    fn from(slot: Option<SlotIdx>) -> Self {
        match slot {
            Some(slot) => Self::Slot(slot),
            None => Self::Unset,
        }
    }
}

/// Sink receiving failure information before routing.
///
/// Mirrors [`NodeOutput`](super::NodeOutput): `Discard` records nothing,
/// `Slot` names the receiving slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSink {
    /// Failure information is discarded.
    Discard,
    /// Failure information is written into this slot.
    Slot(SlotIdx),
}

impl ErrorSink {
    /// Receiving slot, when configured.
    #[must_use]
    pub const fn slot(self) -> Option<SlotIdx> {
        match self {
            Self::Discard => None,
            Self::Slot(slot) => Some(slot),
        }
    }
}

impl From<Option<SlotIdx>> for ErrorSink {
    fn from(slot: Option<SlotIdx>) -> Self {
        match slot {
            Some(slot) => Self::Slot(slot),
            None => Self::Discard,
        }
    }
}
