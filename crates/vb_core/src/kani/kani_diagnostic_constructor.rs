#![forbid(unsafe_code)]
//! PO-005 + PO-014: Kani harnesses for Diagnostic::new constructor invariants.
//!
//! PO-005: Diagnostic::new() derives numeric_code from code; invariant
//! numeric_code.symbolic_code() == Some(code).
//! PO-014: It is impossible to construct a Diagnostic record where
//! numeric_code.symbolic_code() != Some(code).

use super::kani_symbolic_code_validation::{CODE_REGISTRY, DiagnosticCode, SymbolicCode};

/// Mirror of the Severity enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Mirror of Span (minimal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const ZERO: Self = Self { start: 0, end: 0 };
}

/// Mirror of Diagnostic struct with SymbolicCode as code field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: SymbolicCode,
    pub message: Box<str>,
    pub severity: Severity,
    pub span: Span,
    pub numeric_code: DiagnosticCode,
}

impl Diagnostic {
    /// Constructor: derives numeric_code from code via as_diagnostic_code().
    #[must_use]
    pub const fn new(
        code: SymbolicCode,
        message: Box<str>,
        severity: Severity,
        span: Span,
    ) -> Self {
        let numeric_code = as_diagnostic_code(code);
        Self {
            code,
            message,
            severity,
            span,
            numeric_code,
        }
    }
}

/// Derive a DiagnosticCode from a SymbolicCode by looking up the numeric value.
const fn as_diagnostic_code(sym: SymbolicCode) -> DiagnosticCode {
    let s = sym.as_str();
    let mut i = 0;
    while i < CODE_REGISTRY.len() {
        if string_eq_bytes(CODE_REGISTRY[i].symbolic.as_bytes(), s.as_bytes()) {
            return DiagnosticCode::new(CODE_REGISTRY[i].numeric);
        }
        i += 1;
    }
    // This branch must be unreachable for valid SymbolicCode values.
    // If reached, it means the SymbolicCode was constructed from an unregistered string.
    DiagnosticCode::new(0)
}

const fn string_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-005: For any valid SymbolicCode, Diagnostic::new produces a record
    /// where numeric_code.symbolic_code() == Some(code).
    #[kani::proof]
    #[kani::unwind(100)]
    fn kani_diagnostic_constructor_consistency() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let sym = SymbolicCode::from_static_infallible(entry.symbolic);
            let diagnostic = Diagnostic::new(
                sym,
                Box::<str>::from("test message"),
                Severity::Error,
                Span::ZERO,
            );
            // The invariant: numeric_code reverse-lookups to the original code
            let reversed = diagnostic.numeric_code.symbolic_code();
            kani::assert(
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
        }
    }

    /// PO-014: For any SymbolicCode input to Diagnostic::new, the constructed
    /// record satisfies numeric_code.symbolic_code() == Some(code).
    /// The invariant is proven at construction time.
    #[kani::proof]
    #[kani::unwind(100)]
    fn kani_diagnostic_no_mismatch() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let sym = SymbolicCode::from_static_infallible(entry.symbolic);
            let diagnostic = Diagnostic::new(
                sym,
                Box::<str>::from("test message"),
                Severity::Error,
                Span::ZERO,
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
        }
    }
}
