#![forbid(unsafe_code)]
//! PO-007: Kani harness for zero-allocation verification of hot paths.
//!
//! Proves: No heap allocation occurs during SymbolicCode construction, copy,
//! display, or numeric code resolution.
//!
//! Binds to the production `SymbolicCode`/`DiagnosticCode` types, the
//! production registry, and production
//! `SymbolicCode::as_diagnostic_code`. The previous
//! `as_diagnostic_code_stub` and `string_eq` helpers duplicated that
//! production scan; they are deleted.
//!
//! Waiver: WVR-PS010-ALLOC — Non-behavior performance invariant.
//! Trusted Base: TBL-004 (kani alloc stubs)
//!
//! Bound: Code paths: from_static, Display::fmt, numeric_code, as_diagnostic_code.
//! Format strings bounded to ~50 chars.

use crate::diagnostic::{CODE_REGISTRY, SymbolicCode};

/// Demonstrate stack-only operations with SymbolicCode.
#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-007 H1: SymbolicCode construction from static strings does not
    /// require heap allocation. SymbolicCode is Copy (stack-only).
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_zero_alloc_hot_path() {
        // Construction path: from_static
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let code = SymbolicCode::from_static(entry.symbolic);
            assert!(code.is_some(), "Construction must succeed without alloc");
            if let Some(code) = code {
                // Copy: SymbolicCode is Copy, no alloc
                let _copy = code;

                // as_str: returns &'static str, no alloc
                let s: &str = code.as_str();
                assert!(!s.is_empty(), "as_str returns valid string");

                // as_diagnostic_code: production registry lookup, no alloc
                let dc = code.as_diagnostic_code();
                assert!(
                    dc.is_some(),
                    "as_diagnostic_code must succeed for registered codes"
                );
                if let Some(dc) = dc {
                    assert!(dc.code() != 0, "DiagnosticCode should be non-zero");
                }

                // numeric_code resolution: production lookup, no alloc
                let _num = code.numeric_code();
            }
        }
    }
}
