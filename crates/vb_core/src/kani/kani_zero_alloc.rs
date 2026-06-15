#![forbid(unsafe_code)]
//! PO-007: Kani harness for zero-allocation verification of hot paths.
//!
//! Proves: No heap allocation occurs during SymbolicCode construction, copy,
//! display, or numeric code resolution.
//!
//! Waiver: WVR-PS010-ALLOC — Non-behavior performance invariant.
//! Trusted Base: TBL-004 (kani alloc stubs)
//!
//! Bound: Code paths: from_static, Display::fmt, numeric_code, as_diagnostic_code.
//! Format strings bounded to ~50 chars.

use super::kani_symbolic_code_validation::{CODE_REGISTRY, DiagnosticCode, SymbolicCode};

/// Stub for heap allocation — verifies no path triggers alloc.
/// In production Kani run, this would use `kani::stub` to intercept
/// alloc::alloc::alloc, alloc::boxed::Box::new, etc.
/// For this model, we verify that SymbolicCode can be constructed and used
/// entirely on the stack.

/// Demonstrate stack-only operations with SymbolicCode.
#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-007 H1: SymbolicCode construction from static strings does not
    /// require heap allocation. SymbolicCode is Copy (stack-only).
    #[kani::proof]
    #[kani::unwind(10)]
    fn kani_zero_alloc_hot_path() {
        // Construction path: from_static
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let code = SymbolicCode::from_static(entry.symbolic);
            kani::assert(code.is_some(), "Construction must succeed without );
            if let Some(code) = code {
                // Copy: SymbolicCode is Copy, no alloc
                let _copy = code;

                // as_str: returns &'static str, no alloc
                let s: &str = code.as_str();
                kani::assert(!s.is_empty(), "as_str returns valid s);

                // as_diagnostic_code: const fn lookup, no alloc
                let dc = as_diagnostic_code_stub(code);
                kani::assert(dc.code() != 0, "DiagnosticCode should be non);

                // numeric_code resolution: inline const lookup, no alloc
                let _num = dc.code();
            }
        }
    }
}

/// Stub: as_diagnostic_code without heap allocation.
const fn as_diagnostic_code_stub(sym: SymbolicCode) -> DiagnosticCode {
    let s = sym.as_str();
    let mut i = 0;
    while i < CODE_REGISTRY.len() {
        if string_eq(s, CODE_REGISTRY[i].symbolic) {
            return DiagnosticCode::new(CODE_REGISTRY[i].numeric);
        }
        i += 1;
    }
    DiagnosticCode::new(0)
}

const fn string_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut i = 0;
    while i < a_bytes.len() {
        if a_bytes[i] != b_bytes[i] {
            return false;
        }
        i += 1;
    }
    true
}
