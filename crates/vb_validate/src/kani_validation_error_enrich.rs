// Kani proof: ValidationError diagnostic conversion with span propagation
// PO-K06: ValidationError span propagation (C6.1-C6.3)
//
// Implementation update: All ValidationError variants now carry `span: Span`
// fields, and `diagnostic_from_error` in diagnostic.rs propagates the span.
//
// Proves:
//  1. diagnostic_from_error(, None) never panics for all ValidationError variants
//  2. Span is correctly propagated from ValidationError to Diagnostic
//  3. Non-zero spans are preserved
//  4. Backward compat: Span::ZERO propagates as Span::ZERO
//  5. All diagnostics have valid codes and non-empty messages

#![forbid(unsafe_code)]

use crate::ValidationError;
use crate::diagnostic::{diagnostic_from_error, error_code};
use vb_core::span::Span;

// ---------------------------------------------------------------------------
// Span propagation: span flows from error into diagnostic
// ---------------------------------------------------------------------------

/// diagnostic_from_error propagates the span for DuplicateKey.
#[kani::proof]
#[kani::unwind(3)]
fn diagnostic_propagates_span_duplicate_key() {
    let start: u32 = kani::any();
    let end: u32 = kani::any();
    kani::assume(start <= end);
    let input_span = Span::new(start, end);

    let error = ValidationError::DuplicateKey { span: input_span };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, input_span);
    assert_eq!(diag.span.start, start);
    assert_eq!(diag.span.end, end);
}

/// diagnostic_from_error propagates the span for ForbiddenYamlFeature.
#[kani::proof]
#[kani::unwind(3)]
fn diagnostic_propagates_span_forbidden_yaml() {
    let start: u32 = kani::any();
    let end: u32 = kani::any();
    kani::assume(start <= end);
    let input_span = Span::new(start, end);

    let error = ValidationError::ForbiddenYamlFeature { span: input_span };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, input_span);
}

/// diagnostic_from_error propagates the span for ControlFlowCycle.
#[kani::proof]
#[kani::unwind(3)]
fn diagnostic_propagates_span_control_flow() {
    let start: u32 = kani::any();
    let end: u32 = kani::any();
    kani::assume(start <= end);
    let input_span = Span::new(start, end);

    let error = ValidationError::ControlFlowCycle { span: input_span };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, input_span);
}

/// diagnostic_from_error propagates the span for data-carrying variants.
#[kani::proof]
#[kani::unwind(3)]
fn diagnostic_propagates_span_missing_required_field() {
    let start: u32 = kani::any();
    let end: u32 = kani::any();
    kani::assume(start <= end);
    let input_span = Span::new(start, end);

    let error = ValidationError::MissingRequiredField {
        field: String::from("test"),
        span: input_span,
    };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, input_span);
}

/// diagnostic_from_error propagates the span for SecretResultLeak.
#[kani::proof]
#[kani::unwind(3)]
fn diagnostic_propagates_span_secret_leak() {
    let start: u32 = kani::any();
    let end: u32 = kani::any();
    kani::assume(start <= end);
    let input_span = Span::new(start, end);

    let error = ValidationError::SecretResultLeak { span: input_span };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, input_span);
}

// ---------------------------------------------------------------------------
// Backward compatibility: Span::ZERO produces Span::ZERO
// ---------------------------------------------------------------------------

/// When error has Span::ZERO, diagnostic has Span::ZERO.
#[kani::proof]
fn diagnostic_zero_span_produces_zero_span() {
    let error = ValidationError::DuplicateKey { span: Span::ZERO };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, Span::ZERO);
    assert!(diag.span.is_empty());
    assert_eq!(diag.span.start, 0);
    assert_eq!(diag.span.end, 0);
    assert!(diag.span.line.is_none());
    assert!(diag.span.column.is_none());
}

// ---------------------------------------------------------------------------
// Exhaustive: all variants produce valid diagnostics without panic
// ---------------------------------------------------------------------------

/// All ValidationError variants produce a Diagnostic without panic, with
/// valid code and non-empty message. Uses Span::ZERO for each.
#[kani::proof]
#[kani::unwind(5)]
fn exhaustive_variants_no_panic() {
    let zero = Span::ZERO;
    let errors: [ValidationError; 30] = [
        ValidationError::DuplicateKey { span: zero },
        ValidationError::ForbiddenYamlFeature { span: zero },
        ValidationError::UnknownTopLevelField { span: zero },
        ValidationError::UnknownStepField { span: zero },
        ValidationError::MissingRequiredField {
            field: String::from("test"),
            span: zero,
        },
        ValidationError::InvalidVersion {
            version: String::from("1.0"),
            span: zero,
        },
        ValidationError::InvalidId {
            id: String::from("step"),
            span: zero,
        },
        ValidationError::ReservedId {
            id: String::from("reserved"),
            span: zero,
        },
        ValidationError::DuplicateId {
            id: String::from("dup"),
            span: zero,
        },
        ValidationError::MultipleStepPrimitives { span: zero },
        ValidationError::MissingStepPrimitive { span: zero },
        ValidationError::UnknownReference {
            reference: String::from("ref"),
            span: zero,
        },
        ValidationError::FutureReference {
            reference: String::from("future"),
            span: zero,
        },
        ValidationError::SecretNotDeclared {
            secret: String::from("SECRET"),
            span: zero,
        },
        ValidationError::DirectRuntimeReference { span: zero },
        ValidationError::InvalidThenTarget { span: zero },
        ValidationError::ControlFlowCycle { span: zero },
        ValidationError::UnreachableStep {
            step: String::from("s"),
            span: zero,
        },
        ValidationError::InvalidChoose { span: zero },
        ValidationError::InvalidForEach { span: zero },
        ValidationError::InvalidTogether { span: zero },
        ValidationError::InvalidCollect { span: zero },
        ValidationError::InvalidReduce { span: zero },
        ValidationError::InvalidRepeat { span: zero },
        ValidationError::InvalidWait { span: zero },
        ValidationError::InvalidAsk { span: zero },
        ValidationError::InvalidFinish { span: zero },
        ValidationError::InvalidRetry { span: zero },
        ValidationError::InvalidOnError { span: zero },
        ValidationError::SecretResultLeak { span: zero },
    ];

    for err in &errors {
        let diag = diagnostic_from_error(err, None);
        // Every diagnostic must have a valid code
        assert!(diag.code.code() > 0, "Diagnostic code must be valid");
        // Every diagnostic must have a non-empty message
        assert!(
            !diag.message.is_empty(),
            "Diagnostic message must not be empty"
        );
        // Span is preserved (zero in this test)
        assert_eq!(diag.span, Span::ZERO);
    }
}

// ---------------------------------------------------------------------------
// Span with location fields propagates through diagnostic
// ---------------------------------------------------------------------------

/// Span with location (line, column) data is preserved in diagnostic.
#[kani::proof]
#[kani::unwind(3)]
fn span_with_location_propagated() {
    let start: u32 = kani::any();
    let end: u32 = kani::any();
    let line: u32 = kani::any();
    let col: u32 = kani::any();
    kani::assume(start <= end);
    kani::assume(line >= 1);
    kani::assume(col >= 1);

    let input_span = Span::with_location(start, end, line, col);

    let error = ValidationError::InvalidThenTarget { span: input_span };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, input_span);
    assert_eq!(diag.span.line, Some(line));
    assert_eq!(diag.span.column, Some(col));
    assert_eq!(diag.span.location(), Some((line, col)));
}

// ---------------------------------------------------------------------------
// error_code consistency
// ---------------------------------------------------------------------------

/// error_code matches the code embedded in the diagnostic.
#[kani::proof]
fn error_code_consistent_with_diagnostic() {
    let error = ValidationError::MissingStepPrimitive { span: Span::ZERO };
    let code = error_code(&error);
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.code, code);
    assert_eq!(code.code(), 0x010B); // E010B per master contract
}
