#![forbid(unsafe_code)]
//! PO-011: Kani harness verifying CODE_REGISTRY category consistency.
//!
//! Binds to the production registry, production `category_from_numeric`,
//! and production `DiagnosticCode::category`. The previous harness
//! carried its own expected-high-byte table that could not represent
//! the production `Ipc`, `Lifecycle`, or `Internal` categories and
//! contradicted registry entries with explicit categories (e.g.
//! `INTERNAL_INVARIANT_VIOLATION` at `0x1309`); that table is deleted.
//!
//! Bound: Production registry size (unwind=300).

use crate::diagnostic::{CODE_REGISTRY, DiagnosticCode, category_from_numeric};

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-011: For each production registry entry, production
    /// `category_from_numeric` returns the entry's own category
    /// (registry-first lookup).
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_registry_category_match() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            assert_eq!(
                category_from_numeric(entry.numeric),
                entry.category,
                "Production category_from_numeric must return the registry category"
            );
        }
    }

    /// PO-011b: For each production registry entry, production
    /// `DiagnosticCode::category` (gate + classify path) returns
    /// `Some(entry.category)`.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_diagnostic_code_category_match() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let dc = DiagnosticCode::new(entry.numeric);
            assert_eq!(
                dc.category(),
                Some(entry.category),
                "Production DiagnosticCode::category must match the registry"
            );
        }
    }

    /// PO-011c: No production entry has a zero low byte (reserved
    /// for the sentinel range); verified over the production registry.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_registry_schema_low_byte_nonzero() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let low_byte = entry.numeric & 0xFF;
            // Low byte must be non-zero for all categories
            assert!(
                low_byte != 0,
                "Entry '{}': low byte must be non-zero (reserved for sentinel)",
                entry.symbolic
            );
        }
    }
}
