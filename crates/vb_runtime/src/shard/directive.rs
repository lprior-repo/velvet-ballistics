#![forbid(unsafe_code)]
//! Shard directive types for runtime tick control.
//!
//! `ShardDirective` is the control token passed to `Runtime::tick_shard` to direct
//! a shard's behavior for one tick. Each variant encodes an operational directive
//! that the shard must process before returning control.

/// Directive issued to a shard for a single tick.
///
/// These directives are consumed by `Runtime::tick_shard` and determine what
/// work the shard performs. The shard processes directives in priority order:
/// Shutdown > Migrate > Suspend > Barrier > Continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShardDirective {
    /// Continue normal processing.
    ///
    /// The shard will process any pending commands and drive active runs
    /// up to its tick budget. This is the default directive for healthy shards.
    Continue,

    /// Suspend the shard after current work completes.
    ///
    /// The shard finishes its current tick (processing commands and driving runs)
    /// but does not accept new runs afterward. Existing runs continue to
    /// completion or suspension.
    Suspend,

    /// Cancel all runs on this shard immediately.
    ///
    /// All active runs are cancelled and removed from the shard. No further
    /// execution occurs. The shard transitions to a cancelled state.
    Cancel,

    /// Block until all active runs reach a safe checkpoint.
    ///
    /// Barrier blocks the shard until all admitted runs have either:
    /// - Reached a suspension point (awaiting external action/timer)
    /// - Completed naturally
    ///
    /// Barrier is used to coordinate cross-shard operations that require
    /// a consistent snapshot of shard state. Unlike Cancel, Barrier waits
    /// for runs to reach safe points rather than killing them immediately.
    Barrier,

    /// Migrate all pending commands to the target shard.
    ///
    /// All commands in the source shard's queue are transferred to the target
    /// shard. The source shard's queue becomes empty. Used for load balancing
    /// and shard relocation during runtime reconfiguration.
    Migrate {
        /// Target shard index to migrate commands to.
        target: u32,
    },

    /// Drain all remaining commands and shut down the shard.
    ///
    /// The shard processes all queued commands to completion, then transitions
    /// to a shut-down state. Returns `Ok(false)` to indicate the shard is dead.
    Shutdown,
}

impl ShardDirective {
    /// Returns true if this directive allows new runs to be admitted.
    ///
    /// - `Continue`: Yes, new runs may be admitted.
    /// - `Suspend`: No, existing runs complete but no new runs are admitted.
    /// - `Cancel`: No, all runs are cancelled.
    /// - `Barrier`: No, the shard is blocked on existing runs only.
    /// - `Migrate`: No, commands are being migrated away.
    /// - `Shutdown`: No, the shard is shutting down.
    #[must_use]
    pub fn allows_admission(&self) -> bool {
        matches!(self, Self::Continue)
    }

    /// Returns true if this directive completes current work before stopping.
    ///
    /// - `Continue`: Does not stop.
    /// - `Suspend`: Completes current tick then stops accepting new work.
    /// - `Cancel`: Immediately cancels all runs.
    /// - `Barrier`: Waits for all runs to reach safe points.
    /// - `Migrate`: Processes remaining commands before migrating.
    /// - `Shutdown`: Processes remaining commands then stops.
    #[must_use]
    pub fn completes_current_work(&self) -> bool {
        matches!(self, Self::Suspend | Self::Barrier | Self::Migrate { .. })
    }

    /// Returns true if this directive requires explicit migration target.
    ///
    /// Only `Migrate` carries a target. Other directives return `false`.
    #[must_use]
    pub fn has_migration_target(&self) -> bool {
        matches!(self, Self::Migrate { .. })
    }

    /// Returns `true` if this directive allows the shard to continue processing.
    ///
    /// `Shutdown` returns `false` because the shard is dead after shutdown.
    /// All other directives return `true`.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        !matches!(self, Self::Shutdown)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
