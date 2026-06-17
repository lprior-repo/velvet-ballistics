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
            kani::assert(is_supported_code(code, "assertion failed"),
                "Every registry entry's numeric code must be accepted by is_supported_code",
            );
        }
    }

    /// Verify that explicitly excluded values are rejected.
    #[kani::proof]
    #[kani::unwind(20)]
    fn kani_is_supported_code_rejects_gaps() {
        // Edge values just outside supported ranges
        kani::assert(!is_supported_code(0x0000, "assertion failed"), "Zero should be unsupported");
        kani::assert(!is_supported_code(0x0100, "assertion failed"), "E0100 should be unsupported");
        kani::assert(!is_supported_code(0x010C, "assertion failed"),
            "E010C should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x0200, "assertion failed"), "E0200 should be unsupported");
        kani::assert(!is_supported_code(0x0205, "assertion failed"),
            "E0205 should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x0300, "assertion failed"), "E0300 should be unsupported");
        kani::assert(!is_supported_code(0x030A, "assertion failed"),
            "E030A should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x0400, "assertion failed"), "E0400 should be unsupported");
        kani::assert(!is_supported_code(0x040D, "assertion failed"),
            "E040D should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x0500, "assertion failed"), "E0500 should be unsupported");
        kani::assert(!is_supported_code(0x0600, "assertion failed"), "E0600 should be unsupported");
        kani::assert(!is_supported_code(0x0604, "assertion failed"),
            "E0604 should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x09FF, "assertion failed"),
            "E09FF should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x0FFF, "assertion failed"),
            "E0FFF should be unsupported (gap)",
        );
        kani::assert(!is_supported_code(0x401D, "assertion failed"),
            "E401D should be unsupported (gap above 0x401C)",
        );
        kani::assert(!is_supported_code(0xFFFF, "assertion failed"), "EFFFF should be unsupported");
    }

    /// Verify that representative values in each supported range are accepted.
    #[kani::proof]
    #[kani::unwind(30)]
    fn kani_is_supported_code_accepts_ranges() {
        // One value from each supported range
        kani::assert(is_supported_code(0x0101, "assertion failed"), "kani harness assertion"); // Schema first
        kani::assert(is_supported_code(0x010B, "assertion failed"), "kani harness assertion"); // Schema last
        kani::assert(is_supported_code(0x0201, "assertion failed"), "kani harness assertion"); // Reference first
        kani::assert(is_supported_code(0x0204, "assertion failed"), "kani harness assertion"); // Reference last
        kani::assert(is_supported_code(0x0301, "assertion failed"), "kani harness assertion"); // Control Flow first
        kani::assert(is_supported_code(0x0309, "assertion failed"), "kani harness assertion"); // Control Flow last
        kani::assert(is_supported_code(0x0401, "assertion failed"), "kani harness assertion"); // Type/Taint first
        kani::assert(is_supported_code(0x040C, "assertion failed"), "kani harness assertion"); // Type/Taint last
        kani::assert(is_supported_code(0x0501, "assertion failed"), "kani harness assertion"); // Gate first
        kani::assert(is_supported_code(0x0513, "assertion failed"), "kani harness assertion"); // Gate last
        kani::assert(is_supported_code(0x0601, "assertion failed"), "kani harness assertion"); // Contract Discovery first
        kani::assert(is_supported_code(0x0603, "assertion failed"), "kani harness assertion"); // Contract Discovery last
        kani::assert(is_supported_code(0x1001, "assertion failed"), "kani harness assertion"); // Compilation first
        kani::assert(is_supported_code(0x1002, "assertion failed"), "kani harness assertion"); // Compilation second
        kani::assert(is_supported_code(0x1011, "assertion failed"), "kani harness assertion"); // Compilation (canonical)
        kani::assert(is_supported_code(0x1013, "assertion failed"), "kani harness assertion"); // Compilation (canonical last)
        kani::assert(is_supported_code(0x1101, "assertion failed"), "kani harness assertion"); // Workflow IR first
        kani::assert(is_supported_code(0x1104, "assertion failed"), "kani harness assertion"); // Workflow IR last
        kani::assert(is_supported_code(0x1201, "assertion failed"), "kani harness assertion"); // Expression first
        kani::assert(is_supported_code(0x1202, "assertion failed"), "kani harness assertion"); // Expression last
        kani::assert(is_supported_code(0x1301, "assertion failed"), "kani harness assertion"); // Accessor first
        kani::assert(is_supported_code(0x130D, "assertion failed"), "kani harness assertion"); // Accessor mid
        kani::assert(is_supported_code(0x1311, "assertion failed"), "kani harness assertion"); // Accessor idempotency
        kani::assert(is_supported_code(0x1314, "assertion failed"), "kani harness assertion"); // Accessor last
        kani::assert(is_supported_code(0x1401, "assertion failed"), "kani harness assertion"); // Lowering first
        kani::assert(is_supported_code(0x1407, "assertion failed"), "kani harness assertion"); // Lowering last
        kani::assert(is_supported_code(0x2001, "assertion failed"), "kani harness assertion"); // Storage first
        kani::assert(is_supported_code(0x200F, "assertion failed"), "kani harness assertion"); // Storage last
        kani::assert(is_supported_code(0x3001, "assertion failed"), "kani harness assertion"); // Runtime first
        kani::assert(is_supported_code(0x300E, "assertion failed"), "kani harness assertion"); // Runtime last
        kani::assert(is_supported_code(0x4001, "assertion failed"), "kani harness assertion"); // Boundary first
        kani::assert(is_supported_code(0x401C, "assertion failed"), "kani harness assertion") // Boundary last (extended from 0x401B)
    }
}
