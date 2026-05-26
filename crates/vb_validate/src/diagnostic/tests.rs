#![forbid(unsafe_code)]
//! Tests for diagnostic conversion (error-to-Diagnostic mapping).
//!
//! Uses the canonical `all_variants()` from `diag_convert`.

use super::mapping::{diagnostic_from_error, error_code};
use crate::ValidationError;
use crate::diag_convert::all_variants;
use vb_core::diagnostic::Severity;
use vb_core::span::Span;

#[test]
fn duplicate_key_maps_to_e0101() {
    let diag = diagnostic_from_error(&ValidationError::DuplicateKey { span: Span::ZERO }, None);
    assert_eq!(diag.code.code(), 0x0101);
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn invalid_version_maps_to_e0106() {
    let diag = diagnostic_from_error(
        &ValidationError::InvalidVersion {
            version: "v2".into(),
            span: Span::ZERO,
        },
        None,
    );
    assert_eq!(diag.code.code(), 0x0106);
    assert!(diag.message.contains("v2"));
}

#[test]
fn unknown_reference_maps_to_e0201() {
    let diag = diagnostic_from_error(
        &ValidationError::UnknownReference {
            reference: "$input.missing".into(),
            span: Span::ZERO,
        },
        None,
    );
    assert_eq!(diag.code.code(), 0x0201);
}

#[test]
fn control_flow_cycle_maps_to_e0302() {
    let diag = diagnostic_from_error(
        &ValidationError::ControlFlowCycle { span: Span::ZERO },
        None,
    );
    assert_eq!(diag.code.code(), 0x0302);
}

#[test]
fn unreachable_step_maps_to_e0303() {
    let diag = diagnostic_from_error(
        &ValidationError::UnreachableStep {
            step: "skipped".into(),
            span: Span::ZERO,
        },
        None,
    );
    assert_eq!(diag.code.code(), 0x0303);
    assert!(diag.message.contains("skipped"));
}

#[test]
fn secret_result_leak_maps_to_e0406() {
    let diag = diagnostic_from_error(
        &ValidationError::SecretResultLeak { span: Span::ZERO },
        None,
    );
    assert_eq!(diag.code.code(), 0x0406);
}

#[test]
fn type_mismatch_maps_to_e0407() {
    let diag = diagnostic_from_error(
        &ValidationError::TypeMismatch {
            expected: "boolean".into(),
            found: "number".into(),
            span: Span::ZERO,
        },
        None,
    );
    assert_eq!(diag.code.code(), 0x0407);
    assert!(diag.message.contains("boolean"));
    assert!(diag.message.contains("number"));
}

#[test]
fn duplicate_id_maps_to_e0109() {
    let diag = diagnostic_from_error(
        &ValidationError::DuplicateId {
            id: "step1".into(),
            span: Span::ZERO,
        },
        None,
    );
    assert_eq!(diag.code.code(), 0x0109);
}

#[test]
fn direct_runtime_maps_to_e0204() {
    let diag = diagnostic_from_error(
        &ValidationError::DirectRuntimeReference { span: Span::ZERO },
        None,
    );
    assert_eq!(diag.code.code(), 0x0204);
}

#[test]
fn error_code_returns_matching_code() {
    let code = error_code(&ValidationError::ControlFlowCycle { span: Span::ZERO });
    assert_eq!(code.code(), 0x0302);
}

#[test]
fn all_variants_produce_valid_diagnostic() {
    let errors = all_variants();
    for error in errors {
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.severity, Severity::Error);
    }
}

#[test]
fn all_variants_have_unique_diagnostic_codes() {
    let errors = all_variants();
    let mut seen = std::collections::BTreeSet::new();
    for error in errors {
        let code = error_code(&error).code();
        assert!(seen.insert(code), "duplicate diagnostic code {code:#06x}");
    }
}

// ---------------------------------------------------------------------------
// BDD exact-assertion tests
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_from_error_includes_error_code() {
    // Given a ValidationError::DuplicateKey { span: Span::ZERO }
    let error = ValidationError::DuplicateKey { span: Span::ZERO };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the diagnostic has code E0101
    assert_eq!(diag.code.code(), 0x0101);
}

#[test]
fn diagnostic_from_error_includes_message() {
    // Given a ValidationError::MissingRequiredField
    let error = ValidationError::MissingRequiredField {
        field: "steps".to_owned(),
        span: Span::ZERO,
    };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the message is non-empty and contains the field name
    assert!(!diag.message.is_empty());
    assert!(diag.message.contains("steps"));
}

#[test]
fn diagnostic_from_error_includes_location() {
    // Given any ValidationError
    let error = ValidationError::ControlFlowCycle { span: Span::ZERO };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the span is present (ZERO for now but always set)
    assert_eq!(diag.span, Span::ZERO);
}

#[test]
fn error_code_returns_known_code_for_duplicate_key() {
    // Given a ValidationError::DuplicateKey { span: Span::ZERO }
    let error = ValidationError::DuplicateKey { span: Span::ZERO };
    // When error_code is called
    let code = error_code(&error);
    // Then it returns code E0101 (0x0101)
    assert_eq!(code.code(), 0x0101);
}

#[test]
fn error_code_returns_known_code_for_missing_required_field() {
    // Given a ValidationError::MissingRequiredField
    let error = ValidationError::MissingRequiredField {
        field: "version".to_owned(),
        span: Span::ZERO,
    };
    // When error_code is called
    let code = error_code(&error);
    // Then it returns code E0105 (0x0105)
    assert_eq!(code.code(), 0x0105);
}

#[test]
fn error_code_is_non_empty_for_all_variants() {
    // Given all ValidationError variants
    let errors = all_variants();
    // When error_code is called for each
    // Then every variant produces a non-zero code
    for error in &errors {
        let code = error_code(error).code();
        assert_ne!(code, 0, "error_code returned 0 for {error:?}");
    }
}

#[test]
fn diagnostic_from_error_for_invalid_id_includes_id() {
    // Given a ValidationError::InvalidId
    let error = ValidationError::InvalidId {
        id: "bad-id".to_owned(),
        span: Span::ZERO,
    };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the message contains the id
    assert_eq!(diag.code.code(), 0x0107);
    assert!(diag.message.contains("bad-id"));
}

#[test]
fn diagnostic_from_error_for_reserved_id_includes_id() {
    // Given a ValidationError::ReservedId
    let error = ValidationError::ReservedId {
        id: "runtime".to_owned(),
        span: Span::ZERO,
    };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the message contains the id
    assert_eq!(diag.code.code(), 0x0108);
    assert!(diag.message.contains("runtime"));
}

#[test]
fn diagnostic_from_error_for_duplicate_id_includes_id() {
    // Given a ValidationError::DuplicateId
    let error = ValidationError::DuplicateId {
        id: "step1".to_owned(),
        span: Span::ZERO,
    };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the message contains the id
    assert_eq!(diag.code.code(), 0x0109);
    assert!(diag.message.contains("step1"));
}

#[test]
fn diagnostic_from_error_for_unknown_reference_includes_reference() {
    // Given a ValidationError::UnknownReference
    let error = ValidationError::UnknownReference {
        reference: "$input.missing".to_owned(),
        span: Span::ZERO,
    };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the message contains the reference
    assert_eq!(diag.code.code(), 0x0201);
    assert!(diag.message.contains("$input.missing"));
}

#[test]
fn diagnostic_from_error_for_future_reference_includes_reference() {
    // Given a ValidationError::FutureReference
    let error = ValidationError::FutureReference {
        reference: "$steps.build".to_owned(),
        span: Span::ZERO,
    };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the message contains the reference
    assert_eq!(diag.code.code(), 0x0202);
    assert!(diag.message.contains("$steps.build"));
}

#[test]
fn diagnostic_from_error_for_limit_exceeded_includes_resource() {
    // Given a ValidationError::LimitExceeded
    let error = ValidationError::LimitExceeded {
        resource: "max_steps".to_owned(),
        span: Span::ZERO,
    };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the message contains the resource
    assert_eq!(diag.code.code(), 0x040A);
    assert!(diag.message.contains("max_steps"));
}

#[test]
fn diagnostic_from_error_for_unsupported_trigger_includes_trigger() {
    // Given a ValidationError::UnsupportedTrigger
    let error = ValidationError::UnsupportedTrigger {
        trigger: "cron".to_owned(),
        span: Span::ZERO,
    };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the message contains the trigger
    assert_eq!(diag.code.code(), 0x040B);
    assert!(diag.message.contains("cron"));
}

#[test]
fn diagnostic_severity_is_always_error() {
    // Given all ValidationError variants
    let errors = all_variants();
    // When diagnostic_from_error is called for each
    // Then the severity is always Error
    for error in &errors {
        let diag = diagnostic_from_error(error, None);
        assert_eq!(
            diag.severity,
            Severity::Error,
            "wrong severity for {error:?}"
        );
    }
}

#[test]
fn diagnostic_from_error_for_type_mismatch_includes_both_types() {
    // Given a ValidationError::TypeMismatch
    let error = ValidationError::TypeMismatch {
        expected: "boolean".to_owned(),
        found: "number".to_owned(),
        span: Span::ZERO,
    };
    // When diagnostic_from_error is called
    let diag = diagnostic_from_error(&error, None);
    // Then the message contains both type names
    assert_eq!(diag.code.code(), 0x0407);
    assert!(diag.message.contains("boolean"));
    assert!(diag.message.contains("number"));
}

// -----------------------------------------------------------------------
// Span propagation exact-assertion tests (B61-B63)
// -----------------------------------------------------------------------

#[test]
fn diagnostic_from_error_propagates_enriched_span_exactly() {
    // B61: diagnostic_from_error propagates error.span into Diagnostic.span
    let enriched = Span::with_location(10, 20, 3, 5);
    let error = ValidationError::ControlFlowCycle { span: enriched };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, enriched);
    assert_eq!(diag.span.start, 10);
    assert_eq!(diag.span.end, 20);
    assert_eq!(diag.span.line, Some(3));
    assert_eq!(diag.span.column, Some(5));
    assert_eq!(diag.span.location(), Some((3, 5)));
}

#[test]
fn diagnostic_from_error_produces_zero_span_for_zero_span_error() {
    // B62: Span::ZERO propagates as Span::ZERO (backward compat)
    let error = ValidationError::DuplicateKey { span: Span::ZERO };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, Span::ZERO);
    assert_eq!(diag.span.start, 0);
    assert_eq!(diag.span.end, 0);
    assert!(diag.span.location().is_none());
}

#[test]
fn diagnostic_from_error_propagates_location_bearing_span() {
    // B63: location-bearing span preserves line and column in output
    let located = Span::with_location(0, 100, 42, 8);
    let error = ValidationError::InvalidId {
        id: "test-id".into(),
        span: located,
    };
    let diag = diagnostic_from_error(&error, None);

    assert_eq!(diag.span, located);
    assert_eq!(diag.span.line, Some(42));
    assert_eq!(diag.span.column, Some(8));
    assert_eq!(diag.span.location(), Some((42, 8)));
}

#[test]
fn diagnostic_from_error_all_variants_have_non_empty_message() {
    // B66: diagnostic_from_error produces non-empty message for all variants
    let errors = all_variants();
    for error in &errors {
        let diag = diagnostic_from_error(error, None);
        assert!(
            !diag.message.is_empty(),
            "empty message for variant: {error:?}"
        );
    }
}

#[test]
fn error_diagnostic_parts_is_exhaustive_over_all_validation_error_variants() {
    // B67: error_diagnostic_parts covers all ~55 variants exhaustively
    let errors = all_variants();
    for error in &errors {
        // Calling diagnostic_from_error must not panic (match exhaustiveness)
        let diag = diagnostic_from_error(error, None);
        // Every variant produces a valid diagnostic with code > 0
        assert_ne!(diag.code.code(), 0, "zero code for variant: {error:?}");
    }
}

#[test]
fn diagnostic_from_error_includes_variant_specific_data_in_message() {
    // B69: structured data fields appear in diagnostic message
    let error = ValidationError::InvalidId {
        id: "bad-id".into(),
        span: Span::ZERO,
    };
    let diag = diagnostic_from_error(&error, None);

    assert!(diag.message.contains("bad-id"));
}

#[test]
fn diagnostic_from_error_all_variants_produce_severity_error() {
    // B64: severity is Error for all validation error variants
    let errors = all_variants();
    for error in &errors {
        let diag = diagnostic_from_error(error, None);
        assert_eq!(
            diag.severity,
            Severity::Error,
            "wrong severity for {error:?}"
        );
    }
}
