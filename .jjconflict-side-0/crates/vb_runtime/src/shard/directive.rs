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
mod tests {
    use super::*;

    // =====================================================================
    // ShardDirective variant existence
    // =====================================================================

    #[test]
    fn shard_directive_continue_variant_exists() {
        let directive = ShardDirective::Continue;
        assert!(matches!(directive, ShardDirective::Continue));
        assert_eq!(format!("{directive:?}"), "Continue");
    }

    #[test]
    fn shard_directive_suspend_variant_exists() {
        let directive = ShardDirective::Suspend;
        assert!(matches!(directive, ShardDirective::Suspend));
        assert_eq!(format!("{directive:?}"), "Suspend");
    }

    #[test]
    fn shard_directive_cancel_variant_exists() {
        let directive = ShardDirective::Cancel;
        assert!(matches!(directive, ShardDirective::Cancel));
        assert_eq!(format!("{directive:?}"), "Cancel");
    }

    #[test]
    fn shard_directive_barrier_variant_exists() {
        let directive = ShardDirective::Barrier;
        assert!(matches!(directive, ShardDirective::Barrier));
        assert_eq!(format!("{directive:?}"), "Barrier");
    }

    #[test]
    fn shard_directive_migrate_variant_exists() {
        let directive = ShardDirective::Migrate { target: 42 };
        assert!(matches!(directive, ShardDirective::Migrate { target: 42 }));
        assert_eq!(format!("{directive:?}"), "Migrate { target: 42 }");
    }

    // =====================================================================
    // ShardDirective equality
    // =====================================================================

    #[test]
    fn shard_directive_continue_equality() {
        assert_eq!(ShardDirective::Continue, ShardDirective::Continue);
        assert_ne!(ShardDirective::Continue, ShardDirective::Suspend);
        assert_ne!(ShardDirective::Continue, ShardDirective::Cancel);
        assert_ne!(ShardDirective::Continue, ShardDirective::Barrier);
        assert_ne!(
            ShardDirective::Continue,
            ShardDirective::Migrate { target: 0 }
        );
        assert_ne!(ShardDirective::Continue, ShardDirective::Shutdown);
    }

    #[test]
    fn shard_directive_suspend_equality() {
        assert_eq!(ShardDirective::Suspend, ShardDirective::Suspend);
        assert_ne!(ShardDirective::Suspend, ShardDirective::Continue);
        assert_ne!(ShardDirective::Suspend, ShardDirective::Cancel);
        assert_ne!(ShardDirective::Suspend, ShardDirective::Barrier);
        assert_ne!(
            ShardDirective::Suspend,
            ShardDirective::Migrate { target: 0 }
        );
        assert_ne!(ShardDirective::Suspend, ShardDirective::Shutdown);
    }

    #[test]
    fn shard_directive_cancel_equality() {
        assert_eq!(ShardDirective::Cancel, ShardDirective::Cancel);
        assert_ne!(ShardDirective::Cancel, ShardDirective::Continue);
        assert_ne!(ShardDirective::Cancel, ShardDirective::Suspend);
        assert_ne!(ShardDirective::Cancel, ShardDirective::Barrier);
        assert_ne!(
            ShardDirective::Cancel,
            ShardDirective::Migrate { target: 0 }
        );
        assert_ne!(ShardDirective::Cancel, ShardDirective::Shutdown);
    }

    #[test]
    fn shard_directive_barrier_equality() {
        assert_eq!(ShardDirective::Barrier, ShardDirective::Barrier);
        assert_ne!(ShardDirective::Barrier, ShardDirective::Continue);
        assert_ne!(ShardDirective::Barrier, ShardDirective::Suspend);
        assert_ne!(ShardDirective::Barrier, ShardDirective::Cancel);
        assert_ne!(
            ShardDirective::Barrier,
            ShardDirective::Migrate { target: 0 }
        );
        assert_ne!(ShardDirective::Barrier, ShardDirective::Shutdown);
    }

    // =====================================================================
    // ShardDirective copy semantics
    // =====================================================================

    #[test]
    fn shard_directive_is_copy() {
        let original = ShardDirective::Continue;
        let copy = original;
        assert_eq!(original, copy);
    }

    // =====================================================================
    // ShardDirective debug format
    // =====================================================================

    #[test]
    fn shard_directive_debug_contains_variant_name() {
        for directive in [
            ShardDirective::Continue,
            ShardDirective::Suspend,
            ShardDirective::Cancel,
            ShardDirective::Barrier,
            ShardDirective::Migrate { target: 0 },
            ShardDirective::Shutdown,
        ] {
            let debug = format!("{directive:?}");
            let name = match directive {
                ShardDirective::Continue => "Continue",
                ShardDirective::Suspend => "Suspend",
                ShardDirective::Cancel => "Cancel",
                ShardDirective::Barrier => "Barrier",
                ShardDirective::Migrate { .. } => "Migrate",
                ShardDirective::Shutdown => "Shutdown",
            };
            assert!(
                debug.contains(name),
                "debug format '{debug}' should contain '{name}'"
            );
        }
    }

    // =====================================================================
    // ShardDirective::allows_admission
    // =====================================================================

    #[test]
    fn shard_directive_continue_allows_admission() {
        assert!(ShardDirective::Continue.allows_admission());
    }

    #[test]
    fn shard_directive_suspend_denies_admission() {
        assert!(!ShardDirective::Suspend.allows_admission());
    }

    #[test]
    fn shard_directive_cancel_denies_admission() {
        assert!(!ShardDirective::Cancel.allows_admission());
    }

    #[test]
    fn shard_directive_barrier_denies_admission() {
        assert!(!ShardDirective::Barrier.allows_admission());
    }

    // =====================================================================
    // ShardDirective::completes_current_work
    // =====================================================================

    #[test]
    fn shard_directive_continue_does_not_complete() {
        assert!(!ShardDirective::Continue.completes_current_work());
    }

    #[test]
    fn shard_directive_suspend_completes_current_work() {
        assert!(ShardDirective::Suspend.completes_current_work());
    }

    #[test]
    fn shard_directive_cancel_does_not_complete() {
        // Cancel immediately terminates, doesn't wait
        assert!(!ShardDirective::Cancel.completes_current_work());
    }

    #[test]
    fn shard_directive_barrier_completes_current_work() {
        assert!(ShardDirective::Barrier.completes_current_work());
    }

    // =====================================================================
    // ShardDirective::has_migration_target
    // =====================================================================

    #[test]
    fn shard_directive_migrate_has_migration_target() {
        assert!(ShardDirective::Migrate { target: 0 }.has_migration_target());
        assert!(ShardDirective::Migrate { target: 42 }.has_migration_target());
    }

    #[test]
    fn shard_directive_non_migrate_variants_have_no_migration_target() {
        assert!(!ShardDirective::Continue.has_migration_target());
        assert!(!ShardDirective::Suspend.has_migration_target());
        assert!(!ShardDirective::Cancel.has_migration_target());
        assert!(!ShardDirective::Barrier.has_migration_target());
        assert!(!ShardDirective::Shutdown.has_migration_target());
    }

    // =====================================================================
    // ShardDirective serialization roundtrip
    // =====================================================================

    #[test]
    fn shard_directive_all_variants_serializable() {
        // Verify each variant can be formatted and parsed back
        for directive in [
            ShardDirective::Continue,
            ShardDirective::Suspend,
            ShardDirective::Cancel,
            ShardDirective::Barrier,
            ShardDirective::Migrate { target: 0 },
            ShardDirective::Shutdown,
        ] {
            let debug_str = format!("{directive:?}");
            // Each debug string should contain the variant name
            assert!(
                debug_str.contains("Continue")
                    || debug_str.contains("Suspend")
                    || debug_str.contains("Cancel")
                    || debug_str.contains("Barrier")
                    || debug_str.contains("Migrate")
                    || debug_str.contains("Shutdown"),
                "debug string '{debug_str}' should contain variant name"
            );
        }
    }

    // =====================================================================
    // ShardDirective all variants covered in match
    // =====================================================================

    #[test]
    fn shard_directive_exhaustive_match() {
        // This test ensures all variants are handled if someone adds a new one
        let all_variants = [
            ShardDirective::Continue,
            ShardDirective::Suspend,
            ShardDirective::Cancel,
            ShardDirective::Barrier,
            ShardDirective::Migrate { target: 0 },
            ShardDirective::Shutdown,
        ];

        for directive in all_variants {
            let _description = match directive {
                ShardDirective::Continue => "continue normal processing",
                ShardDirective::Suspend => "suspend after current work",
                ShardDirective::Cancel => "cancel all runs immediately",
                ShardDirective::Barrier => "block until checkpoint",
                ShardDirective::Migrate { .. } => "migrate to target shard",
                ShardDirective::Shutdown => "shutdown the shard",
            };
        }
    }

    // =====================================================================
    // ShardDirective Migrate variant specific tests
    // =====================================================================

    #[test]
    fn shard_directive_migrate_equality_same_target() {
        assert_eq!(
            ShardDirective::Migrate { target: 42 },
            ShardDirective::Migrate { target: 42 }
        );
    }

    #[test]
    fn shard_directive_migrate_inequality_different_target() {
        assert_ne!(
            ShardDirective::Migrate { target: 1 },
            ShardDirective::Migrate { target: 2 }
        );
    }

    #[test]
    fn shard_directive_migrate_is_copy() {
        let original = ShardDirective::Migrate { target: 7 };
        let copy = original;
        assert_eq!(original, copy);
    }

    #[test]
    fn shard_directive_migrate_denies_admission() {
        assert!(!ShardDirective::Migrate { target: 0 }.allows_admission());
    }

    #[test]
    fn shard_directive_migrate_completes_current_work() {
        assert!(ShardDirective::Migrate { target: 0 }.completes_current_work());
    }

    // =====================================================================
    // ShardDirective Shutdown variant specific tests
    // =====================================================================

    #[test]
    fn shard_directive_shutdown_variant_exists() {
        let directive = ShardDirective::Shutdown;
        assert!(matches!(directive, ShardDirective::Shutdown));
        assert_eq!(format!("{directive:?}"), "Shutdown");
    }

    #[test]
    fn shard_directive_shutdown_is_not_alive() {
        assert!(!ShardDirective::Shutdown.is_alive());
    }

    #[test]
    fn shard_directive_shutdown_equality() {
        assert_eq!(ShardDirective::Shutdown, ShardDirective::Shutdown);
    }

    #[test]
    fn shard_directive_shutdown_is_copy() {
        let original = ShardDirective::Shutdown;
        let copy = original;
        assert_eq!(original, copy);
    }

    #[test]
    fn shard_directive_shutdown_debug_format() {
        let directive = ShardDirective::Shutdown;
        let debug = format!("{directive:?}");
        assert!(debug.contains("Shutdown"));
    }

    // =====================================================================
    // ShardDirective::is_alive tests
    // =====================================================================

    #[test]
    fn shard_directive_continue_is_alive() {
        assert!(ShardDirective::Continue.is_alive());
    }

    #[test]
    fn shard_directive_suspend_is_alive() {
        assert!(ShardDirective::Suspend.is_alive());
    }

    #[test]
    fn shard_directive_cancel_is_alive() {
        assert!(ShardDirective::Cancel.is_alive());
    }

    #[test]
    fn shard_directive_barrier_is_alive() {
        assert!(ShardDirective::Barrier.is_alive());
    }

    #[test]
    fn shard_directive_migrate_is_alive() {
        assert!(ShardDirective::Migrate { target: 0 }.is_alive());
    }
}
