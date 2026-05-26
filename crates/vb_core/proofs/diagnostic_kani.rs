// Kani proof: Diagnostic source_file invariants
// PO-K03: Diagnostic source_file field (C2.1-C2.3)
//
// Proves against enriched Diagnostic (5-arg constructor with source_file):
//  1. Diagnostic::new(.., None) produces source_file == None
//  2. Diagnostic::new(.., Some(file)) preserves source_file
//  3. The source_file field is always Option<Box<str>>
// Assumptions: DiagnosticCode::new(u16) safe for any u16 value.
//  String allocation for Box<str> abstracted in Kani (TB-022).

use vb_core::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use vb_core::span::Span;

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

#[kani::proof]
fn diag_source_file_invariant() {
    let code_val: u16 = kani::any();
    let code = DiagnosticCode::new(code_val);
    let sp_start: u32 = kani::any();
    let sp_end: u32 = kani::any();
    let span = Span::new(sp_start, sp_end);

    // Construct with explicit source_file: None
    let diag_none = Diagnostic::new(
        code,
        Box::<str>::from("test"),
        Severity::Error,
        span,
        None,
    );
    assert!(diag_none.source_file.is_none());
    assert_eq!(diag_none.span, span);

    // Construct with explicit source_file: Some
    let diag_some = Diagnostic::new(
        code,
        Box::<str>::from("test"),
        Severity::Warning,
        span,
        Some(Box::<str>::from("workflow.yaml")),
    );
    assert!(diag_some.source_file.is_some());
    assert_eq!(diag_some.source_file.as_deref(), Some("workflow.yaml"));
    assert_eq!(diag_some.span, span);
    assert_eq!(diag_some.severity, Severity::Warning);
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
/// No transformation or defaulting occurs.
#[kani::proof]
fn diag_constructor_preserves_source_file_exactly() {
    let code_val: u16 = kani::any();
    let code = DiagnosticCode::new(code_val);
    let sp_start: u32 = kani::any();
    let sp_end: u32 = kani::any();

    // source_file: None
    let diag1 = Diagnostic::new(code, Box::<str>::from("a"), Severity::Info, Span::new(sp_start, sp_end), None);
    assert_eq!(diag1.source_file, None);

    // source_file: Some
    let diag2 = Diagnostic::new(code, Box::<str>::from("b"), Severity::Error, Span::new(sp_start, sp_end), Some(Box::<str>::from("f.yaml")));
    assert_eq!(diag2.source_file.as_deref(), Some("f.yaml"));
}
