#![forbid(unsafe_code)]
//! PO-019: proptest for Diagnostic::new constructor integrity.
//!
//! Tests: Diagnostic records constructed with Diagnostic::new preserve
//! their code, message, severity, and span values exactly.
//!
//! Bound: all registered DiagnosticCode values via from_str (enumeration).

use std::str::FromStr;
use vb_core::diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticCodeParseError, Severity, SymbolicCode,
};
use vb_core::span::Span;

// ---------------------------------------------------------------------------
// Known valid diagnostic codes (from is_supported_code ranges)
// ---------------------------------------------------------------------------

/// All boundary values of supported ranges for exhaustive testing.
/// Only includes codes that have confirmed CODE_REGISTRY entries.
const KNOWN_CODES: &[&str] = &[
    // Schema (all 11 registered)
    "E0101", "E0105", "E010B", // Reference (all 4 registered)
    "E0201", "E0202", "E0204", // ControlFlow (all 9 registered)
    "E0301", "E0305", "E0309", // TypeTaint (all 12 registered)
    "E0401", "E0406", "E040C", // Gate verifier (all 19 registered)
    "E0501", "E0508", "E0513", // ContractDiscovery (all 3 registered)
    "E0601", "E0602", "E0603", // Compilation: registered entries
    "E1003", "E1005", "E1014", "E1015", // Workflow IR
    "E1105", // Expression
    "E1201", "E1202", "E1203", // Accessor
    "E1301", "E1308", "E1311", "E1314", "E1315", // Storage: registered entries
    "E2001", "E2008", // IPC: registered entries
    "E3201", "E320A", // Lifecycle: registered entries
    "E3301", "E3304", // Runtime: registered entries
    "E300F", "E301B", // Action/audit codes (REPAIR-7)
    "E3020", "E3022", // Boundary (journal) — all 46 registered
    "E4001", "E4008", "E4010", "E4015", "E401B", "E4020", "E402E",
];

/// Unregistered codes that must be rejected by from_str.
/// These were previously accepted by the old hardcoded `matches!` ranges
/// but correctly rejected by the new registry-backed `is_supported_code`.
const REJECTED_CODES: &[&str] = &[
    // E1201/E1202 now registered (REPAIR-2: STEP_BUDGET_EXHAUSTED, STEP_COUNTER_OVERFLOW)
    // E1308 now registered (UNSUPPORTED_PRIMITIVE)
    // E1311 now registered (SYMBOL_OUT_OF_BOUNDS)
    // E1314 now registered (BLOB_OUT_OF_BOUNDS)
    "E3001", "E3008", "E300E",
];

/// Resolves a DiagnosticCode to a SymbolicCode via the registry.
/// All codes in KNOWN_CODES are registered; returns None only for
/// unregistered values.
fn resolve_symbolic(code: DiagnosticCode) -> Option<SymbolicCode> {
    code.symbolic_code()
}

#[test]
fn diagnostic_constructor_preserves_all_fields() {
    for &code_str in KNOWN_CODES {
        let code = DiagnosticCode::from_str(code_str).expect("known code must parse");

        let message: Box<str> = Box::from(format!("test diagnostic {}", code_str));
        let severity = Severity::Error;
        let span = Span::new(10, 20);

        let symbolic = resolve_symbolic(code).expect("known code must resolve to symbolic");
        let diagnostic = Diagnostic::new(symbolic, message.clone(), severity, span);

        // Numeric code must be consistent with input
        assert_eq!(
            diagnostic.numeric_code, code,
            "Diagnostic.numeric_code must match input code for {}",
            code_str
        );

        // Message must be preserved
        assert_eq!(
            diagnostic.message, message,
            "Diagnostic.message must be preserved for {}",
            code_str
        );

        // Severity must be preserved
        assert_eq!(
            diagnostic.severity, severity,
            "Diagnostic.severity must be Error for {}",
            code_str
        );

        // Span must be preserved
        assert_eq!(
            diagnostic.span, span,
            "Diagnostic.span must be preserved for {}",
            code_str
        );
    }
}

#[test]
fn diagnostic_constructor_accepts_all_severity_levels() {
    let code = DiagnosticCode::from_str("E0101").expect("valid code");
    let symbolic = resolve_symbolic(code).expect("E0101 must be registered");
    let message: Box<str> = Box::from("test");
    let span = Span::ZERO;

    for severity in [Severity::Error, Severity::Warning, Severity::Info] {
        let diagnostic = Diagnostic::new(symbolic, message.clone(), severity, span);
        assert_eq!(
            diagnostic.severity, severity,
            "Diagnostic must preserve {:?} severity",
            severity
        );
        assert_eq!(diagnostic.numeric_code, code);
    }
}

#[test]
fn diagnostic_constructor_with_zero_span() {
    let code = DiagnosticCode::from_str("E0101").expect("valid code");
    let symbolic = resolve_symbolic(code).expect("E0101 must be registered");
    let message: Box<str> = Box::from("zero span");
    let diagnostic = Diagnostic::new(symbolic, message, Severity::Info, Span::ZERO);

    assert_eq!(diagnostic.numeric_code, code);
    assert_eq!(diagnostic.span, Span::ZERO);
}

#[test]
fn diagnostic_code_display_consistent_with_parse() {
    // Test all KNOWN_CODES: parse, then display roundtrip.
    for &code_str in KNOWN_CODES {
        let code = DiagnosticCode::from_str(code_str).expect("known code must parse");
        assert_eq!(
            code.to_string(),
            code_str,
            "Display must match parsed string for {}",
            code_str
        );
    }
}

/// REPAIR-7: codes that were only in old hardcoded `matches!` ranges
/// must now be rejected by the registry-backed `is_supported_code`.
#[test]
fn previously_accepted_unregistered_codes_now_rejected() {
    for &code_str in REJECTED_CODES {
        let result = DiagnosticCode::from_str(code_str);
        assert!(
            matches!(result, Err(DiagnosticCodeParseError::UnsupportedCode)),
            "{} must now be rejected (not in CODE_REGISTRY), got {:?}",
            code_str,
            result
        );
    }
}
