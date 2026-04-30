//! Diagnostic conversion for validation errors.
//!
//! Converts `ValidationError` variants into stable `Diagnostic` records with
//! error codes matching the master contract (Section 16).

use crate::ValidationError;
use vb_core::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use vb_core::span::Span;

// ---------------------------------------------------------------------------
// Stable error codes (Section 16 of the master contract)
// ---------------------------------------------------------------------------

/// Schema validation errors: E01xx.
const CODE_DUPLICATE_KEY: u16 = 0x0101;
const CODE_FORBIDDEN_YAML_FEATURE: u16 = 0x0102;
const CODE_UNKNOWN_TOP_LEVEL_FIELD: u16 = 0x0103;
const CODE_UNKNOWN_STEP_FIELD: u16 = 0x0104;
const CODE_MISSING_REQUIRED_FIELD: u16 = 0x0105;
const CODE_INVALID_VERSION: u16 = 0x0106;
const CODE_INVALID_ID: u16 = 0x0107;
const CODE_RESERVED_ID: u16 = 0x0108;
const CODE_DUPLICATE_ID: u16 = 0x0109;

/// Reference validation errors: E02xx.
const CODE_UNKNOWN_REFERENCE: u16 = 0x0201;
const CODE_FUTURE_REFERENCE: u16 = 0x0202;
const CODE_SECRET_NOT_DECLARED: u16 = 0x0203;
const CODE_DIRECT_RUNTIME_REFERENCE: u16 = 0x0204;

/// Control-flow errors: E03xx.
const CODE_INVALID_THEN_TARGET: u16 = 0x0301;
const CODE_CONTROL_FLOW_CYCLE: u16 = 0x0302;
const CODE_UNREACHABLE_STEP: u16 = 0x0303;
const CODE_INVALID_CHOOSE: u16 = 0x0304;
const CODE_INVALID_FOR_EACH: u16 = 0x0305;
const CODE_INVALID_TOGETHER: u16 = 0x0306;
const CODE_INVALID_COLLECT: u16 = 0x0307;
const CODE_INVALID_REDUCE: u16 = 0x0308;
const CODE_INVALID_REPEAT: u16 = 0x0309;

/// Type/taint/resource errors: E04xx.
const CODE_INVALID_WAIT: u16 = 0x0401;
const CODE_INVALID_ASK: u16 = 0x0402;
const CODE_INVALID_FINISH: u16 = 0x0403;
const CODE_INVALID_RETRY: u16 = 0x0404;
const CODE_INVALID_ON_ERROR: u16 = 0x0405;
const CODE_SECRET_RESULT_LEAK: u16 = 0x0406;
const CODE_TYPE_MISMATCH: u16 = 0x0407;
const CODE_PAYLOAD_TOO_LARGE: u16 = 0x0408;
const CODE_LIMIT_REQUIRED: u16 = 0x0409;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Converts a validation error into a diagnostic record.
pub fn diagnostic_from_error(error: &ValidationError) -> Diagnostic {
    let (code, message) = error_diagnostic_parts(error);
    Diagnostic::new(code, message.into(), Severity::Error, Span::ZERO)
}

/// Returns the stable diagnostic code for a validation error.
pub fn error_code(error: &ValidationError) -> DiagnosticCode {
    let (code, _) = error_diagnostic_parts(error);
    code
}

// ---------------------------------------------------------------------------
// Internal mapping
// ---------------------------------------------------------------------------

fn error_diagnostic_parts(error: &ValidationError) -> (DiagnosticCode, String) {
    match error {
        ValidationError::DuplicateKey => {
            (DiagnosticCode::new(CODE_DUPLICATE_KEY), "duplicate key".into())
        }
        ValidationError::ForbiddenYamlFeature => {
            (DiagnosticCode::new(CODE_FORBIDDEN_YAML_FEATURE), "forbidden YAML feature".into())
        }
        ValidationError::UnknownTopLevelField => {
            (DiagnosticCode::new(CODE_UNKNOWN_TOP_LEVEL_FIELD), "unknown top-level field".into())
        }
        ValidationError::UnknownStepField => {
            (DiagnosticCode::new(CODE_UNKNOWN_STEP_FIELD), "unknown step field".into())
        }
        ValidationError::MissingRequiredField { field } => {
            (DiagnosticCode::new(CODE_MISSING_REQUIRED_FIELD), format!("missing required field: {field}"))
        }
        ValidationError::InvalidVersion { version } => {
            (DiagnosticCode::new(CODE_INVALID_VERSION), format!("invalid version: {version}"))
        }
        ValidationError::InvalidId { id } => {
            (DiagnosticCode::new(CODE_INVALID_ID), format!("invalid ID: {id}"))
        }
        ValidationError::ReservedId { id } => {
            (DiagnosticCode::new(CODE_RESERVED_ID), format!("reserved ID: {id}"))
        }
        ValidationError::DuplicateId { id } => {
            (DiagnosticCode::new(CODE_DUPLICATE_ID), format!("duplicate ID: {id}"))
        }
        ValidationError::MultipleStepPrimitives => {
            (DiagnosticCode::new(CODE_UNKNOWN_STEP_FIELD), "multiple step primitives".into())
        }
        ValidationError::MissingStepPrimitive => {
            (DiagnosticCode::new(CODE_UNKNOWN_STEP_FIELD), "missing step primitive".into())
        }
        ValidationError::UnknownReference { reference } => {
            (DiagnosticCode::new(CODE_UNKNOWN_REFERENCE), format!("unknown reference: {reference}"))
        }
        ValidationError::FutureReference { reference } => {
            (DiagnosticCode::new(CODE_FUTURE_REFERENCE), format!("future reference: {reference}"))
        }
        ValidationError::SecretNotDeclared { secret } => {
            (DiagnosticCode::new(CODE_SECRET_NOT_DECLARED), format!("secret not declared: {secret}"))
        }
        ValidationError::DirectRuntimeReference => {
            (DiagnosticCode::new(CODE_DIRECT_RUNTIME_REFERENCE), "direct runtime reference".into())
        }
        ValidationError::InvalidThenTarget => {
            (DiagnosticCode::new(CODE_INVALID_THEN_TARGET), "invalid then target".into())
        }
        ValidationError::ControlFlowCycle => {
            (DiagnosticCode::new(CODE_CONTROL_FLOW_CYCLE), "control-flow cycle".into())
        }
        ValidationError::UnreachableStep { step } => {
            (DiagnosticCode::new(CODE_UNREACHABLE_STEP), format!("unreachable step: {step}"))
        }
        ValidationError::InvalidChoose => {
            (DiagnosticCode::new(CODE_INVALID_CHOOSE), "invalid choose".into())
        }
        ValidationError::InvalidForEach => {
            (DiagnosticCode::new(CODE_INVALID_FOR_EACH), "invalid for_each".into())
        }
        ValidationError::InvalidTogether => {
            (DiagnosticCode::new(CODE_INVALID_TOGETHER), "invalid together".into())
        }
        ValidationError::InvalidCollect => {
            (DiagnosticCode::new(CODE_INVALID_COLLECT), "invalid collect".into())
        }
        ValidationError::InvalidReduce => {
            (DiagnosticCode::new(CODE_INVALID_REDUCE), "invalid reduce".into())
        }
        ValidationError::InvalidRepeat => {
            (DiagnosticCode::new(CODE_INVALID_REPEAT), "invalid repeat".into())
        }
        ValidationError::InvalidWait => {
            (DiagnosticCode::new(CODE_INVALID_WAIT), "invalid wait".into())
        }
        ValidationError::InvalidAsk => {
            (DiagnosticCode::new(CODE_INVALID_ASK), "invalid ask".into())
        }
        ValidationError::InvalidFinish => {
            (DiagnosticCode::new(CODE_INVALID_FINISH), "invalid finish".into())
        }
        ValidationError::InvalidRetry => {
            (DiagnosticCode::new(CODE_INVALID_RETRY), "invalid retry".into())
        }
        ValidationError::InvalidOnError => {
            (DiagnosticCode::new(CODE_INVALID_ON_ERROR), "invalid on_error".into())
        }
        ValidationError::SecretResultLeak => {
            (DiagnosticCode::new(CODE_SECRET_RESULT_LEAK), "secret result leak".into())
        }
        ValidationError::TypeMismatch { expected, found } => {
            (DiagnosticCode::new(CODE_TYPE_MISMATCH), format!("type mismatch: expected {expected}, found {found}"))
        }
        ValidationError::PayloadTooLarge => {
            (DiagnosticCode::new(CODE_PAYLOAD_TOO_LARGE), "payload too large".into())
        }
        ValidationError::LimitRequired { resource } => {
            (DiagnosticCode::new(CODE_LIMIT_REQUIRED), format!("limit required: {resource}"))
        }
        ValidationError::LimitExceeded { resource } => {
            (DiagnosticCode::new(CODE_PAYLOAD_TOO_LARGE), format!("limit exceeded: {resource}"))
        }
        ValidationError::UnsupportedTrigger { trigger } => {
            (DiagnosticCode::new(CODE_MISSING_REQUIRED_FIELD), format!("unsupported trigger: {trigger}"))
        }
        ValidationError::HttpTriggerOutOfCore => {
            (DiagnosticCode::new(CODE_FORBIDDEN_YAML_FEATURE), "HTTP trigger out of core".into())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_key_maps_to_e0101() {
        let diag = diagnostic_from_error(&ValidationError::DuplicateKey);
        assert_eq!(diag.code.code(), 0x0101);
        assert_eq!(diag.severity, Severity::Error);
    }

    #[test]
    fn invalid_version_maps_to_e0106() {
        let diag = diagnostic_from_error(&ValidationError::InvalidVersion {
            version: "v2".into(),
        });
        assert_eq!(diag.code.code(), 0x0106);
        assert!(diag.message.contains("v2"));
    }

    #[test]
    fn unknown_reference_maps_to_e0201() {
        let diag = diagnostic_from_error(&ValidationError::UnknownReference {
            reference: "$input.missing".into(),
        });
        assert_eq!(diag.code.code(), 0x0201);
    }

    #[test]
    fn control_flow_cycle_maps_to_e0302() {
        let diag = diagnostic_from_error(&ValidationError::ControlFlowCycle);
        assert_eq!(diag.code.code(), 0x0302);
    }

    #[test]
    fn unreachable_step_maps_to_e0303() {
        let diag = diagnostic_from_error(&ValidationError::UnreachableStep {
            step: "skipped".into(),
        });
        assert_eq!(diag.code.code(), 0x0303);
        assert!(diag.message.contains("skipped"));
    }

    #[test]
    fn secret_result_leak_maps_to_e0406() {
        let diag = diagnostic_from_error(&ValidationError::SecretResultLeak);
        assert_eq!(diag.code.code(), 0x0406);
    }

    #[test]
    fn type_mismatch_maps_to_e0407() {
        let diag = diagnostic_from_error(&ValidationError::TypeMismatch {
            expected: "boolean".into(),
            found: "number".into(),
        });
        assert_eq!(diag.code.code(), 0x0407);
        assert!(diag.message.contains("boolean"));
        assert!(diag.message.contains("number"));
    }

    #[test]
    fn duplicate_id_maps_to_e0109() {
        let diag = diagnostic_from_error(&ValidationError::DuplicateId {
            id: "step1".into(),
        });
        assert_eq!(diag.code.code(), 0x0109);
    }

    #[test]
    fn direct_runtime_maps_to_e0204() {
        let diag = diagnostic_from_error(&ValidationError::DirectRuntimeReference);
        assert_eq!(diag.code.code(), 0x0204);
    }

    #[test]
    fn error_code_returns_matching_code() {
        let code = error_code(&ValidationError::ControlFlowCycle);
        assert_eq!(code.code(), 0x0302);
    }

    #[test]
    fn all_variants_produce_valid_diagnostic() {
        let errors = all_variants();
        for error in errors {
            let diag = diagnostic_from_error(&error);
            assert_eq!(diag.severity, Severity::Error);
        }
    }

    fn all_variants() -> Vec<ValidationError> {
        vec![
            ValidationError::DuplicateKey,
            ValidationError::ForbiddenYamlFeature,
            ValidationError::UnknownTopLevelField,
            ValidationError::UnknownStepField,
            ValidationError::MissingRequiredField { field: "test".into() },
            ValidationError::InvalidVersion { version: "v0".into() },
            ValidationError::InvalidId { id: "BAD".into() },
            ValidationError::ReservedId { id: "runtime".into() },
            ValidationError::DuplicateId { id: "dup".into() },
            ValidationError::MultipleStepPrimitives,
            ValidationError::MissingStepPrimitive,
            ValidationError::UnknownReference { reference: "$x".into() },
            ValidationError::FutureReference { reference: "$steps.s".into() },
            ValidationError::SecretNotDeclared { secret: "tok".into() },
            ValidationError::DirectRuntimeReference,
            ValidationError::InvalidThenTarget,
            ValidationError::ControlFlowCycle,
            ValidationError::UnreachableStep { step: "s".into() },
            ValidationError::InvalidChoose,
            ValidationError::InvalidForEach,
            ValidationError::InvalidTogether,
            ValidationError::InvalidCollect,
            ValidationError::InvalidReduce,
            ValidationError::InvalidRepeat,
            ValidationError::InvalidWait,
            ValidationError::InvalidAsk,
            ValidationError::InvalidFinish,
            ValidationError::InvalidRetry,
            ValidationError::InvalidOnError,
            ValidationError::SecretResultLeak,
            ValidationError::TypeMismatch { expected: "a".into(), found: "b".into() },
            ValidationError::PayloadTooLarge,
            ValidationError::LimitRequired { resource: "r".into() },
            ValidationError::LimitExceeded { resource: "r".into() },
            ValidationError::UnsupportedTrigger { trigger: "cron".into() },
            ValidationError::HttpTriggerOutOfCore,
        ]
    }
}
