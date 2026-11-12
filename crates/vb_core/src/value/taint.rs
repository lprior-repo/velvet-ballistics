#![forbid(unsafe_code)]

pub use super::slot_cell::SlotCell;
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
