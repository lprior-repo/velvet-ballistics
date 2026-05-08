//! Runtime admission policy controlling verification strictness and durability.

/// Controls how strictly artifact admission verification is enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePolicy {
    /// Require accepted artifact for all runs, SyncAll before return.
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
        let variants = [
            RuntimePolicy::Strict,
            RuntimePolicy::Journaled,
            RuntimePolicy::Relaxed,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b, "same index must be equal");
                } else {
                    assert_ne!(a, b, "different indices must be distinct");
                }
            }
        }
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
