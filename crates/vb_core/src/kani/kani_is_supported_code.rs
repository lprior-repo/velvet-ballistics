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
            kani::assert(
                is_supported_code(code),
                "Every registry entry's numeric code must be accepted by is_supported_code",
            );
        }
    }

    /// Verify that explicitly excluded values are rejected.
    #[kani::proof]
    #[kani::unwind(20)]
    fn kani_is_supported_code_rejects_gaps() {
        // Edge values just outside supported ranges
        kani::assert(!is_supported_code(0x0000), "Zero should be unsupported");
        kani::assert(!is_supported_code(0x0100), "E0100 should be unsupported");
        kani::assert(
            !is_supported_code(0x010C),
            "E010C should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x0200), "E0200 should be unsupported");
        kani::assert(
            !is_supported_code(0x0205),
            "E0205 should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x0300), "E0300 should be unsupported");
        kani::assert(
            !is_supported_code(0x030A),
            "E030A should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x0400), "E0400 should be unsupported");
        kani::assert(
            !is_supported_code(0x040D),
            "E040D should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x0500), "E0500 should be unsupported");
        kani::assert(!is_supported_code(0x0600), "E0600 should be unsupported");
        kani::assert(
            !is_supported_code(0x0604),
            "E0604 should be unsupported (gap)",
        );
        kani::assert(
            !is_supported_code(0x09FF),
            "E09FF should be unsupported (gap)",
        );
        kani::assert(
            !is_supported_code(0x0FFF),
            "E0FFF should be unsupported (gap)",
        );
        kani::assert(
            !is_supported_code(0x401D),
            "E401D should be unsupported (gap above 0x401C)",
        );
        kani::assert(!is_supported_code(0xFFFF), "EFFFF should be unsupported");
    }

    /// Verify that representative values in each supported range are accepted.
    #[kani::proof]
    #[kani::unwind(30)]
    fn kani_is_supported_code_accepts_ranges() {
        // One value from each supported range
        kani::assert(is_supported_code(0x0101), "kani harness assertion"); // Schema first
        kani::assert(is_supported_code(0x010B), "kani harness assertion"); // Schema last
        kani::assert(is_supported_code(0x0201), "kani harness assertion"); // Reference first
        kani::assert(is_supported_code(0x0204), "kani harness assertion"); // Reference last
        kani::assert(is_supported_code(0x0301), "kani harness assertion"); // Control Flow first
        kani::assert(is_supported_code(0x0309), "kani harness assertion"); // Control Flow last
        kani::assert(is_supported_code(0x0401), "kani harness assertion"); // Type/Taint first
        kani::assert(is_supported_code(0x040C), "kani harness assertion"); // Type/Taint last
        kani::assert(is_supported_code(0x0501), "kani harness assertion"); // Gate first
        kani::assert(is_supported_code(0x0513), "kani harness assertion"); // Gate last
        kani::assert(is_supported_code(0x0601), "kani harness assertion"); // Contract Discovery first
        kani::assert(is_supported_code(0x0603), "kani harness assertion"); // Contract Discovery last
        kani::assert(is_supported_code(0x1001), "kani harness assertion"); // Compilation first
        kani::assert(is_supported_code(0x1002), "kani harness assertion"); // Compilation second
        kani::assert(is_supported_code(0x1011), "kani harness assertion"); // Compilation (canonical)
        kani::assert(is_supported_code(0x1013), "kani harness assertion"); // Compilation (canonical last)
        kani::assert(is_supported_code(0x1101), "kani harness assertion"); // Workflow IR first
        kani::assert(is_supported_code(0x1104), "kani harness assertion"); // Workflow IR last
        kani::assert(is_supported_code(0x1201), "kani harness assertion"); // Expression first
        kani::assert(is_supported_code(0x1202), "kani harness assertion"); // Expression last
        kani::assert(is_supported_code(0x1301), "kani harness assertion"); // Accessor first
        kani::assert(is_supported_code(0x130D), "kani harness assertion"); // Accessor mid
        kani::assert(is_supported_code(0x1311), "kani harness assertion"); // Accessor idempotency
        kani::assert(is_supported_code(0x1314), "kani harness assertion"); // Accessor last
        kani::assert(is_supported_code(0x1401), "kani harness assertion"); // Lowering first
        kani::assert(is_supported_code(0x1407), "kani harness assertion"); // Lowering last
        kani::assert(is_supported_code(0x2001), "kani harness assertion"); // Storage first
        kani::assert(is_supported_code(0x200F), "kani harness assertion"); // Storage last
        kani::assert(is_supported_code(0x3001), "kani harness assertion"); // Runtime first
        kani::assert(is_supported_code(0x300E), "kani harness assertion"); // Runtime last
        kani::assert(is_supported_code(0x4001), "kani harness assertion"); // Boundary first
        kani::assert(is_supported_code(0x401C), "kani harness assertion") // Boundary last (extended from 0x401B)
    }
}
