#![forbid(unsafe_code)]
//! PO-004: Kani harness verifying the production `is_supported_code`
//! predicate against the production `CODE_REGISTRY`.
//!
//! Binds to `crate::diagnostic::is_supported_code` (a `pub(crate)`
//! predicate delegating to the registry) and the production registry
//! itself. No shadow range table lives here: the previous hardcoded
//! ranges had drifted from the registry (e.g. they rejected `0x0205`,
//! `0x1315`, `0x300F`, and `0x401D`, all registered in production),
//! which is exactly the vacuum this bead removes.
//!
//! Bound: Production registry size (unwind=300); full-u16 symbolic
//! property (unwind=300).

use crate::diagnostic::{CODE_REGISTRY, is_registered_numeric, is_supported_code};

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-004 H1: Every numeric code in the production registry is
    /// accepted by the production `is_supported_code`.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_is_supported_code_all_constants() {
        for i in 0..CODE_REGISTRY.len() {
            let code = CODE_REGISTRY[i].numeric;
            assert!(
                is_supported_code(code),
                "Every registry entry's numeric code must be accepted by is_supported_code"
            );
        }
    }

    /// PO-004 H2 (binding): over all `u16` inputs, the production
    /// predicate agrees with an independent harness-side scan of the
    /// production registry. This proves the predicate's delegation to
    /// the registry rather than to a stale range table.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_is_supported_code_matches_registry() {
        let raw: u16 = kani::any();
        let mut found = false;
        for entry in CODE_REGISTRY {
            if entry.numeric == raw {
                found = true;
            }
        }
        assert_eq!(
            is_supported_code(raw),
            found,
            "is_supported_code must agree with the production registry"
        );
        // Cross-check the independent scan against the production query.
        assert_eq!(
            found,
            is_registered_numeric(raw),
            "harness scan must agree with production is_registered_numeric"
        );
    }

    /// PO-004 H3: Verified boundary anchors. Range endpoints below are
    /// confirmed registered in production; gap values confirmed absent
    /// (see bead contract). H1/H2 prove the general case.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_is_supported_code_boundary_anchors() {
        // First/last of several production ranges (all registered).
        assert!(is_supported_code(0x0101)); // Schema first
        assert!(is_supported_code(0x010B)); // Schema last
        assert!(is_supported_code(0x0201)); // Reference first
        assert!(is_supported_code(0x0205)); // Reference last (new vs shadow)
        assert!(is_supported_code(0x0301)); // Control Flow first
        assert!(is_supported_code(0x0309)); // Control Flow last
        assert!(is_supported_code(0x0401)); // Type/Taint first
        assert!(is_supported_code(0x040C)); // Type/Taint last
        assert!(is_supported_code(0x0501)); // Gate first
        assert!(is_supported_code(0x0513)); // Gate last
        assert!(is_supported_code(0x0601)); // Contract Discovery first
        assert!(is_supported_code(0x0603)); // Contract Discovery last
        assert!(is_supported_code(0x1001)); // Compilation first
        assert!(is_supported_code(0x300F)); // Runtime first (new vs shadow)
        assert!(is_supported_code(0x3022)); // Runtime last (new vs shadow)
        assert!(is_supported_code(0x3201)); // Ipc first (new vs shadow)
        assert!(is_supported_code(0x3301)); // Lifecycle sample (new vs shadow)
        assert!(is_supported_code(0x4001)); // Boundary first
        assert!(is_supported_code(0x4032)); // Boundary last (new vs shadow)

        // Verified gaps (confirmed absent from the production registry).
        assert!(!is_supported_code(0x0000)); // Zero is unsupported
        assert!(!is_supported_code(0x0100)); // Below Schema range
        assert!(!is_supported_code(0x010C)); // Above Schema range
        assert!(!is_supported_code(0x0200)); // Below Reference range
        assert!(!is_supported_code(0x0206)); // Above Reference range
        assert!(!is_supported_code(0x3000)); // Below Runtime range
        assert!(!is_supported_code(0x300E)); // Removed shadow range is a gap
        assert!(!is_supported_code(0xFFFF)); // Max u16 unsupported
    }
}
