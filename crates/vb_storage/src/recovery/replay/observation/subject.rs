#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Subject-classifier digest observations.
//!
//! Tags a 32-byte digest with a [`DigestSubject`] so two observations
//! that share the same byte digest are not collapsed when their
//! semantic subject differs (e.g. a workflow digest vs an action ABI
//! digest vs a slot value digest).
//!
//! The subject tag is bound into every canonical encoding via
//! `serialized_digest` (see [`super::digest`]) so a byte-digest
//! collision across subjects is impossible.

use vb_core::WorkflowDigest;

/// Current schema version for the semantic observation signature.
///
/// Bumping this constant signals to downstream consumers that the
/// observation layout changed in a way that may invalidate cached
/// digests. The constant is re-exported from [`super::types`] so the
/// public observation schema version contract is preserved.
pub(crate) const SEMANTIC_OBSERVATION_SCHEMA_VERSION: u16 = 2;

/// Subject classifier for [`DigestObservation`].
///
/// `DigestObservation` is intentionally tagged so that two observations
/// with the same byte digest are not collapsed when their semantic
/// subject differs (e.g. a workflow digest vs an action ABI digest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub(crate) enum DigestSubject {
    /// Workflow source digest emitted on `RunAccepted`.
    Workflow = 1,
    /// Compiled artifact digest emitted on `RunAdmission`.
    Artifact = 2,
    /// Action ABI digest emitted on ticket / completed-envelope actions.
    Action = 3,
    /// Slot value digest emitted on `SlotWrittenEvent`.
    Slot = 4,
    /// Capability-set digest emitted on `RunAdmission`.
    CapabilitySet = 5,
    /// Cancellation reason digest emitted on `RunCancelled`.
    CancellationReason = 6,
}

impl DigestSubject {
    /// Stable single-byte discriminant for canonical encoding.
    #[must_use]
    pub(crate) const fn tag(self) -> u8 {
        // `DigestSubject` is `#[repr(u8)]` with explicit discriminants 1..=6;
        // this is a documented byte projection, not a numeric conversion.
        #[allow(clippy::as_conversions)]
        let tag = self as u8;
        tag
    }
}

/// One semantic digest observation.
///
/// The subject tag distinguishes observations that would otherwise share the
/// same 32-byte digest (e.g. workflow vs action-ABI vs slot value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DigestObservation {
    /// Subject classifier for this digest.
    pub(crate) subject: DigestSubject,
    /// Raw 32-byte digest.
    pub(crate) bytes: [u8; 32],
}

impl DigestObservation {
    /// Build a digest observation from a `WorkflowDigest`.
    #[must_use]
    pub(crate) const fn from_workflow(subject: DigestSubject, digest: WorkflowDigest) -> Self {
        Self {
            subject,
            bytes: digest.as_bytes(),
        }
    }
}
