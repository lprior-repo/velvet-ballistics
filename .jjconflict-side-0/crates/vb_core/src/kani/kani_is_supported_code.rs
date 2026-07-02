#![forbid(unsafe_code)]
//! PO-004: Kani harness verifying is_supported_code() accepts all numeric
//! code constants defined across the workspace.
//!
//! Bound: ~100 code constants to check (unwind=100)
//! Assumptions: Code constants imported from vb_core::diagnostic and
//! vb_storage::error::codes; Updated is_supported_code() ranges include
//! E05xx, E06xx, and codes above 0x401B.

use super::kani_symbolic_code_validation::{CODE_REGISTRY, is_supported_code};

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-004 H1: Every numeric code in the registry is accepted by is_supported_code().
    #[kani::proof]
    #[kani::unwind(100)]
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
    #[kani::proof]
    #[kani::unwind(20)]
    fn kani_is_supported_code_rejects_gaps() {
        // Edge values just outside supported ranges
        assert!(!is_supported_code(0x0000), "Zero should be unsupported");
        assert!(!is_supported_code(0x0100), "E0100 should be unsupported");
        assert!(
            !is_supported_code(0x010C),
            "E010C should be unsupported (gap)"
        );
        assert!(!is_supported_code(0x0200), "E0200 should be unsupported");
        assert!(
            !is_supported_code(0x0205),
            "E0205 should be unsupported (gap)"
        );
        assert!(!is_supported_code(0x0300), "E0300 should be unsupported");
        assert!(
            !is_supported_code(0x030A),
            "E030A should be unsupported (gap)"
        );
        assert!(!is_supported_code(0x0400), "E0400 should be unsupported");
        assert!(
            !is_supported_code(0x040D),
            "E040D should be unsupported (gap)"
        );
        assert!(!is_supported_code(0x0500), "E0500 should be unsupported");
        assert!(!is_supported_code(0x0600), "E0600 should be unsupported");
        assert!(
            !is_supported_code(0x0604),
            "E0604 should be unsupported (gap)"
        );
        assert!(
            !is_supported_code(0x09FF),
            "E09FF should be unsupported (gap)"
        );
        assert!(
            !is_supported_code(0x0FFF),
            "E0FFF should be unsupported (gap)"
        );
        assert!(
            !is_supported_code(0x401D),
            "E401D should be unsupported (gap above 0x401C)"
        );
        assert!(!is_supported_code(0xFFFF), "EFFFF should be unsupported");
    }

    /// Verify that representative values in each supported range are accepted.
    #[kani::proof]
    #[kani::unwind(30)]
    fn kani_is_supported_code_accepts_ranges() {
        // One value from each supported range
        assert!(is_supported_code(0x0101)); // Schema first
        assert!(is_supported_code(0x010B)); // Schema last
        assert!(is_supported_code(0x0201)); // Reference first
        assert!(is_supported_code(0x0204)); // Reference last
        assert!(is_supported_code(0x0301)); // Control Flow first
        assert!(is_supported_code(0x0309)); // Control Flow last
        assert!(is_supported_code(0x0401)); // Type/Taint first
        assert!(is_supported_code(0x040C)); // Type/Taint last
        assert!(is_supported_code(0x0501)); // Gate first
        assert!(is_supported_code(0x0513)); // Gate last
        assert!(is_supported_code(0x0601)); // Contract Discovery first
        assert!(is_supported_code(0x0603)); // Contract Discovery last
        assert!(is_supported_code(0x1001)); // Compilation first
        assert!(is_supported_code(0x1002)); // Compilation second
        assert!(is_supported_code(0x1011)); // Compilation (canonical)
        assert!(is_supported_code(0x1013)); // Compilation (canonical last)
        assert!(is_supported_code(0x1101)); // Workflow IR first
        assert!(is_supported_code(0x1104)); // Workflow IR last
        assert!(is_supported_code(0x1201)); // Expression first
        assert!(is_supported_code(0x1202)); // Expression last
        assert!(is_supported_code(0x1301)); // Accessor first
        assert!(is_supported_code(0x130D)); // Accessor mid
        assert!(is_supported_code(0x1311)); // Accessor idempotency
        assert!(is_supported_code(0x1314)); // Accessor last
        assert!(is_supported_code(0x1401)); // Lowering first
        assert!(is_supported_code(0x1407)); // Lowering last
        assert!(is_supported_code(0x2001)); // Storage first
        assert!(is_supported_code(0x200F)); // Storage last
        assert!(is_supported_code(0x3001)); // Runtime first
        assert!(is_supported_code(0x300E)); // Runtime last
        assert!(is_supported_code(0x4001)); // Boundary first
        assert!(is_supported_code(0x401C)); // Boundary last (extended from 0x401B)
    }
}
