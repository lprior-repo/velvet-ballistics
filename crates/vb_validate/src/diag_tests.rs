#![forbid(unsafe_code)]
//! BDD exact-assertion tests for diagnostic conversion.
//!
//! Verifies exact error code and message mapping fidelity.

#![allow(unreachable_pub)]
#[cfg(test)]
mod tests {
    use crate::ValidationError;
    use crate::diag_convert::all_variants;
    use crate::diag_render::{diagnostic_from_error, error_code};
    use vb_core::diagnostic::Severity;
    use vb_core::span::Span;

    #[test]
    fn diagnostic_from_error_includes_error_code() {
        let error = ValidationError::DuplicateKey;
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.code.code(), 0x0101);
    }

    #[test]
    fn diagnostic_from_error_includes_message() {
        let error = ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        };
        let diag = diagnostic_from_error(&error);
        assert!(!diag.message.is_empty());
        assert!(diag.message.contains("steps"));
    }

    #[test]
    fn diagnostic_from_error_includes_location() {
        let error = ValidationError::ControlFlowCycle;
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.span, Span::ZERO);
    }

    #[test]
    fn error_code_returns_known_code_for_duplicate_key() {
        let error = ValidationError::DuplicateKey;
        let code = error_code(&error);
        assert_eq!(code.code(), 0x0101);
    }

    #[test]
    fn error_code_returns_known_code_for_missing_required_field() {
        let error = ValidationError::MissingRequiredField {
            field: "version".to_owned(),
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
        };
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.code.code(), 0x0107);
        assert!(diag.message.contains("bad-id"));
    }

    #[test]
    fn diagnostic_from_error_for_reserved_id_includes_id() {
        let error = ValidationError::ReservedId {
            id: "runtime".to_owned(),
        };
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.code.code(), 0x0108);
        assert!(diag.message.contains("runtime"));
    }

    #[test]
    fn diagnostic_from_error_for_duplicate_id_includes_id() {
        let error = ValidationError::DuplicateId {
            id: "step1".to_owned(),
        };
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.code.code(), 0x0109);
        assert!(diag.message.contains("step1"));
    }

    #[test]
    fn diagnostic_from_error_for_unknown_reference_includes_reference() {
        let error = ValidationError::UnknownReference {
            reference: "$input.missing".to_owned(),
        };
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.code.code(), 0x0201);
        assert!(diag.message.contains("$input.missing"));
    }

    #[test]
    fn diagnostic_from_error_for_future_reference_includes_reference() {
        let error = ValidationError::FutureReference {
            reference: "$steps.build".to_owned(),
        };
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.code.code(), 0x0202);
        assert!(diag.message.contains("$steps.build"));
    }

    #[test]
    fn diagnostic_from_error_for_limit_exceeded_includes_resource() {
        let error = ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        };
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.code.code(), 0x040A);
        assert!(diag.message.contains("max_steps"));
    }

    #[test]
    fn diagnostic_from_error_for_unsupported_trigger_includes_trigger() {
        let error = ValidationError::UnsupportedTrigger {
            trigger: "cron".to_owned(),
        };
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.code.code(), 0x040B);
        assert!(diag.message.contains("cron"));
    }

    #[test]
    fn diagnostic_severity_is_always_error() {
        for error in all_variants() {
            let diag = diagnostic_from_error(&error);
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
        };
        let diag = diagnostic_from_error(&error);
        assert_eq!(diag.code.code(), 0x0407);
        assert!(diag.message.contains("boolean"));
        assert!(diag.message.contains("number"));
    }
}
