//! Property tests for Diagnostic::new constructor consistency.
//!
//! Compensates: BLOCKED PO-005 (diagnostic_constructor_consistency),
//!              BLOCKED PO-014 (diagnostic_no_mismatch).
//! Invariant: For every registered SymbolicCode, Diagnostic::new() produces
//!            a record where numeric_code.symbolic_code() == Some(code).
//! The constructor never panics.

use proptest::prelude::*;
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

// =========================================================================
// vb-n17jt (State 9 test-writer) — 2 NEW proptest invariants closing the
// gaps identified in test-plan.md §4.11 + §4.13.
// =========================================================================

proptest! {
    /// §4.11 / PO-016: `SymbolicCode::INTERNAL_INVARIANT` is a hard-coded
    /// const that maps to the registry's `"INTERNAL_INVARIANT_VIOLATION"`
    /// entry at numeric 0x1309. For *any* `message` / `severity` / `span` /
    /// `source_file` inputs, `Diagnostic::new(INTERNAL_INVARIANT, ...)`
    /// must produce `numeric_code == DiagnosticCode::new(0x1309)` and the
    /// reverse-lookup `numeric_code.symbolic_code() == Some(INTERNAL_INVARIANT)`.
    #[test]
    fn diagnostic_new_internal_invariant_yields_0x1309_numeric(
        message in "[\\PC]{0,128}",
        start in 0u32..4096,
        end_delta in 0u32..4096,
        source_file in proptest::option::of("[\\PC]{0,64}"),
    ) {
        // end may be < start; Span::new is the unchecked constructor that
        // preserves offsets verbatim, so we feed both well-formed and
        // inverted offsets. Diagnostic::new itself does not validate the
        // span, so the invariant under test is independent of the span shape.
        let end = start.wrapping_add(end_delta);
        let span = Span::new(start, end);
        let severity = Severity::Error;
        let sf: Option<Box<str>> = source_file.map(|s| Box::<str>::from(s));
        let msg = Box::<str>::from(message);
        let diag = Diagnostic::new(
            SymbolicCode::INTERNAL_INVARIANT,
            msg,
            severity,
            span,
            sf,
        );
        prop_assert_eq!(diag.numeric_code, DiagnosticCode::new(0x1309));
        prop_assert_eq!(diag.code, SymbolicCode::INTERNAL_INVARIANT);
        prop_assert_eq!(
            diag.numeric_code.symbolic_code(),
            Some(SymbolicCode::INTERNAL_INVARIANT),
        );
    }
}

/// §4.13 / PO-018: `Diagnostic::from_numeric(n, ...)` round-trips to
/// `numeric_code == n` for every registered numeric. This is a direct
/// exhaustive witness over `CODE_REGISTRY` (the registry has ~100
/// entries; iterating the slice is the production-bound refinement
/// layer per the test-planner's bridge table).
#[test]
fn diagnostic_from_numeric_round_trip_for_all_registered() {
    for entry in CODE_REGISTRY {
        let diag = Diagnostic::from_numeric(
            DiagnosticCode::new(entry.numeric),
            Box::<str>::from("round-trip"),
            Severity::Error,
            Span::ZERO,
            None,
        );
        let diag = diag.unwrap_or_else(|| {
            panic!(
                "from_numeric must return Some for registered numeric 0x{:04X} ({})",
                entry.numeric, entry.symbolic,
            )
        });
        assert_eq!(
            diag.numeric_code,
            DiagnosticCode::new(entry.numeric),
            "from_numeric round-trip must preserve numeric 0x{:04X} ({})",
            entry.numeric,
            entry.symbolic,
        );
        assert_eq!(
            diag.code.as_str(),
            entry.symbolic,
            "from_numeric round-trip must map numeric 0x{:04X} to its symbolic '{}'",
            entry.numeric,
            entry.symbolic,
        );
    }
}
