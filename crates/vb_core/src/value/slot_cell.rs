#![forbid(unsafe_code)]

use super::{SlotValue, Taint};
use serde::{Deserialize, Serialize};

/// Unified runtime slot cell pairing a value with its taint.
///
/// Before this type, frames kept parallel `slots` and `taint` arrays that
/// could drift (taint written without a value, or a value overwritten
/// without updating taint). A `SlotCell` makes that state unrepresentable:
/// an empty slot carries no taint, and a populated slot always carries
/// exactly one taint marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotCell {
    /// Slot has never been written (or was cleared).
    Empty,
    /// Slot holds a value with its taint marker.
    Value {
        /// Slot payload.
        value: SlotValue,
        /// Taint marker for `value`.
        taint: Taint,
    },
}

impl SlotCell {
    /// Empty slot constructor.
    #[must_use]
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Populated slot constructor.
    #[must_use]
    pub const fn value(value: SlotValue, taint: Taint) -> Self {
        Self::Value { value, taint }
    }

    /// Borrowed slot value, if initialized.
    #[must_use]
    pub const fn value_ref(&self) -> Option<&SlotValue> {
        match self {
            Self::Empty => None,
            Self::Value { value, .. } => Some(value),
        }
    }

    /// Slot taint, if initialized. Empty slots report `None` so callers
    /// cannot observe a taint detached from a value.
    #[must_use]
    pub const fn taint(&self) -> Option<Taint> {
        match self {
            Self::Empty => None,
            Self::Value { taint, .. } => Some(*taint),
        }
    }

    /// Whether the slot has been initialized.
    #[must_use]
    pub const fn is_initialized(&self) -> bool {
        matches!(self, Self::Value { .. })
    }
}
