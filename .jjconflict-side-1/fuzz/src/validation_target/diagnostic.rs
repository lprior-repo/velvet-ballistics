//! Diagnostic rendering fuzz target bodies.

pub fn fuzz_diagnostic_from_error(data: &[u8]) {
    use vb_validate::ValidationError;
    use vb_validate::diagnostic::diagnostic_from_error;

    let Ok(payload) = std::str::from_utf8(data) else {
        return;
    };
    let field = if payload.is_empty() { "fuzz" } else { payload };
    let errors: [ValidationError; 16] = [
        ValidationError::DuplicateKey,
        ValidationError::ForbiddenYamlFeature,
        ValidationError::UnknownTopLevelField,
        ValidationError::UnknownStepField,
        ValidationError::MultipleStepPrimitives,
        ValidationError::MissingStepPrimitive,
        ValidationError::DirectRuntimeReference,
        ValidationError::InvalidThenTarget,
        ValidationError::ControlFlowCycle,
        ValidationError::SecretResultLeak,
        ValidationError::PayloadTooLarge,
        ValidationError::HttpTriggerOutOfCore,
        ValidationError::MissingRequiredField {
            field: field.into(),
        },
        ValidationError::InvalidId { id: field.into() },
        ValidationError::TypeMismatch {
            expected: "bool".into(),
            found: field.into(),
        },
        ValidationError::LimitExceeded {
            resource: field.into(),
        },
    ];
    for error in &errors {
        let diag = diagnostic_from_error(error);
        assert!(!diag.message.is_empty());
        assert_ne!(diag.numeric_code.code(), 0);
    }
}

pub fn fuzz_diagnostic_code_from_str(data: &[u8]) {
    use std::str::FromStr;
    use vb_core::diagnostic::DiagnosticCode;

    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };
    if let Ok(code) = DiagnosticCode::from_str(input) {
        let display = code.to_string();
        assert!(display.starts_with('E'));
        assert_eq!(display.len(), 5);
    }
}
