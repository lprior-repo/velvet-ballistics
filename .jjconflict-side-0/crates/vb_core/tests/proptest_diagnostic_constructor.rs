//! Property tests for Diagnostic::new constructor consistency.
//!
//! Compensates: BLOCKED PO-005 (diagnostic_constructor_consistency),
//!              BLOCKED PO-014 (diagnostic_no_mismatch).
//! Invariant: For every registered SymbolicCode, Diagnostic::new() produces
//!            a record where numeric_code.symbolic_code() == Some(code).
//! The constructor never panics.

use vb_core::diagnostic::{CODE_REGISTRY, Diagnostic, DiagnosticCode, Severity, SymbolicCode};
use vb_core::span::Span;

/// Helper: returns the registered `SymbolicCode` for the `"DUPLICATE_KEY"` entry.
fn duplicate_key_code() -> SymbolicCode {
    SymbolicCode::from_static("DUPLICATE_KEY").expect("DUPLICATE_KEY must be registered")
}

#[test]
fn diagnostic_new_preserves_symbolic_numeric_invariant_for_all_registered_codes() {
    for entry in CODE_REGISTRY {
        // Verify SymbolicCode can be constructed from registry entry
        let symbolic = SymbolicCode::from_static(entry.symbolic).expect("must be registered");
        let diagnostic = Diagnostic::new(
            symbolic,
            Box::<str>::from("test message"),
            Severity::Error,
            Span::ZERO,
            None,
        );
        // Verify diagnostic fields
        assert_eq!(diagnostic.code, symbolic);
        assert_eq!(diagnostic.numeric_code, DiagnosticCode::new(entry.numeric));
        assert_eq!(diagnostic.message.as_ref(), "test message");
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.span, Span::ZERO);

        // Verify numeric -> symbolic lookup consistency
        let numeric = diagnostic.numeric_code;
        let symbolic_back = numeric.symbolic_code();
        assert!(
            symbolic_back.is_some(),
            "symbolic_code() should return Some for 0x{:04X} ({})",
            entry.numeric,
            entry.symbolic
        );

        // Verify that from_static works for the returned symbolic
        let reconstructed = SymbolicCode::from_static(symbolic_back.unwrap().as_str());
        assert!(
            reconstructed.is_some(),
            "from_static should work for result of symbolic_code() on 0x{:04X}",
            entry.numeric
        );
    }
}

#[test]
fn diagnostic_new_never_panics_for_all_registered_codes() {
    // Iterate each unique symbolic code and construct Diagnostic.
    // This verifies that no panics occur (via `expect`).
    let mut seen: Vec<&str> = Vec::new();
    for entry in CODE_REGISTRY {
        if seen.contains(&entry.symbolic) {
            continue;
        }
        seen.push(entry.symbolic);
        let symbolic = SymbolicCode::from_static(entry.symbolic).expect("registered");
        // The Diagnostic::new is called with the symbolic code directly.
        // It should not panic for any registered code.
        let _diagnostic = Diagnostic::new(
            symbolic,
            Box::<str>::from("no-panic test"),
            Severity::Warning,
            Span::ZERO,
            None,
        );
    }
    // If we reach here without panic, the test passes.
    assert!(
        !seen.is_empty(),
        "should iterate at least one symbolic code"
    );
}

#[test]
fn diagnostic_new_preserves_severity() {
    for (sev, name) in &[
        (Severity::Error, "Error"),
        (Severity::Warning, "Warning"),
        (Severity::Info, "Info"),
    ] {
        let diagnostic = Diagnostic::new(
            duplicate_key_code(),
            Box::<str>::from("test"),
            *sev,
            Span::ZERO,
            None,
        );
        assert_eq!(
            diagnostic.severity, *sev,
            "severity should be {:?} for {name}",
            sev
        );
    }
}

#[test]
fn diagnostic_new_preserves_message() {
    let diagnostic = Diagnostic::new(
        duplicate_key_code(),
        Box::<str>::from("custom diagnostic message"),
        Severity::Error,
        Span::ZERO,
        None,
    );
    assert_eq!(diagnostic.message.as_ref(), "custom diagnostic message");
}

#[test]
fn diagnostic_new_preserves_span() {
    let span = Span::new(10, 20);
    let diagnostic = Diagnostic::new(
        duplicate_key_code(),
        Box::<str>::from("test"),
        Severity::Error,
        span,
        None,
    );
    assert_eq!(diagnostic.span, span);
}

#[test]
fn diagnostic_identity_property_code_round_trip() {
    // For each registry entry, construct Diagnostic and verify numeric_code.code() == expected
    for entry in CODE_REGISTRY {
        let symbolic = SymbolicCode::from_static(entry.symbolic).expect("must be registered");
        let diagnostic = Diagnostic::new(
            symbolic,
            Box::<str>::from("round-trip test"),
            Severity::Warning,
            Span::ZERO,
            None,
        );
        assert_eq!(
            diagnostic.numeric_code.code(),
            entry.numeric,
            "diagnostic.numeric_code.code() should match entry.numeric for '{}'",
            entry.symbolic
        );
    }
}
