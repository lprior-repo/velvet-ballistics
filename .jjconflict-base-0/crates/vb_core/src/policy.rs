#![forbid(unsafe_code)]
//! Typed policy foundation: artifact digests, live authority, decision records.
//!
//! Core policy types shared by the evaluator and durable storage layers.
//! Pure value types with no runtime state or execution logic.
//!
//! `RuntimePolicy` (Strict/Journaled/Relaxed) is preserved for durability
//! semantics; this module adds live authority and decision record types.

use crate::action::ActionTicket;
use crate::ids::ActionId;
use crate::value::Taint;
use serde::{Deserialize, Serialize};

/// 32-byte policy artifact digest. Callers supply the actual digest of a
/// compiled policy artifact; never fabricated or default-filled with zeros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PolicyDigest([u8; 32]);

impl PolicyDigest {
    /// Creates a digest from verified artifact bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Outcome of a policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DecisionCode {
    /// Action is authorized to proceed.
    Allowed = 0,
    /// Action is blocked by policy.
    Denied = 1,
    /// Action is deferred (suspension boundary).
    Suspended = 2,
}

/// Layer at which a policy decision was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PolicyLayer {
    /// Decision from the current live policy.
    Live = 0,
    /// Decision from the pinned admission policy.
    Admission = 1,
    /// Decision forced by the emergency overlay.
    Emergency = 2,
}

/// Pinned admission policy identity established at artifact admission.
/// `Unverified` is never a fabricated zero digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinnedAdmissionPolicy {
    /// Verified artifact pin by digest.
    Artifact(PolicyDigest),
    /// Explicitly unverified (not a fabricated digest).
    Unverified,
}

/// Canonical byte encoding for Taint.
/// Exhaustively matches all known Taint variants.
#[must_use]
pub fn encode_taint_canonical(taint: Taint) -> Option<u8> {
    match taint {
        Taint::Clean => Some(0),
        Taint::DerivedFromSecret => Some(1),
        Taint::Secret => Some(2),
        Taint::Random => Some(3),
        Taint::TimeDependent => Some(4),
    }
}

/// Bounded policy rule: one action maps to one decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Action this rule applies to.
    pub action: ActionId,
    /// Decision for this action.
    pub code: DecisionCode,
}

/// Current live policy configuration.
/// `Unconfigured` is explicit default-deny; never substituted with zero digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurrentLivePolicy {
    /// No policy configured; default deny all actions.
    Unconfigured,
    /// Policy configured with a verified artifact and a single rule.
    Configured {
        /// Verified artifact digest.
        digest: PolicyDigest,
        /// The policy rule for this configuration.
        rule: PolicyRule,
    },
}

impl CurrentLivePolicy {
    /// Returns the configured digest, or `None` if unconfigured.
    #[must_use]
    pub fn digest(&self) -> Option<PolicyDigest> {
        match self {
            Self::Unconfigured => None,
            Self::Configured { digest, .. } => Some(*digest),
        }
    }

    /// Returns the rule, or `None` if unconfigured.
    #[must_use]
    pub fn rule(&self) -> Option<PolicyRule> {
        match self {
            Self::Unconfigured => None,
            Self::Configured { rule, .. } => Some(*rule),
        }
    }

    /// Returns the policy identity (digest only, no rule).
    #[must_use]
    pub fn identity(&self) -> CurrentPolicyIdentity {
        match self {
            Self::Unconfigured => CurrentPolicyIdentity::Unconfigured,
            Self::Configured { digest, .. } => {
                CurrentPolicyIdentity::Configured { digest: *digest }
            }
        }
    }
}

/// Current policy identity for decision records.
/// Stores only digest (or unconfigured), never the full rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurrentPolicyIdentity {
    /// No policy configured; default deny all actions.
    Unconfigured,
    /// Policy configured with a verified artifact.
    Configured {
        /// Verified artifact digest.
        digest: PolicyDigest,
    },
}

/// Influence state for emergency or compatibility policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfluenceState {
    /// Influence was not evaluated (decision at another layer).
    NotEvaluated,
    /// Influence was active but did not change the outcome.
    Inactive,
    /// Influence was active and determined the outcome.
    Active,
}

/// Immutable record of a single policy decision.
/// Retains full identity of the evaluated ticket, policy state, and influence
/// factors. Cannot mint new effect authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// The evaluated action ticket (full 8-field identity).
    pub ticket: ActionTicket,
    /// Pinned admission policy at time of evaluation.
    pub admission: PinnedAdmissionPolicy,
    /// Current live policy identity at time of evaluation.
    pub current_live: CurrentPolicyIdentity,
    /// Deterministic digest of canonical input bytes, taint, and all ticket
    /// fields with domain separation.
    pub input_digest: PolicyDigest,
    /// Taint level of the evaluated input.
    pub taint: Taint,
    /// Final decision outcome.
    pub outcome: DecisionCode,
    /// Layer at which the decision was made.
    pub layer: PolicyLayer,
    /// Emergency overlay influence state.
    pub emergency_influence: InfluenceState,
    /// Compatibility policy influence state.
    pub compat_influence: InfluenceState,
}

/// Durability policy for runtime event persistence.
///
/// Preserved from original policy semantics; controls verification
/// strictness and sync behavior but is not involved in live effect
/// authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RuntimePolicy {
    /// Require accepted artifact for all runs, sync before return.
    Strict,
    /// Accept runs without artifact, queue events without sync barrier.
    Journaled,
    /// No verification required, testing only.
    Relaxed,
}

#[cfg(test)]
mod tests {
    use super::RuntimePolicy;

    #[test]
    fn policy_variants_are_distinct() {
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Journaled);
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Relaxed);
        assert_ne!(RuntimePolicy::Journaled, RuntimePolicy::Relaxed);
    }

    #[test]
    fn policy_copy_semantics_preserve_equality() {
        let a = RuntimePolicy::Strict;
        let b = a;
        assert_eq!(a, b, "copy must preserve equality");
    }

    #[test]
    fn policy_strict_is_not_journaled() {
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Journaled);
    }

    #[test]
    fn policy_strict_is_not_relaxed() {
        assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Relaxed);
    }

    #[test]
    fn policy_journaled_is_not_relaxed() {
        assert_ne!(RuntimePolicy::Journaled, RuntimePolicy::Relaxed);
    }

    #[test]
    fn policy_debug_output_contains_variant_name() {
        let formatted = format!("{:?}", RuntimePolicy::Strict);
        assert!(
            formatted.contains("Strict"),
            "debug output must contain variant name: {formatted}"
        );
    }

    #[test]
    fn policy_clone_produces_equal_value() {
        let original = RuntimePolicy::Journaled;
        let cloned = original.clone();
        assert_eq!(original, cloned, "clone must produce equal value");
    }
}

#[cfg(test)]
#[path = "policy/new_tests.rs"]
mod new_tests;
