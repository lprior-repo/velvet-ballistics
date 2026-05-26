#![forbid(unsafe_code)]
//! BDD exact-assertion tests for diagnostic conversion.
//!
//! Verifies exact error code, message fidelity, and span propagation.

#![allow(unreachable_pub)]
#[cfg(test)]
mod tests {
    use crate::ValidationError;
    use crate::diag_convert::all_variants;
    // Canonical diagnostic functions (diag_render now re-exports from diagnostic).
    use crate::diag_render::{diagnostic_from_error, error_code};
    use vb_core::diagnostic::Severity;
    use vb_core::span::Span;

    #[test]
    fn diagnostic_from_error_includes_error_code() {
<<<<<<< HEAD
        let error = ValidationError::DuplicateKey;
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.numeric_code.code(), 0x0101);
=======
        let error = ValidationError::DuplicateKey { span: Span::ZERO };
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.code.code(), 0x0101);
>>>>>>> landing/vb-xi2f.9
    }

    #[test]
    fn diagnostic_from_error_includes_message() {
        let error = ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
            span: Span::ZERO,
        };
        let diag = diagnostic_from_error(&error, None);
        assert!(!diag.message.is_empty());
        assert!(diag.message.contains("steps"));
    }

    #[test]
    fn diagnostic_from_error_includes_location() {
        let error = ValidationError::ControlFlowCycle { span: Span::ZERO };
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.span, Span::ZERO);
    }

    #[test]
    fn diagnostic_from_error_propagates_non_zero_span() {
        let input_span = Span::new(10, 50);
        let error = ValidationError::InvalidThenTarget { span: input_span };
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.span, input_span);
        assert_eq!(diag.span.start, 10);
        assert_eq!(diag.span.end, 50);
    }

    #[test]
    fn error_code_returns_known_code_for_duplicate_key() {
        let error = ValidationError::DuplicateKey { span: Span::ZERO };
        let code = error_code(&error);
        assert_eq!(code.code(), 0x0101);
    }

    #[test]
    fn error_code_returns_known_code_for_missing_required_field() {
        let error = ValidationError::MissingRequiredField {
            field: "version".to_owned(),
            span: Span::ZERO,
        };
        let code = error_code(&error);
        assert_eq!(code.code(), 0x0105);
    }

    #[test]
    fn error_code_is_non_empty_for_all_variants() {
        for error in &all_variants() {
            let code = error_code(error).code();
            assert_ne!(code, 0, "error_code returned 0 for {error:?}");
        }
    }

    #[test]
    fn diagnostic_from_error_for_invalid_id_includes_id() {
        let error = ValidationError::InvalidId {
            id: "bad-id".to_owned(),
            span: Span::ZERO,
        };
<<<<<<< HEAD
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.numeric_code.code(), 0x0107);
=======
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.code.code(), 0x0107);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("bad-id"));
    }

    #[test]
    fn diagnostic_from_error_for_reserved_id_includes_id() {
        let error = ValidationError::ReservedId {
            id: "runtime".to_owned(),
            span: Span::ZERO,
        };
<<<<<<< HEAD
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.numeric_code.code(), 0x0108);
=======
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.code.code(), 0x0108);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("runtime"));
    }

    #[test]
    fn diagnostic_from_error_for_duplicate_id_includes_id() {
        let error = ValidationError::DuplicateId {
            id: "step1".to_owned(),
            span: Span::ZERO,
        };
<<<<<<< HEAD
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.numeric_code.code(), 0x0109);
=======
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.code.code(), 0x0109);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("step1"));
    }

    #[test]
    fn diagnostic_from_error_for_unknown_reference_includes_reference() {
        let error = ValidationError::UnknownReference {
            reference: "$input.missing".to_owned(),
            span: Span::ZERO,
        };
<<<<<<< HEAD
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.numeric_code.code(), 0x0201);
=======
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.code.code(), 0x0201);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("$input.missing"));
    }

    #[test]
    fn diagnostic_from_error_for_future_reference_includes_reference() {
        let error = ValidationError::FutureReference {
            reference: "$steps.build".to_owned(),
            span: Span::ZERO,
        };
<<<<<<< HEAD
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.numeric_code.code(), 0x0202);
=======
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.code.code(), 0x0202);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("$steps.build"));
    }

    #[test]
    fn diagnostic_from_error_for_limit_exceeded_includes_resource() {
        let error = ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
            span: Span::ZERO,
        };
<<<<<<< HEAD
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.numeric_code.code(), 0x040A);
=======
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.code.code(), 0x040A);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("max_steps"));
    }

    #[test]
    fn diagnostic_from_error_for_unsupported_trigger_includes_trigger() {
        let error = ValidationError::UnsupportedTrigger {
            trigger: "cron".to_owned(),
            span: Span::ZERO,
        };
<<<<<<< HEAD
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.numeric_code.code(), 0x040B);
=======
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.code.code(), 0x040B);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("cron"));
    }

    #[test]
    fn diagnostic_severity_is_always_error() {
        for error in all_variants() {
            let diag = diagnostic_from_error(&error, None);
            assert_eq!(
                diag.severity,
                Severity::Error,
                "wrong severity for {error:?}"
            );
        }
    }

    #[test]
    fn diagnostic_from_error_for_type_mismatch_includes_both_types() {
        let error = ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
            span: Span::ZERO,
        };
<<<<<<< HEAD
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.numeric_code.code(), 0x0407);
=======
        let diag = diagnostic_from_error(&error, None);
        assert_eq!(diag.code.code(), 0x0407);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("boolean"));
        assert!(diag.message.contains("number"));
    }
}
