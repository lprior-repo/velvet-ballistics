#![forbid(unsafe_code)]
//! IPC numeric identifiers.
//!
//! These are wire-format identifiers used at the IPC boundary. They are distinct
//! from the internal domain tickets (AskTicket, ActionTicket) which carry richer
//! structural information.

use serde::{Deserialize, Serialize};

/// Ask ticket identifier from the wire protocol.
///
/// Wraps a `u64` encoding where the lower 16 bits contain a step index.
/// This is the identifier provided when answering a suspended ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct AskTicketId(u64);

impl AskTicketId {
    /// Creates an ask ticket ID from a raw wire value.
    ///
    /// # Panics
    ///
    /// Panics if the lower 16 bits exceed `u16::MAX` (which is impossible for
    /// a valid 64-bit integer, but validates the encoding invariant).
    #[must_use]
    pub const fn from_wire(raw: u64) -> Self {
        // Invariant: lower 16 bits always fit in u16 for any u64 value.
        Self(raw)
    }

    /// Returns the raw wire value.
    #[must_use]
    pub const fn wire_value(self) -> u64 {
        self.0
    }

    /// Extracts the step index from the lower 16 bits of the wire encoding.
    ///
    /// The lower 16 bits are masked before conversion, so the value always
    /// fits in `u16`; `try_from` cannot fail. We avoid the lossy `as u16`
    /// conversion per the lint policy (see bead vb-af1hu).
    #[must_use]
    pub fn step_idx(self) -> u16 {
        let masked = self.0 & 0xFFFF;
        u16::try_from(masked).expect("mask guarantees value fits in u16")
    }
}

/// Action ticket identifier from the wire protocol.
///
/// Wraps a `u64` encoding where the lower 16 bits contain a step index.
/// This is the identifier provided when completing or failing an external action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ActionTicketId(u64);

impl ActionTicketId {
    /// Creates an action ticket ID from a raw wire value.
    ///
    /// # Panics
    ///
    /// Panics if the lower 16 bits exceed `u16::MAX` (which is impossible for
    /// a valid 64-bit integer, but validates the encoding invariant).
    #[must_use]
    pub const fn from_wire(raw: u64) -> Self {
        // Invariant: lower 16 bits always fit in u16 for any u64 value.
        Self(raw)
    }

    /// Returns the raw wire value.
    #[must_use]
    pub const fn wire_value(self) -> u64 {
        self.0
    }

    /// Extracts the step index from the lower 16 bits of the wire encoding.
    ///
    /// The lower 16 bits are masked before conversion, so the value always
    /// fits in `u16`; `try_from` cannot fail. We avoid the lossy `as u16`
    /// conversion per the lint policy (see bead vb-af1hu).
    #[must_use]
    pub fn step_idx(self) -> u16 {
        let masked = self.0 & 0xFFFF;
        u16::try_from(masked).expect("mask guarantees value fits in u16")
    }
}

#[cfg(test)]
#[path = "ids/tests.rs"]
mod tests;
