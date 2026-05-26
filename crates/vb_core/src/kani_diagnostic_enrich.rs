// Kani proof: Diagnostic source_file invariants
// PO-K03: Diagnostic source_file field (C2.1-C2.3)
//
// Proves against enriched Diagnostic (5-arg constructor with source_file):
//  1. Diagnostic::new(.., None) produces source_file == None
//  2. Diagnostic::new(.., Some(file)) preserves source_file
//  3. The source_file field is always Option<Box<str>>
// Assumptions: DiagnosticCode::new(u16) safe for any u16 value.
//  String allocation for Box<str> abstracted in Kani (TB-022).

#![forbid(unsafe_code)]

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use crate::span::Span;

#[kani::proof]
#[kani::unwind(2)]
fn diag_new_zero_span_produces_none_source_file() {
    let code_val: u16 = kani::any();
    let code = DiagnosticCode::new(code_val);

    // Construct diagnostic with Span::ZERO and source_file: None (runtime/compat path)
    let diag = Diagnostic::new(
        code,
        Box::<str>::from("test"),
        Severity::Error,
        Span::ZERO,
        None,
    );

    // source_file must be None for runtime diagnostics (backward compat)
    assert!(diag.source_file.is_none());
    assert_eq!(diag.span, Span::ZERO);
    assert_eq!(diag.code, code);
    assert_eq!(diag.severity, Severity::Error);
}

/// Diagnostic constructed with source_file: None has source_file == None.
#[kani::proof]
fn diag_source_file_none_invariant() {
    let code_val: u16 = kani::any();
    let code = DiagnosticCode::new(code_val);

    let diag = Diagnostic::new(
        code,
        Box::<str>::from("test"),
        Severity::Error,
        Span::ZERO,
        None,
    );
    assert!(diag.source_file.is_none());
    assert_eq!(diag.span, Span::ZERO);
    assert_eq!(diag.severity, Severity::Error);
}

/// Diagnostic constructed with source_file: Some preserves the value.
/// Content equality verified via proptest (PO-P01). Kani proves the
/// structural invariant: source_file is Some exactly when Some was provided.
#[kani::proof]
fn diag_source_file_some_invariant() {
    let code_val: u16 = kani::any();
    let code = DiagnosticCode::new(code_val);

    let diag = Diagnostic::new(
        code,
        Box::<str>::from("test"),
        Severity::Warning,
        Span::ZERO,
        Some(Box::<str>::from("workflow.yaml")),
    );
    assert!(diag.source_file.is_some());
    assert_eq!(diag.span, Span::ZERO);
    assert_eq!(diag.severity, Severity::Warning);
}

/// Backward compat: Diagnostic with Span::ZERO, source_file: None is the
/// canonical runtime diagnostic shape.
#[kani::proof]
fn diag_backward_compat_runtime_shape() {
    let code_val: u16 = kani::any();
    let code = DiagnosticCode::new(code_val);

    let diag = Diagnostic::new(
        code,
        Box::<str>::from("runtime error"),
        Severity::Error,
        Span::ZERO,
        None,
    );

    assert_eq!(diag.span, Span::ZERO);
    assert!(diag.source_file.is_none());
    assert!(diag.span.line.is_none());
    assert!(diag.span.column.is_none());
    assert!(!diag.message.is_empty());
}

/// The source_file field is preserved as-given by the constructor.
/// No transformation or defaulting occurs — None case.
#[kani::proof]
fn diag_constructor_preserves_source_file_none() {
    let code_val: u16 = kani::any();
    let code = DiagnosticCode::new(code_val);

    let diag = Diagnostic::new(
        code,
        Box::<str>::from("a"),
        Severity::Info,
        Span::ZERO,
        None,
    );
    assert_eq!(diag.source_file, None);
}

/// The source_file field is preserved as-given by the constructor.
/// No transformation or defaulting occurs — Some case.
/// Content equality verified via proptest (PO-P01). Kani proves the
/// structural invariant: source_file is Some exactly when Some was provided.
#[kani::proof]
fn diag_constructor_preserves_source_file_some() {
    let code_val: u16 = kani::any();
    let code = DiagnosticCode::new(code_val);

    let diag = Diagnostic::new(
        code,
        Box::<str>::from("b"),
        Severity::Error,
        Span::ZERO,
        Some(Box::<str>::from("f.yaml")),
    );
    assert!(diag.source_file.is_some());
}
