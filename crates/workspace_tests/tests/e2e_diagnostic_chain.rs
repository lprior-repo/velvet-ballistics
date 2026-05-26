//! End-to-end behavior tests: full diagnostic chain.
//!
//! E2E scenarios:
//!   - YAML parse error → YamlError → SymbolicCode → Display
//!   - Validation error → Diagnostic → symbolic code lookup
//!   - Error → code → as_str → Display round-trip

use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};
use vb_core::errors::CoreError;
use vb_core::ids::StepIdx;
use vb_runtime::RuntimeError;
use vb_storage::JournalError;
use vb_validate::ValidationError;
use vb_yaml::YamlError;

// ---------------------------------------------------------------------------
// E2E: YamlError → code() → SymbolicCode → as_str → Display
// ---------------------------------------------------------------------------

#[test]
fn e2e_yaml_duplicate_key_chain() {
    // Given: invalid YAML with duplicate keys produces YamlError::DuplicateKey
    let error = YamlError::DuplicateKey {
        key: Box::<str>::from("steps"),
    };

    // When: we convert to SymbolicCode
    let code = error.code();

    // Then: the chain is consistent
    assert_eq!(code.as_str(), "DUPLICATE_KEY");
    assert_eq!(format!("{code}"), "DUPLICATE_KEY");

    // And: the symbolic code is registered and reconstuctible
    let reconstructed = SymbolicCode::from_static(code.as_str());
    assert!(reconstructed.is_some(), "DUPLICATE_KEY must be registered");
    assert_eq!(reconstructed.unwrap(), code);
}

#[test]
fn e2e_yaml_forbidden_feature_chain() {
    let error = YamlError::ForbiddenFeature { detail: "anchor" };
    let code = error.code();
    assert_eq!(code.as_str(), "FORBIDDEN_YAML_FEATURE");
    assert_eq!(format!("{code}"), "FORBIDDEN_YAML_FEATURE");
}

#[test]
fn e2e_yaml_missing_required_field_chain() {
    let error = YamlError::EmptySource;
    let code = error.code();
    assert_eq!(code.as_str(), "MISSING_REQUIRED_FIELD");
}

#[test]
fn e2e_yaml_type_mismatch_chain() {
    let error = YamlError::FieldShape {
        field: "steps",
        expected: "array",
    };
    let code = error.code();
    assert_eq!(code.as_str(), "TYPE_MISMATCH");
}

#[test]
fn e2e_yaml_payload_too_large_chain() {
    let error = YamlError::SourceTooLarge {
        size: 10_000_000,
        max: 1_000_000,
    };
    let code = error.code();
    assert_eq!(code.as_str(), "PAYLOAD_TOO_LARGE");
}

#[test]
fn e2e_yaml_limit_exceeded_chain() {
    let error = YamlError::NestingTooDeep { depth: 65, max: 64 };
    let code = error.code();
    assert_eq!(code.as_str(), "LIMIT_EXCEEDED");
}

// ---------------------------------------------------------------------------
// E2E: ValidationError → Diagnostic → symbolic code lookup
// ---------------------------------------------------------------------------

#[test]
fn e2e_validation_error_to_diagnostic_chain() {
    // Given: a ValidationError
    let error = &ValidationError::DuplicateKey;

    // When: we convert to Diagnostic via vb_validate's diagnostic pipeline
    use vb_core::diagnostic::DiagnosticCode;
    let dc: DiagnosticCode = vb_validate::diagnostic::error_code(error);

    // Then: the numeric code matches the registry
    assert_eq!(dc.code(), 0x0101);

    // And: the symbolic lookup works
    let symbolic = dc.symbolic_code();
    assert!(symbolic.is_some());
    assert_eq!(symbolic.unwrap().as_str(), "DUPLICATE_KEY");
}

#[test]
fn e2e_validation_error_code_type_mismatch_chain() {
    let error = &ValidationError::TypeMismatch {
        expected: "integer".into(),
        found: "string".into(),
    };
    let dc = vb_validate::diagnostic::error_code(error);
    assert_eq!(dc.code(), 0x0407);
    assert_eq!(dc.symbolic_code().unwrap().as_str(), "TYPE_MISMATCH");
}

#[test]
fn e2e_validation_error_code_diagnostic_full_chain() {
    let error = &ValidationError::MissingRequiredField {
        field: "steps".into(),
    };
    use vb_core::diagnostic::Diagnostic;
    let diagnostic: Diagnostic = vb_validate::diagnostic::diagnostic_from_error(error);

    assert_eq!(diagnostic.code.code(), 0x0105);
    // Diagnostic message preserves the human-readable error text
    assert!(
        diagnostic
            .message
            .as_ref()
            .contains("missing required field")
    );
    assert!(diagnostic.message.as_ref().contains("steps"));
    assert_eq!(diagnostic.severity, vb_core::diagnostic::Severity::Error);
}

// ---------------------------------------------------------------------------
// E2E: CoreError → HasSymbolicCode → SymbolicCode → Display
// ---------------------------------------------------------------------------

#[test]
fn e2e_core_error_chain() {
    let error = CoreError::InvalidProgramCounter {
        step: StepIdx::new(42),
    };
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "INVALID_PROGRAM_COUNTER");
    assert_eq!(format!("{code}"), "INVALID_PROGRAM_COUNTER");

    // Verify we can reconstruct from string
    let reconstructed = SymbolicCode::from_static(code.as_str());
    assert!(reconstructed.is_some());
    assert_eq!(reconstructed.unwrap(), code);
}

// ---------------------------------------------------------------------------
// E2E: RuntimeError → HasSymbolicCode → SymbolicCode → Display
// ---------------------------------------------------------------------------

#[test]
fn e2e_runtime_error_chain() {
    let error = RuntimeError::RunNotFound;
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "RUN_NOT_FOUND");
}

// ---------------------------------------------------------------------------
// E2E: JournalError → HasSymbolicCode → SymbolicCode → Display
// ---------------------------------------------------------------------------

#[test]
fn e2e_journal_error_chain() {
    let error = JournalError::KeyCapacity;
    let code = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "KEY_CAPACITY_EXCEEDED");
}

// ---------------------------------------------------------------------------
// E2E: Full traceability — SymbolicCode → numeric → SymbolicCode round-trip
// ---------------------------------------------------------------------------

#[test]
fn e2e_symbolic_to_numeric_to_symbolic_round_trip() {
    // For every unique symbolic name in the registry, verify
    // symbolic → numeric → symbolic round-trip works.
    use vb_core::diagnostic::{CODE_REGISTRY, numeric_to_symbolic};

    let mut seen: Vec<&str> = Vec::new();
    for entry in CODE_REGISTRY {
        if seen.contains(&entry.symbolic) {
            continue;
        }
        seen.push(entry.symbolic);

        let code = SymbolicCode::from_static(entry.symbolic).expect("registered");
        let num = code.numeric_code();
        let back = numeric_to_symbolic(num);
        assert!(
            back.is_some(),
            "numeric_to_symbolic({:#06X}) should return Some for '{}'",
            num,
            entry.symbolic
        );

        // The reverse lookup may return a different symbolic name if there
        // are cross-category duplicates. Verify that from_static works.
        let back_str = back.unwrap().as_str();
        let reconstructed = SymbolicCode::from_static(back_str);
        assert!(
            reconstructed.is_some(),
            "from_static('{back_str}') should return Some"
        );
    }
}

// ---------------------------------------------------------------------------
// E2E: DiagnosticCode Display formats correctly across all categories
// ---------------------------------------------------------------------------

#[test]
fn e2e_diagnostic_code_display_formats_correctly() {
    use core::str::FromStr;
    use vb_core::diagnostic::DiagnosticCode;

    // Sample from each category range
    let inputs: &[(&str, &str)] = &[
        ("E0101", "E0101"), // Schema
        ("E0203", "E0203"), // Reference
        ("E0305", "E0305"), // Control Flow
        ("E0407", "E0407"), // Type/Taint
        ("E0501", "E0501"), // Gate (new)
        ("E0601", "E0601"), // Contract Discovery (new)
        ("E1001", "E1001"), // Compilation
        ("E1102", "E1102"), // Workflow IR
        ("E1201", "E1201"), // Expression
        ("E1304", "E1304"), // Accessor
        ("E1406", "E1406"), // Lowering
        ("E1506", "E1506"), // Lifecycle
        ("E2001", "E2001"), // Storage
        ("E3001", "E3001"), // Runtime
        ("E401C", "E401C"), // Runtime Boundary (extended)
    ];

    for (input, expected) in inputs {
        let dc = DiagnosticCode::from_str(input).expect(input);
        assert_eq!(
            format!("{dc}"),
            *expected,
            "Display for {input} should be {expected}"
        );
    }
}
