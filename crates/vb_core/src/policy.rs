#![forbid(unsafe_code)]
//! Runtime admission policy controlling verification strictness and durability.

/// Controls how strictly artifact admission verification is enforced and whether
/// [`JournalBeforeDispatch.DispatchSafety`](crate::policy::JournalBeforeDispatch::DispatchSafety)
/// is guaranteed.
///
/// # Policy and DispatchSafety
///
/// - `Strict` and `Journaled`: [`DispatchSafety`] is enforced — every dispatched action
///   is guaranteed to have a corresponding `ActionScheduled` entry in the journal before
///   dispatch occurs. The journal provides evidence that enables replay and crash recovery.
///
/// - `Relaxed`: [`DispatchSafety`] is intentionally **bypassed**. No `ActionScheduled`
///   journal entry is required before dispatch, and all journal events are silently dropped.
///   This makes the safety property vacuously true without being actually enforced.
///   Use `Relaxed` only for testing or internal workflows where recovery is not required.
///
/// # Journal Selection
///
/// When using `Runtime::new()` or `Shard::new()`, a `NoopRuntimeJournal` is used regardless
/// of the policy setting. This means **all** policies bypass journal enforcement through
/// those convenience constructors. For production or safety-critical use, construct the
/// runtime with `Runtime::new_with_journal()` and pass an appropriate journal
/// (`VolatileRuntimeJournal`, `StorageRuntimeJournal`, or `QueuedStorageRuntimeJournal`).
///
/// [`DispatchSafety`]: crate::policy::JournalBeforeDispatch::DispatchSafety
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RuntimePolicy {
    /// Require accepted artifact for all runs, SyncAll before return.
    /// DispatchSafety is enforced with a persistent journal.
    Strict,
    /// Accept runs without artifact, queue events without sync barrier.
    /// DispatchSafety is enforced with a queued or persistent journal.
    Journaled,
    /// No verification required, testing only.
    /// **DispatchSafety is bypassed — events are silently dropped.**
    /// Use only when recovery and replay are not required.
    Relaxed,
}

/// JournalBeforeDispatch safety property.
///
/// Invariant: An action is never dispatched before `ActionScheduled` is committed to the journal.
/// This is the core safety property for durable execution and enables crash recovery.
///
/// # TLA+ Specification
///
/// See `specs/tla/JournalBeforeDispatch.tla` for the formal specification and proof.
pub mod JournalBeforeDispatch {
    /// Safety property: every dispatched action has a corresponding `ActionScheduled`
    /// journal entry.
    ///
    /// # Vacuous Truth with NoopRuntimeJournal
    ///
    /// When `NoopRuntimeJournal` is used, all journal events are dropped and this property
    /// becomes **vacuously true** — there are no journal entries to violate the invariant,
    /// but the safety guarantee is not actually enforced. Only use `NoopRuntimeJournal`
    /// when you intentionally want to bypass `DispatchSafety` enforcement.
    pub const DispatchSafety: &str = "DispatchSafety";
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
