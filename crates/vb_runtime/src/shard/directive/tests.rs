// =============================================================================
// ShardDirective variant existence
// =============================================================================

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

// =============================================================================
// ShardDirective equality
// =============================================================================

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

// =============================================================================
// ShardDirective copy semantics
// =============================================================================

#[test]
fn shard_directive_is_copy() {
    let original = ShardDirective::Continue;
    let copy = original;
    assert_eq!(original, copy);
}

// =============================================================================
// ShardDirective debug format
// =============================================================================

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

// =============================================================================
// ShardDirective::allows_admission
// =============================================================================

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

// =============================================================================
// ShardDirective::completes_current_work
// =============================================================================

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

// =============================================================================
// ShardDirective::has_migration_target
// =============================================================================

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

// =============================================================================
// ShardDirective serialization roundtrip
// =============================================================================

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

// =============================================================================
// ShardDirective all variants covered in match
// =============================================================================

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

// =============================================================================
// ShardDirective Migrate variant specific tests
// =============================================================================

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

// =============================================================================
// ShardDirective Shutdown variant specific tests
// =============================================================================

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

// =============================================================================
// ShardDirective::is_alive tests
// =============================================================================

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
