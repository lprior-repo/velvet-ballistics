//! Runtime admission policy controlling verification strictness and durability.

use super::ids::WorkflowDigest;
/// Controls how strictly artifact admission verification is enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum RuntimePolicy {
    /// Require accepted artifact for all runs, SyncAll before return.
    Strict,
    /// Accept runs without artifact, queue events without sync barrier.
    Journaled,
    /// No verification required, testing only.
    Relaxed,
}
/// Policy digest identifying a specific policy configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PolicyDigest(pub WorkflowDigest);

/// Typed outcome of a policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DecisionCode {
    /// The action was allowed to proceed.
    Allowed,
    /// The action was denied by the policy.
    Denied,
    /// The action was suspended pending external review.
    Suspended,
}

/// Which policy layer produced the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PolicyLayer {
    /// Pinned at run admission.
    Admission,
    /// Used for replay reconstruction.
    Replay,
    /// Evaluated at effect dispatch.
    LiveEffect,
    /// Monotonic emergency deny overlay.
    EmergencyDeny,
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
