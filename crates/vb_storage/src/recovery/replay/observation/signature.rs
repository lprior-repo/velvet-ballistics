#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Canonical `JournalObservation` enum and top-level signature.
//!
//! The enum is the wire-format root for every observation produced by
//! `observe_journal`; the [`JournalObservationSignature`] struct
//! carries the schema version, the ordered observation list, and the
//! deterministic BLAKE3 digest over the canonical encoding of the
//! list. The signature is what the cross-run comparison pipeline
//! hashes for divergence detection.

use super::action_types::ActionObservation;
use super::ask::{AskObservation, SlotObservation};
use super::lifecycle::{
    LifecycleObservation, StepObservation, TerminalObservation, TimerObservation, WaitObservation,
};
use super::subject::DigestObservation;

/// Sentinel digest used by action-completion observations that did not
/// carry a value digest in the source event (legacy completion path).
///
/// `ActionCompletedEvent` (legacy) carries no `value_digest` field, but
/// the canonical encoding of `ActionObservation::outcome` is fixed-width
/// to keep the BLAKE3 input byte-deterministic. Using the constant makes
/// the placeholder explicit at every call site and prevents the value
/// from being silently re-defined elsewhere.
pub(crate) const LEGACY_OUTCOME_PLACEHOLDER_DIGEST: [u8; 32] = [0; 32];

/// Canonical semantic observation emitted by `observe_journal`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) enum JournalObservation {
    /// Lifecycle event observation.
    Lifecycle(LifecycleObservation),
    /// Step event observation.
    Step(StepObservation),
    /// Slot write observation.
    Slot(SlotObservation),
    /// Action event observation.
    Action(ActionObservation),
    /// Ask event observation.
    Ask(AskObservation),
    /// Wait event observation.
    Wait(WaitObservation),
    /// Timer event observation.
    Timer(TimerObservation),
    /// Terminal event observation.
    Terminal(TerminalObservation),
    /// Standalone digest observation (workflow / artifact / capability / reason).
    Digest(DigestObservation),
}

impl JournalObservation {
    /// Stable single-byte tag used during canonical encoding.
    #[must_use]
    pub(crate) const fn kind_tag(&self) -> u8 {
        match self {
            Self::Lifecycle(_) => 1,
            Self::Step(_) => 2,
            Self::Slot(_) => 3,
            Self::Action(_) => 4,
            Self::Ask(_) => 5,
            Self::Wait(_) => 6,
            Self::Timer(_) => 7,
            Self::Terminal(_) => 8,
            Self::Digest(_) => 9,
        }
    }
}

/// Top-level semantic observation signature for a journal slice.
///
/// Carries the schema version so consumers can reject signatures from
/// incompatible versions and the deterministic digest over the ordered
/// observations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct JournalObservationSignature {
    /// Schema version for the observation layout.
    pub(crate) schema_version: u16,
    /// Ordered observations (event order, never re-sorted).
    pub(crate) observations: Vec<JournalObservation>,
    /// Deterministic BLAKE3 digest over the canonical encoding of
    /// `observations`, with subject context bound into the prefix.
    pub(crate) digest: [u8; 32],
}
