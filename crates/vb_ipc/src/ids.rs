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
        // Invariant: lower 16 bits must fit in u16 (always true for u64)
        let _ = raw & 0xFFFF;
        Self(raw)
    }

    /// Returns the raw wire value.
    #[must_use]
    pub const fn wire_value(self) -> u64 {
        self.0
    }

    /// Extracts the step index from the lower 16 bits of the wire encoding.
    #[must_use]
    pub const fn step_idx(self) -> u16 {
        (self.0 & 0xFFFF) as u16
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
        // Invariant: lower 16 bits must fit in u16 (always true for u64)
        let _ = raw & 0xFFFF;
        Self(raw)
    }

    /// Returns the raw wire value.
    #[must_use]
    pub const fn wire_value(self) -> u64 {
        self.0
    }

    /// Extracts the step index from the lower 16 bits of the wire encoding.
    #[must_use]
    pub const fn step_idx(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AskTicketId tests
    // =========================================================================

    #[test]
    fn ask_ticket_id_from_wire_zero() {
        let id = AskTicketId::from_wire(0);
        assert_eq!(id.wire_value(), 0);
        assert_eq!(id.step_idx(), 0);
    }

    #[test]
    fn ask_ticket_id_from_wire_step_in_lower_bits() {
        // Wire encoding: step_idx in lower 16 bits
        let wire = 0x0000_0000_0000_0042u64; // step 66
        let id = AskTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), 66);
    }

    #[test]
    fn ask_ticket_id_from_wire_max_u16_step() {
        let wire = u16::MAX as u64;
        let id = AskTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), u16::MAX);
    }

    #[test]
    fn ask_ticket_id_wire_value_preserves_full_encoding() {
        let wire = 0xABCD_EF00_1234_5678u64;
        let id = AskTicketId::from_wire(wire);
        assert_eq!(id.wire_value(), wire);
    }

    // =========================================================================
    // ActionTicketId tests
    // =========================================================================

    #[test]
    fn action_ticket_id_from_wire_zero() {
        let id = ActionTicketId::from_wire(0);
        assert_eq!(id.wire_value(), 0);
        assert_eq!(id.step_idx(), 0);
    }

    #[test]
    fn action_ticket_id_from_wire_step_in_lower_bits() {
        let wire = 0x0000_0000_0000_0100u64; // step 256
        let id = ActionTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), 256);
    }

    #[test]
    fn action_ticket_id_from_wire_max_u16_step() {
        let wire = u16::MAX as u64;
        let id = ActionTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), u16::MAX);
    }

    #[test]
    fn action_ticket_id_wire_value_preserves_full_encoding() {
        let wire = 0x1234_5678_9ABC_DEF0u64;
        let id = ActionTicketId::from_wire(wire);
        assert_eq!(id.wire_value(), wire);
    }

    // =========================================================================
    // Type separation tests — ask vs action are distinct
    // =========================================================================

    #[test]
    fn ask_and_action_ticket_ids_are_type_distinct() {
        let ask = AskTicketId::from_wire(100);
        let action = ActionTicketId::from_wire(100);
        // Same wire value but different types — not equal
        assert_ne!(ask, action);
    }

    #[test]
    fn same_wire_value_different_types() {
        let wire = 42u64;
        let ask_id = AskTicketId::from_wire(wire);
        let action_id = ActionTicketId::from_wire(wire);
        assert_eq!(ask_id.wire_value(), action_id.wire_value());
        assert_ne!(ask_id, action_id);
    }
}
