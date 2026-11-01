#![forbid(unsafe_code)]
//! PO-005 + PO-014: Kani harnesses for Diagnostic::new constructor invariants.
//!
//! PO-005: Production `Diagnostic::new()` derives `numeric_code` from
//! `code`; invariant `numeric_code.symbolic_code() == Some(code)`.
//! PO-014: It is impossible to construct a Diagnostic record where
//! `numeric_code.symbolic_code() != Some(code)`.
//!
//! Binds to the production `Diagnostic`, `Severity`, and `Span`
//! types and the production registry. The previous mirrored
//! `Diagnostic`/`Severity`/`Span` structs and the hand-rolled
//! `as_diagnostic_code` scan are deleted; the harnesses execute
//! production `Diagnostic::new` and production reverse lookup.
//!
//! Bound: Production registry size (unwind=300).

use crate::diagnostic::{CODE_REGISTRY, Diagnostic, Severity, SymbolicCode};
use crate::span::Span;

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-005: For every production registry entry, production
    /// `Diagnostic::new` produces a record where
    /// `numeric_code.symbolic_code() == Some(code)` and the numeric
    /// value matches the registry entry.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_diagnostic_constructor_consistency() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            if let Some(sym) = SymbolicCode::from_static(entry.symbolic) {
                let diagnostic = Diagnostic::new(
                    sym,
                    Box::<str>::from("test message"),
                    Severity::Error,
                    Span::ZERO,
                    None,
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
                // Also verify code matches
                assert_eq!(
                    diagnostic.code, sym,
                    "Diagnostic.code must match the input SymbolicCode"
                );
                // And the derived numeric value matches the registry entry
                assert_eq!(
                    diagnostic.numeric_code.code(),
                    entry.numeric,
                    "Numeric code must match the registry entry for this SymbolicCode"
                );
            } else {
                kani::assert(false, "registered symbolic must construct");
            }
        }
    }

    /// PO-014: For every production registry entry, the record built
    /// by production `Diagnostic::new` satisfies
    /// `numeric_code.symbolic_code() == Some(code)`.
    /// The invariant is proven at construction time.
    #[kani::proof]
    #[kani::unwind(300)]
    fn kani_diagnostic_no_mismatch() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            if let Some(sym) = SymbolicCode::from_static(entry.symbolic) {
                let diagnostic = Diagnostic::new(
                    sym,
                    Box::<str>::from("test message"),
                    Severity::Error,
                    Span::ZERO,
                    None,
                );
                // The core invariant: no mismatch between symbolic and numeric codes
                let numeric_sym = diagnostic.numeric_code.symbolic_code();
                assert_eq!(
                    numeric_sym,
                    Some(sym),
                    "Invariant: numeric_code.symbolic_code() must equal Some(code)"
                );

                // Also verify that numeric_code's inner value matches the registry
                assert_eq!(
                    diagnostic.numeric_code.code(),
                    entry.numeric,
                    "Numeric code must match the registry entry for this SymbolicCode"
                );
            } else {
                kani::assert(false, "registered symbolic must construct");
            }
        }
    }
}
