#![forbid(unsafe_code)]
//! PO-004: Kani harness verifying is_supported_code() accepts all numeric
//! code constants defined in the production CODE_REGISTRY.
//!
//! Bound: 157 code constants to check (unwind=160)
//! Assumptions: production is_supported_code() delegates to
//! [`is_registered_numeric`] which uses `iter().find()` over CODE_REGISTRY.
//! The mirror here provides a public entrypoint for Kani harnesses since
//! the production `is_supported_code` is private.
//!
//! Rewired: uses production CODE_REGISTRY from crate::diagnostic instead of
//! inline model from kani_symbolic_code_validation.
//!
//! REPAIR-7: Mirror now delegates to `is_registered_numeric` (iter().find()
//! over CODE_REGISTRY) matching the updated production implementation.
//! Hardcoded `matches!` ranges removed to eliminate range-drift risk.

use crate::diagnostic::{CODE_REGISTRY, is_registered_numeric};

/// Public entrypoint mirroring the production `is_supported_code()`.
///
/// The production function delegates to [`is_registered_numeric`], which
/// uses `iter().find()` over [`CODE_REGISTRY`].  This mirror provides the
/// same logic so Kani harnesses can verify the registry-backed lookup.
///
/// Previously this maintained a separate hardcoded `matches!` range list
/// that had drifted from the registry (production `0x3001..=0x301B` vs
/// mirror `0x3001..=0x3022`).  Delegating to `is_registered_numeric`
/// eliminates that drift permanently.
#[must_use]
pub fn is_supported_code(code: u16) -> bool {
    is_registered_numeric(code)
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-004 H1: Every numeric code in the production registry is accepted.
    ///
    /// REPAIR-8: BLOCKED — iter().find() state explosion.
    /// This harness iterates over all 157 CODE_REGISTRY entries and calls
    /// `is_supported_code()` for each, which internally does `iter().find()`
    /// over the same 157 entries → O(157²) ≈ 24,649 symbolic paths.
    /// Exceeds practical Kani limits regardless of unwind bound.
    /// Compensating evidence: proptest PO-018 (proptest_supported_codes,
    /// 22/22 PASS) verifies all registry entries are accepted at runtime.
    /// Trusted-base ledger: TBL-VB-XI2F-R6-001 (iter().find() SSO).
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_is_supported_code_all_constants() {
        for i in 0..CODE_REGISTRY.len() {
            let code = CODE_REGISTRY[i].numeric;
            assert!(
                is_supported_code(code),
                "Every registry entry's numeric code must be accepted by is_supported_code"
            );
        }
    }

    /// Verify that explicitly excluded values are rejected.
    ///
    /// REPAIR-8: With `iter().find()` over CODE_REGISTRY (157 entries), each
    /// call must exhaust all 157 entries to prove a gap value is NOT found
    /// (no early match for gap values). Split into smaller harness groups
    /// per the REPAIR-8 required mitigation for iter().find() state-space.
    ///
    /// Sub-harness 1: low-range gaps (before and between early categories).
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_is_supported_code_rejects_gaps_1() {
        assert!(!is_supported_code(0x0000), "Zero should be unsupported");
        assert!(!is_supported_code(0x0100), "E0100 should be unsupported");
        assert!(!is_supported_code(0x010C), "E010C should be unsupported (gap)");
        assert!(!is_supported_code(0x0200), "E0200 should be unsupported");
        assert!(!is_supported_code(0x0205), "E0205 should be unsupported (gap)");
    }

    /// Sub-harness 2: mid-range gaps (between ControlFlow/TypeTaint/Gate/ContractDiscovery).
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_is_supported_code_rejects_gaps_2() {
        assert!(!is_supported_code(0x0300), "E0300 should be unsupported");
        assert!(!is_supported_code(0x030A), "E030A should be unsupported (gap)");
        assert!(!is_supported_code(0x0400), "E0400 should be unsupported");
        assert!(!is_supported_code(0x040D), "E040D should be unsupported (gap)");
        assert!(!is_supported_code(0x0500), "E0500 should be unsupported");
    }

    /// Sub-harness 3: upper-range gaps and boundary values.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_is_supported_code_rejects_gaps_3() {
        assert!(!is_supported_code(0x0600), "E0600 should be unsupported");
        assert!(!is_supported_code(0x0604), "E0604 should be unsupported (gap)");
        assert!(!is_supported_code(0x09FF), "E09FF should be unsupported (gap)");
        assert!(!is_supported_code(0x0FFF), "E0FFF should be unsupported (gap)");
        assert!(!is_supported_code(0xFFFF), "EFFFF should be unsupported");
    }

    /// Verify that representative values in each supported category range
    /// are accepted. Values are drawn from the production CODE_REGISTRY.
    ///
    /// REPAIR-8: unwind increased 30→160 because `is_supported_code()` now
    /// delegates to `is_registered_numeric()` which uses `iter().find()` over
    /// CODE_REGISTRY (157 entries). Each call requires unwinding the full
    /// iterator (157 iterations + "not found" branch).
    /// Also updated harness values to match actual CODE_REGISTRY contents
    /// (previous values like 0x1001, 0x1011, 0x1101, 0x1201, 0x1301, 0x130D,
    /// 0x1311, 0x1401, 0x1407, 0x200F, 0x3001 were not registered).
    /// Cumulative solver time for many sequential find() calls can be high;
    /// if this times out, split into per-category harnesses.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_is_supported_code_accepts_ranges() {
        // One representative code per major category block.
        assert!(is_supported_code(0x0101));  // Schema
        assert!(is_supported_code(0x0201));  // Reference
        assert!(is_supported_code(0x0301));  // ControlFlow
        assert!(is_supported_code(0x0401));  // TypeTaint
        assert!(is_supported_code(0x0501));  // Gate
        assert!(is_supported_code(0x0601));  // ContractDiscovery
        assert!(is_supported_code(0x1006));  // Compilation
        assert!(is_supported_code(0x1105));  // WorkflowIr
        assert!(is_supported_code(0x1203));  // Expression
        assert!(is_supported_code(0x1315));  // Accessor
        assert!(is_supported_code(0x2001));  // Storage
        assert!(is_supported_code(0x300F));  // Runtime
        assert!(is_supported_code(0x3201));  // IPC
        assert!(is_supported_code(0x3301));  // Lifecycle
        assert!(is_supported_code(0x4001));  // RuntimeBoundary
    }
}
