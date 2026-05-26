#![forbid(unsafe_code)]
//! PO-005 + PO-014: Kani harnesses for Diagnostic::new constructor invariants.
//!
//! PO-005: Diagnostic::new() derives numeric_code from code; invariant
//! numeric_code.symbolic_code() == Some(code).
//! PO-014: It is impossible to construct a Diagnostic record where
//! numeric_code.symbolic_code() != Some(code).
//!
//! Rewired: uses production types from crate::diagnostic.
//! Does NOT redefine DiagnosticCode::symbolic_code() — uses production impl.

use crate::diagnostic::{CODE_REGISTRY, Diagnostic, Severity, SymbolicCode};
use crate::span::Span;

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-005: For any valid SymbolicCode, Diagnostic::new produces a record
    /// where numeric_code.symbolic_code() == Some(code).
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_diagnostic_constructor_consistency() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let sym = SymbolicCode::from_static(entry.symbolic)
                .expect("Registry entry must produce valid SymbolicCode");

            // Use production Diagnostic::new directly
            let diagnostic = Diagnostic::new(
                sym,
                Box::<str>::from("test message"),
                Severity::Error,
                Span::ZERO,
            );

            // The invariant: numeric_code reverse-lookups to the original code
            let reversed = diagnostic.numeric_code.symbolic_code();
            assert!(
                reversed.is_some(),
                "numeric_code must resolve to a SymbolicCode"
            );
            assert_eq!(
                reversed,
                Some(sym),
                "Reverse lookup must return the original SymbolicCode"
            );
            assert_eq!(
                diagnostic.code, sym,
                "Diagnostic.code must match the input SymbolicCode"
            );
        }
    }

    /// PO-014: For any SymbolicCode input to Diagnostic::new, the constructed
    /// record satisfies numeric_code.symbolic_code() == Some(code).
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_diagnostic_no_mismatch() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let sym = SymbolicCode::from_static(entry.symbolic)
                .expect("Registry entry must produce valid SymbolicCode");

            let diagnostic = Diagnostic::new(
                sym,
                Box::<str>::from("test message"),
                Severity::Error,
                Span::ZERO,
            );

            let numeric_sym = diagnostic.numeric_code.symbolic_code();
            assert_eq!(
                numeric_sym,
                Some(sym),
                "Invariant: numeric_code.symbolic_code() must equal Some(code)"
            );

            assert_eq!(
                diagnostic.numeric_code.code(),
                entry.numeric,
                "Numeric code must match the registry entry for this SymbolicCode"
            );
        }
    }
}
