// Proptest: ValidationError diagnostic conversion
// PO-P04: ValidationError diagnostic properties (C6.2)
//
// TF-VB-005: BLOCKED BEHAVIOR — ValidationError span propagation
// =================================================================
//
// BLOCKER-IMPLEMENTATION: ValidationError variants do NOT carry Span fields
// with YAML-parsed location data. The current `diagnostic_from_error` always
// uses Span::ZERO. This is documented in the accepted behavior gap
// GAP-VERR-SPAN (see proof-findings.jsonl).
//
// Contract: C6.1-C6.2 (VERR-SPAN)
//   C6.1: ValidationError variants SHALL carry a Span field populated from
//         the YAML parse location when available.
//   C6.2: diagnostic_from_error(, None) SHALL propagate the error's Span into
//         the resulting Diagnostic.span, preserving line and column.
//
// Behaviors awaiting implementation (B61-B63, B67, B70):
//   B61: diagnostic_from_error propagates non-ZERO span exactly
//   B62: location-bearing span preserves line and column in diagnostic
//   B63: source_file propagates from context into Diagnostic
//   B67: all 55 variants have unique diagnostic codes (C6.3)
//   B70: future variants from #[non_exhaustive] produce valid diagnostics
//
// What this test suite CAN prove (current state — documented baseline):
//  1. Every ValidationError variant produces a valid Diagnostic (no panic)
//  2. All diagnostics have consistent severity=Error
//  3. Messages are non-empty for all variants
//  4. All diagnostics currently use Span::ZERO (known gap)
//  5. source_file is always None (not yet propagated)
//  6. Diagnostic codes are > 0 for all variants
//  7. Diagnostics are deterministic (same input → same output)
//
// Unblock conditions:
//  1. ValidationError variants gain Span fields populated from YAML parse
//     location (requires vb_yaml → vb_validate span pipe)
//  2. diagnostic_from_error(, None) reads ValidationError.span and writes
//     Diagnostic.span (currently hardcoded Span::ZERO)
//  3. ValidationContext carries source_file: Option<String> for B63
//
// When unblocked, expand this test suite with:
//  - span_propagates_exactly_to_diagnostic() (B61)
//  - span_preserves_line_and_column() (B62)
//  - source_file_propagates_to_diagnostic() (B63)
//  - all_variants_have_unique_diagnostic_codes() (B65/B67)
//  - future_variant_produces_parse_error_diagnostic() (B70)
//
// See: test-plan.md Section 8.6, behaviors B61-B70
// See: contract.md C6.1-C6.3
// See: proof-findings.jsonl entry GAP-VERR-SPAN

use vb_core::diagnostic::Severity;
use vb_core::span::Span;
use vb_validate::ValidationError;
use vb_validate::diagnostic::diagnostic_from_error;

/// All ValidationError variants.
fn all_validation_errors() -> Vec<ValidationError> {
    vec![
        ValidationError::DuplicateKey { span: Span::ZERO },
        ValidationError::ForbiddenYamlFeature { span: Span::ZERO },
        ValidationError::UnknownTopLevelField { span: Span::ZERO },
        ValidationError::UnknownStepField { span: Span::ZERO },
        ValidationError::MissingRequiredField {
            field: String::from("test"),
            span: Span::ZERO,
        },
        ValidationError::InvalidVersion {
            version: String::from("1.0"),
            span: Span::ZERO,
        },
        ValidationError::InvalidId {
            id: String::from("step"),
            span: Span::ZERO,
        },
        ValidationError::ReservedId {
            id: String::from("reserved"),
            span: Span::ZERO,
        },
        ValidationError::DuplicateId {
            id: String::from("dup"),
            span: Span::ZERO,
        },
        ValidationError::MultipleStepPrimitives { span: Span::ZERO },
        ValidationError::MissingStepPrimitive { span: Span::ZERO },
        ValidationError::UnknownReference {
            reference: String::from("ref"),
            span: Span::ZERO,
        },
        ValidationError::FutureReference {
            reference: String::from("future"),
            span: Span::ZERO,
        },
        ValidationError::SecretNotDeclared {
            secret: String::from("SECRET"),
            span: Span::ZERO,
        },
        ValidationError::DirectRuntimeReference { span: Span::ZERO },
        ValidationError::InvalidThenTarget { span: Span::ZERO },
        ValidationError::ControlFlowCycle { span: Span::ZERO },
        ValidationError::UnreachableStep {
            step: String::from("s"),
            span: Span::ZERO,
        },
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
    ]
}

#[test]
fn all_errors_produce_valid_diagnostic() {
    for error in all_validation_errors() {
        let diag = diagnostic_from_error(&error, None);
        assert!(diag.code.code() > 0, "Diagnostic code must be valid");
        assert!(
            !diag.message.is_empty(),
            "Diagnostic message must not be empty"
        );
        assert_eq!(diag.span, Span::ZERO);
        assert!(diag.source_file.is_none());
    }
}

#[test]
fn diagnostics_are_deterministic() {
    let error = ValidationError::MissingStepPrimitive { span: Span::ZERO };
    let diag1 = diagnostic_from_error(&error, None);
    let diag2 = diagnostic_from_error(&error, None);
    assert_eq!(diag1.code, diag2.code);
    assert_eq!(diag1.message, diag2.message);
    assert_eq!(diag1.span, diag2.span);
    assert_eq!(diag1.severity, diag2.severity);
}

#[test]
fn span_is_always_zero() {
    for error in all_validation_errors() {
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.span, Span::ZERO);
        assert!(diag.span.is_empty());
        assert_eq!(diag.span.line, None);
        assert_eq!(diag.span.column, None);
    }
}

#[test]
fn severity_is_always_error() {
    for error in all_validation_errors() {
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.severity, Severity::Error);
    }
}

#[test]
fn source_file_always_none() {
    for error in all_validation_errors() {
        let diag = diagnostic_from_error(&error, None);
        assert!(diag.source_file.is_none());
    }
}
