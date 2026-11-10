#![forbid(unsafe_code)]

/// State that durable headers/events still cannot reconstruct into a live frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            slot_taint: self.slot_taint || other.slot_taint,
            action_payloads: self.action_payloads || other.action_payloads,
            pending_actions: self.pending_actions || other.pending_actions,
        }
    }

    /// Production proof surface for `SUPPORTED`: every unsupported flag is false.
    #[must_use]
    pub const fn is_fully_supported(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }

    /// Production proof surface for flag-wise union correspondence.
    #[must_use]
    pub const fn union_matches_flags(self, other: Self, union: Self) -> bool {
        union.slot_values == (self.slot_values || other.slot_values)
            && union.slot_taint == (self.slot_taint || other.slot_taint)
            && union.action_payloads == (self.action_payloads || other.action_payloads)
            && union.pending_actions == (self.pending_actions || other.pending_actions)
    }
}
