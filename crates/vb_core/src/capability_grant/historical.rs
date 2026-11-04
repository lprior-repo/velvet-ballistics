#![forbid(unsafe_code)]
//! Historical grant records retained for wire compatibility.
//!
//! These values and structural predicates do not issue authority or convert to
//! the full-ticket scoped grants used by live execution.

use super::{GrantId, RevocationEpoch};
use crate::capability::Capability;
use crate::ids::{ActionId, RunId, WorkflowDigest};
use crate::manual_gate::GateDigest;
use serde::{Deserialize, Serialize};

/// Maximum identity or resource-pattern length in bytes.
pub const MAX_GRANT_TEXT_LEN: usize = 128;
/// Maximum capabilities in a structurally valid historical record.
pub const MAX_GRANT_CAPABILITIES: usize = 256;
/// Maximum resource patterns in a structurally valid historical record.
pub const MAX_GRANT_RESOURCES: usize = 256;
/// Maximum variable signature length in bytes.
pub const MAX_GRANT_SIGNATURE_LEN: usize = 1024;

/// Retained digest bytes, without an authority or verification claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct GrantDigest([u8; 32]);

impl GrantDigest {
    /// Wraps caller-supplied digest bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the retained digest bytes.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Historical issuer or subject identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantIdentity {
    /// Retained identity text.
    pub value: String,
}

impl GrantIdentity {
    /// Creates a nonempty, bounded identity value.
    #[must_use]
    pub fn new(value: String) -> Option<Self> {
        if value.is_empty() || value.len() > MAX_GRANT_TEXT_LEN {
            None
        } else {
            Some(Self { value })
        }
    }

    /// Returns the retained identity.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Checks the historical identity bounds.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.value.is_empty() && self.value.len() <= MAX_GRANT_TEXT_LEN
    }
}

/// Variable-length historical signature, separate from the scoped signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalGrantSignature {
    /// Retained opaque signature bytes.
    pub bytes: Box<[u8]>,
}

impl HistoricalGrantSignature {
    /// Creates a nonempty, bounded signature record without verifying it.
    #[must_use]
    pub fn new(bytes: Box<[u8]>) -> Option<Self> {
        if bytes.is_empty() || bytes.len() > MAX_GRANT_SIGNATURE_LEN {
            None
        } else {
            Some(Self { bytes })
        }
    }

    /// Checks only the historical signature length bounds.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.bytes.is_empty() && self.bytes.len() <= MAX_GRANT_SIGNATURE_LEN
    }
}

/// Historical exact-match resource patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceScope {
    /// Retained resource patterns.
    pub patterns: Box<[Box<str>]>,
}

impl ResourceScope {
    /// Checks exact membership in a bounded historical collection.
    #[must_use]
    pub fn covers(&self, resource: &str) -> bool {
        self.patterns.len() <= MAX_GRANT_RESOURCES
            && self
                .patterns
                .iter()
                .any(|pattern| pattern.as_ref() == resource)
    }

    /// Checks historical bounds; an empty resource collection remains valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.patterns.len() <= MAX_GRANT_RESOURCES
            && self
                .patterns
                .iter()
                .all(|pattern| !pattern.is_empty() && pattern.len() <= MAX_GRANT_TEXT_LEN)
    }
}

/// Historical grant data in its original fifteen-field serialization order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    /// Retained grant identifier.
    pub id: GrantId,
    /// Retained issuer.
    pub issuer: GrantIdentity,
    /// Retained subject.
    pub subject: GrantIdentity,
    /// Artifact digest named by the historical record.
    pub artifact_digest: WorkflowDigest,
    /// Run named by the historical record.
    pub run: RunId,
    /// Action named by the historical record.
    pub action: ActionId,
    /// Retained capability collection.
    pub capabilities: Box<[Capability]>,
    /// Retained resource patterns.
    pub resources: ResourceScope,
    /// Recorded issue time in UNIX milliseconds.
    pub issued_at_ms: u64,
    /// Recorded exclusive expiry in UNIX milliseconds.
    pub expires_at_ms: u64,
    /// Recorded delegation marker.
    pub delegable: bool,
    /// Recorded revocation epoch.
    pub revocation_epoch: RevocationEpoch,
    /// Recorded break-glass marker.
    pub break_glass: bool,
    /// Recorded receipt digest, when present.
    pub approval_receipt_digest: Option<GateDigest>,
    /// Recorded variable-length signature, when present.
    pub signature: Option<HistoricalGrantSignature>,
}

impl CapabilityGrant {
    /// Checks exact membership without authorizing a live dispatch.
    #[must_use]
    pub fn grants(&self, capability: &Capability) -> bool {
        self.capabilities.len() <= MAX_GRANT_CAPABILITIES
            && self.capabilities.iter().any(|granted| {
                granted.name() == capability.name() && granted.action_id() == capability.action_id()
            })
    }

    /// Checks retained structural constraints, not decoding allocation or trust.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issuer.is_valid()
            && self.subject.is_valid()
            && !self.capabilities.is_empty()
            && self.capabilities.len() <= MAX_GRANT_CAPABILITIES
            && self
                .capabilities
                .iter()
                .all(|capability| !capability.name().is_empty())
            && self.resources.is_valid()
            && self
                .signature
                .as_ref()
                .is_none_or(HistoricalGrantSignature::is_valid)
            && self.issued_at_ms < self.expires_at_ms
    }
}

/// Historical token data with no live scoped-grant conversion or factory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrowedToken {
    /// Recorded grant identifier.
    pub grant: GrantId,
    /// Recorded run identifier.
    pub run: RunId,
    /// Recorded action identifier.
    pub action: ActionId,
    /// Recorded single capability.
    pub capability: Capability,
    /// Recorded exclusive expiry in UNIX milliseconds.
    pub expires_at_ms: u64,
    /// Recorded break-glass marker.
    pub break_glass: bool,
}

#[cfg(test)]
#[path = "historical_tests.rs"]
mod tests;
