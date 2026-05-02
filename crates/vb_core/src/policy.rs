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
