//! Action table rows, contract specs, and CLI action registration.
//!
//! Structure:
//! - `types` — struct and newtype definitions
//! - `generation` — static data, contract builders, CLI output, enum mappers

mod generation;
mod types;

pub(crate) use generation::*;
pub(crate) use types::*;

// ── Tests (live here so `action_retry_safety_name` is importable via `super`) ─

#[cfg(test)]
#[allow(clippy::doc_markdown)]
mod tests {
    use super::action_retry_safety_name;
    use vb_core::action::RetrySafety;

    /// Tier 2: `action_retry_safety_name(RetrySafety::Idempotent) == "idempotent"`.
    /// On 3-variant code: returns "safe"; on 4-variant code: returns "idempotent".
    #[test]
    fn action_retry_safety_name_idempotent_returns_idempotent_literal() {
        let s = action_retry_safety_name(RetrySafety::Idempotent);
        assert_eq!(s, "idempotent");
    }

    /// Tier 2: `action_retry_safety_name(RetrySafety::RequiresIdempotencyKey) == "requires_idempotency_key"`.
    /// On 3-variant code: returns "key_required"; on 4-variant code: returns "requires_idempotency_key".
    #[test]
    fn action_retry_safety_name_requires_key_returns_requires_idempotency_key_literal() {
        let s = action_retry_safety_name(RetrySafety::RequiresIdempotencyKey);
        assert_eq!(s, "requires_idempotency_key");
    }

    /// Tier 2: `action_retry_safety_name(RetrySafety::NotRetrySafe) == "not_retry_safe"`.
    /// On 3-variant code: returns "unsafe"; on 4-variant code: returns "not_retry_safe".
    #[test]
    fn action_retry_safety_name_not_retry_safe_returns_not_retry_safe_literal() {
        let s = action_retry_safety_name(RetrySafety::NotRetrySafe);
        assert_eq!(s, "not_retry_safe");
    }

    /// Tier 1: `action_retry_safety_name(RetrySafety::Unknown) == "unknown"`.
    /// On 3-variant code: `RetrySafety::Unknown` does not exist; compile fail.
    #[test]
    fn action_retry_safety_name_unknown_returns_unknown_literal() {
        let s = action_retry_safety_name(RetrySafety::Unknown);
        assert_eq!(s, "unknown");
    }

    /// Tier 1: the 4 strings are pairwise distinct (set size = 4).
    #[test]
    fn action_retry_safety_name_4_strings_pairwise_distinct() {
        let s_idempotent = action_retry_safety_name(RetrySafety::Idempotent);
        let s_requires_key = action_retry_safety_name(RetrySafety::RequiresIdempotencyKey);
        let s_not_retry_safe = action_retry_safety_name(RetrySafety::NotRetrySafe);
        let s_unknown = action_retry_safety_name(RetrySafety::Unknown);
        let set: std::collections::BTreeSet<&str> =
            [s_idempotent, s_requires_key, s_not_retry_safe, s_unknown]
                .into_iter()
                .collect();
        assert_eq!(
            set.len(),
            4,
            "4 strings must be pairwise distinct; got set {set:?}"
        );
    }
}
