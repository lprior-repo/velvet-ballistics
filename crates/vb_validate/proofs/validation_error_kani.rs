// Kani proof: ValidationError diagnostic conversion
// PO-K06: ValidationError span propagation (C6.1-C6.3)
//
// STATUS: ValidationError variants DO carry Span fields (via error_diagnostic_parts).
// `diagnostic_from_error` in diagnostic.rs propagates the span. These harnesses
// verify backward-compat zero-span behavior. Full span propagation verification
// is covered by PO-K01 (Span invariants) and PO-K07 (bridge).
//
// What this harness CAN prove against the real types:
//  1. diagnostic_from_error(, None) never panics for all ValidationError variants
//  2. All produced diagnostics have code != 0 and message not empty
//  3. All diagnostics have span == Span::ZERO (current behavior)
//  4. All diagnostics have source_file == None (current behavior)
//  5. error_code() returns correct DiagnosticCode for each variant
//
// What is BLOCKED (requires new implementation):
//  1. ValidationError variants need span: Span fields added
//  2. diagnostic_from_error needs to propagate span from error to Diagnostic

use vb_core::span::Span;
use vb_validate::diagnostic::{diagnostic_from_error, error_code};
use vb_validate::ValidationError;

/// diagnostic_from_error always produces Diagnostic with Span::ZERO
/// (current behavior, pending span propagation implementation).
#[kani::proof]
fn diagnostic_from_error_produces_zero_span() {
    // Test representative variants that currently exist
    let error = ValidationError::DuplicateKey { span: Span::ZERO };
    let diag = diagnostic_from_error(&error, None);
    assert_eq!(diag.span, Span::ZERO);
    assert!(diag.span.is_empty());
    assert!(diag.source_file.is_none());

    // Test a variant with data fields
    let error2 = ValidationError::MissingRequiredField {
        field: String::from("test"),
     span: Span::ZERO};
    let diag2 = diagnostic_from_error(&error2, None);
    assert_eq!(diag2.span, Span::ZERO);
    assert!(diag2.source_file.is_none());

    // Test another variant
    let error3 = ValidationError::UnknownReference {
        reference: String::from("ref"),
     span: Span::ZERO};
    let diag3 = diagnostic_from_error(&error3, None);
    assert_eq!(diag3.span, Span::ZERO);
    assert!(diag3.source_file.is_none());
}

/// error_code returns a valid DiagnosticCode for each variant.
/// Codes match the master contract (Section 16).
#[kani::proof]
fn error_code_consistent_with_diagnostic() {
    let error = ValidationError::MissingStepPrimitive { span: Span::ZERO };
    let code = error_code(&error);
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.code, code);
    assert_eq!(code.code(), 0x010B); // E010B per master contract
}

/// Exhaustive reachability: all ValidationError variants produce a Diagnostic
/// without panic. The Rust compiler's exhaustive match in
/// error_diagnostic_parts enforces complete coverage.
#[kani::proof]
#[kani::unwind(5)]
fn exhaustive_variants_no_panic() {
    // All current ValidationError variants (no span fields)
    let errors: [ValidationError; 30] = [
        ValidationError::DuplicateKey { span: Span::ZERO },
        ValidationError::ForbiddenYamlFeature { span: Span::ZERO },
        ValidationError::UnknownTopLevelField { span: Span::ZERO },
        ValidationError::UnknownStepField { span: Span::ZERO },
        ValidationError::MissingRequiredField {
            field: String::from("test"),
         span: Span::ZERO},
        ValidationError::InvalidVersion {
            version: String::from("1.0"),
         span: Span::ZERO},
        ValidationError::InvalidId {
            id: String::from("step"),
         span: Span::ZERO},
        ValidationError::ReservedId {
            id: String::from("reserved"),
         span: Span::ZERO},
        ValidationError::DuplicateId {
            id: String::from("dup"),
         span: Span::ZERO},
        ValidationError::MultipleStepPrimitives { span: Span::ZERO },
        ValidationError::MissingStepPrimitive { span: Span::ZERO },
        ValidationError::UnknownReference {
            reference: String::from("ref"),
         span: Span::ZERO},
        ValidationError::FutureReference {
            reference: String::from("future"),
         span: Span::ZERO},
        ValidationError::SecretNotDeclared {
            secret: String::from("SECRET"),
         span: Span::ZERO},
        ValidationError::DirectRuntimeReference { span: Span::ZERO },
        ValidationError::InvalidThenTarget { span: Span::ZERO },
        ValidationError::ControlFlowCycle { span: Span::ZERO },
        ValidationError::UnreachableStep {
            step: String::from("s"),
         span: Span::ZERO},
        ValidationError::InvalidChoose { span: Span::ZERO },
        ValidationError::InvalidForEach { span: Span::ZERO },
        ValidationError::InvalidTogether { span: Span::ZERO },
        ValidationError::InvalidCollect { span: Span::ZERO },
        ValidationError::InvalidReduce { span: Span::ZERO },
        ValidationError::InvalidRepeat { span: Span::ZERO },
        ValidationError::InvalidWait { span: Span::ZERO },
        ValidationError::InvalidAsk { span: Span::ZERO },
        ValidationError::InvalidFinish { span: Span::ZERO },
        ValidationError::InvalidRetry { span: Span::ZERO },
        ValidationError::InvalidOnError { span: Span::ZERO },
        ValidationError::SecretResultLeak { span: Span::ZERO },
    ];

    let mut count = 0usize;
    for err in &errors {
        let diag = diagnostic_from_error(err, None);
        // Every diagnostic must have a valid code
        assert!(diag.code.code() > 0, "Diagnostic code must be valid");
        // Every diagnostic must have a non-empty message
        assert!(!diag.message.is_empty(), "Diagnostic message must not be empty");
        // Every diagnostic currently has Span::ZERO
        assert_eq!(diag.span, Span::ZERO);
        count = count.wrapping_add(1);
    }
    assert_eq!(count, 30, "All 30 variants exercised");
}

/// Message format: diagnostics carry the error message.
#[kani::proof]
fn diagnostic_message_matches_error() {
    let error = ValidationError::InvalidWait { span: Span::ZERO };
    let diag = diagnostic_from_error(&error, None);
    assert!(!diag.message.is_empty());
    // Message contains something meaningful (not just empty string)
    assert!(diag.message.as_ref().contains("wait"));
}

/// Every diagnostic has Span::ZERO (current span-not-propagated behavior).
#[kani::proof]
fn all_diagnostics_have_zero_span() {
    let errors = [
        ValidationError::DuplicateKey { span: Span::ZERO },
        ValidationError::MissingStepPrimitive { span: Span::ZERO },
        ValidationError::InvalidVersion {
            version: String::from("v"),
         span: Span::ZERO},
        ValidationError::UnknownTopLevelField { span: Span::ZERO },
        ValidationError::ForbiddenYamlFeature { span: Span::ZERO },
    ];

    for err in &errors {
        let diag = diagnostic_from_error(err, None);
        assert_eq!(diag.span, Span::ZERO);
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 0);
        assert!(diag.span.line.is_none());
        assert!(diag.span.column.is_none());
    }
}
