#![forbid(unsafe_code)]
//! PO-011: Kani harness verifying CODE_REGISTRY category/numeric consistency.
//!
//! Proves: For each production CodeEntry, (numeric >> 8) & 0xFF matches the
//! expected high-byte range for its CodeCategory.
//!
//! Bound: 157 entries (unwind=160)
//!
//! Rewired: uses production CodeCategory and CODE_REGISTRY from
//! crate::diagnostic instead of inline models from kani_symbolic_code_validation.

use crate::diagnostic::{CODE_REGISTRY, CodeCategory};

/// Expected high byte for each category.
const fn expected_high_byte(cat: CodeCategory) -> u16 {
    match cat {
        CodeCategory::Schema => 0x01,
        CodeCategory::Reference => 0x02,
        CodeCategory::ControlFlow => 0x03,
        CodeCategory::TypeTaint => 0x04,
        CodeCategory::Gate => 0x05,
        CodeCategory::ContractDiscovery => 0x06,
        CodeCategory::Compilation => 0x10,
        CodeCategory::WorkflowIr => 0x11,
        CodeCategory::Expression => 0x12,
        CodeCategory::Accessor => 0x13,
        CodeCategory::Lowering => 0x14,
        CodeCategory::Storage => 0x20,
        CodeCategory::Runtime => 0x30,
        CodeCategory::Ipc => 0x32,
        CodeCategory::Lifecycle => 0x33,
        CodeCategory::RuntimeBoundary => 0x40,
        CodeCategory::Internal => 0x13,
    }
}

/// Category name for diagnostics.
const fn category_name(cat: CodeCategory) -> &'static str {
    match cat {
        CodeCategory::Schema => "Schema",
        CodeCategory::Reference => "Reference",
        CodeCategory::ControlFlow => "ControlFlow",
        CodeCategory::TypeTaint => "TypeTaint",
        CodeCategory::Gate => "Gate",
        CodeCategory::ContractDiscovery => "ContractDiscovery",
        CodeCategory::Compilation => "Compilation",
        CodeCategory::WorkflowIr => "WorkflowIr",
        CodeCategory::Expression => "Expression",
        CodeCategory::Accessor => "Accessor",
        CodeCategory::Lowering => "Lowering",
        CodeCategory::Storage => "Storage",
        CodeCategory::Runtime => "Runtime",
        CodeCategory::Ipc => "Ipc",
        CodeCategory::Lifecycle => "Lifecycle",
        CodeCategory::RuntimeBoundary => "RuntimeBoundary",
        CodeCategory::Internal => "Internal",
    }
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-011: For each CodeEntry, the numeric high byte matches the expected
    /// range for its CodeCategory.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_registry_category_match() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let high_byte = (entry.numeric >> 8) & 0xFF;
            let expected = expected_high_byte(entry.category);
            assert_eq!(
                high_byte,
                expected,
                "Entry '{}' (code {:04X}, category {:?}): high byte {:02X} != expected {:02X}",
                entry.symbolic,
                entry.numeric,
                category_name(entry.category),
                high_byte,
                expected
            );
        }
    }

    /// Additional: verify that low byte is never zero for all entries
    /// (ensures all codes have valid low bytes — zeros reserved for sentinel).
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_registry_schema_low_byte_nonzero() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let low_byte = entry.numeric & 0xFF;
            assert!(
                low_byte != 0,
                "Entry '{}': low byte must be non-zero (reserved for sentinel)",
                entry.symbolic
            );
        }
    }
}
