#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Action-event observation types.
//!
//! Defines the per-event action projection carried by
//! [`ActionObservation`]. The action dispatch logic that maps a
//! [`crate::JournalEvent`] into an [`ActionObservation`] lives in
//! [`super::action`]; this module is types only so the dispatch can
//! stay free of `JournalEvent` and the observation struct is reusable
//! from other code paths (cross-run diff tooling, etc.).

use vb_core::{ActionId, StepIdx};

use super::subject::DigestObservation;

/// Action-state discriminant for [`ActionObservation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub(crate) enum ActionStateObservation {
    /// Action was scheduled.
    Scheduled = 1,
    /// Action completed with the outcome below.
    Completed = 2,
    /// Action failed at this attempt.
    Failed = 3,
    /// Action was abandoned because the run terminated mid-flight.
    Abandoned = 4,
}

impl ActionStateObservation {
    /// Stable single-byte tag for canonical encoding.
    #[must_use]
    pub(crate) const fn tag(self) -> u8 {
        // `ActionStateObservation` is `#[repr(u8)]` with explicit
        // discriminants 1..=4; this is a documented byte projection, not
        // a numeric conversion.
        #[allow(clippy::as_conversions)]
        let tag = self as u8;
        tag
    }
}

/// Action-completion outcome reduced to semantic fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum ActionOutcomeObservation {
    /// Action wrote a ready output value with this taint and digest.
    Ready {
        /// Taint discriminant (matches `vb_core::Taint` `repr(u8)`).
        taint_tag: u8,
        /// BLAKE3 digest of the encoded output value bytes.
        value_digest: [u8; 32],
    },
}

/// One action observation.
///
/// `action_abi_digest` is preserved only when the source event carries it
/// (currently `ActionScheduledTicket` / `ActionCompletedEnvelope`).
/// `capacity` is preserved only for `ActionAbandoned`. `outcome` is
/// preserved only for completion events.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ActionObservation {
    /// Owning step.
    pub(crate) step: StepIdx,
    /// Action identifier.
    pub(crate) action: ActionId,
    /// Attempt number.
    pub(crate) attempt: u16,
    /// Action state discriminant.
    pub(crate) state: ActionStateObservation,
    /// Action ABI digest, preserved when present in the source event.
    pub(crate) action_abi_digest: Option<DigestObservation>,
    /// Action capacity from the abandoned ticket, preserved when present.
    pub(crate) capacity: Option<u16>,
    /// Completion outcome, preserved when present in the source event.
    pub(crate) outcome: Option<ActionOutcomeObservation>,
}

impl ActionObservation {
    /// True when the source event carried an action ABI digest.
    #[must_use]
    pub(crate) const fn has_action_abi_digest(&self) -> bool {
        self.action_abi_digest.is_some()
    }

    /// True when the source event carried a capacity field.
    #[must_use]
    pub(crate) const fn has_capacity(&self) -> bool {
        self.capacity.is_some()
    }
}
