#![forbid(unsafe_code)]

use super::slot::SlotValue;
use serde::{Deserialize, Serialize};

/// Secret propagation marker attached to each runtime slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum Taint {
    /// Slot contains no secret-derived data.
    Clean = 0,
    /// Slot contains data derived from one or more secrets.
    DerivedFromSecret = 1,
    /// Slot contains a secret value.
    Secret = 2,
    /// Slot contains a randomly generated value.
    Random = 3,
    /// Slot contains a time-dependent value.
    TimeDependent = 4,
}

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

/// Joins two taint levels, returning the more restrictive one.
#[must_use]
pub fn join_taint(a: Taint, b: Taint) -> Taint {
    let a_disc: u8 = match a {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
        Taint::Random => 3,
        Taint::TimeDependent => 4,
    };
    let b_disc: u8 = match b {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
        Taint::Random => 3,
        Taint::TimeDependent => 4,
    };
    if a_disc >= b_disc { a } else { b }
}
