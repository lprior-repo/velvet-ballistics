#![forbid(unsafe_code)]
//! Tests for diagnostic rendering (construction + mapping).
//!
//! Uses `all_variants()` from `diag_convert` for exhaustive coverage.

#![allow(unreachable_pub)]
use super::construction::{diagnostic_from_error, error_code};
use super::mapping::error_diagnostic_parts;
use crate::ValidationError;
use vb_core::diagnostic::Severity;
use vb_core::span::Span;

// ---------------------------------------------------------------------------
// Construction tests — diagnostic_from_error
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_from_error_returns_error_severity() {
    let diag = diagnostic_from_error(&ValidationError::DuplicateKey);
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn diagnostic_from_error_returns_zero_span() {
    let diag = diagnostic_from_error(&ValidationError::ControlFlowCycle);
    assert_eq!(diag.span, Span::ZERO);
}

#[test]
fn diagnostic_from_error_message_is_non_empty_for_all_variants() {
    for error in crate::diag_convert::all_variants() {
        let diag = diagnostic_from_error(&error);
        assert!(!diag.message.is_empty(), "empty message for {error:?}");
    }
}

#[test]
fn error_code_is_non_zero_for_all_variants() {
    for error in crate::diag_convert::all_variants() {
        let code = error_code(&error).code();
        assert_ne!(code, 0, "zero code for {error:?}");
    }
}

#[test]
fn error_code_is_unique_for_all_variants() {
    use std::collections::BTreeSet;
    let mut seen = BTreeSet::new();
    for error in crate::diag_convert::all_variants() {
        let code = error_code(&error).code();
        assert!(seen.insert(code), "duplicate code {code:#06x}");
    }
}

// ---------------------------------------------------------------------------
// Field-value message tests
// ---------------------------------------------------------------------------

#[test]
fn missing_required_field_message_contains_field_name() {
    let diag = diagnostic_from_error(&ValidationError::MissingRequiredField {
        field: "steps".to_owned(),
    });
    assert!(diag.message.contains("steps"));
}

#[test]
fn invalid_version_message_contains_version() {
    let diag = diagnostic_from_error(&ValidationError::InvalidVersion {
        version: "bad/v2".to_owned(),
    });
    assert!(diag.message.contains("bad/v2"));
}

#[test]
fn invalid_id_message_contains_id() {
    let diag = diagnostic_from_error(&ValidationError::InvalidId {
        id: "123bad".to_owned(),
    });
    assert!(diag.message.contains("123bad"));
}

#[test]
fn reserved_id_message_contains_id() {
    let diag = diagnostic_from_error(&ValidationError::ReservedId {
        id: "runtime".to_owned(),
    });
    assert!(diag.message.contains("runtime"));
}

#[test]
fn duplicate_id_message_contains_id() {
    let diag = diagnostic_from_error(&ValidationError::DuplicateId {
        id: "dup_step".to_owned(),
    });
    assert!(diag.message.contains("dup_step"));
}

#[test]
fn unknown_reference_message_contains_reference() {
    let diag = diagnostic_from_error(&ValidationError::UnknownReference {
        reference: "$input.x".to_owned(),
    });
    assert!(diag.message.contains("$input.x"));
}

#[test]
fn future_reference_message_contains_reference() {
    let diag = diagnostic_from_error(&ValidationError::FutureReference {
        reference: "$steps.later".to_owned(),
    });
    assert!(diag.message.contains("$steps.later"));
}

#[test]
fn secret_not_declared_message_contains_secret() {
    let diag = diagnostic_from_error(&ValidationError::SecretNotDeclared {
        secret: "api_key".to_owned(),
    });
    assert!(diag.message.contains("api_key"));
}

#[test]
fn unreachable_step_message_contains_step() {
    let diag = diagnostic_from_error(&ValidationError::UnreachableStep {
        step: "orphan".to_owned(),
    });
    assert!(diag.message.contains("orphan"));
}

#[test]
fn type_mismatch_message_contains_both_types() {
    let diag = diagnostic_from_error(&ValidationError::TypeMismatch {
        expected: "boolean".to_owned(),
        found: "number".to_owned(),
    });
    assert!(diag.message.contains("boolean"));
    assert!(diag.message.contains("number"));
}

#[test]
fn limit_required_message_contains_resource() {
    let diag = diagnostic_from_error(&ValidationError::LimitRequired {
        resource: "max_slots".to_owned(),
    });
    assert!(diag.message.contains("max_slots"));
}

#[test]
fn limit_exceeded_message_contains_resource() {
    let diag = diagnostic_from_error(&ValidationError::LimitExceeded {
        resource: "max_steps".to_owned(),
    });
    assert!(diag.message.contains("max_steps"));
}

#[test]
fn unsupported_trigger_message_contains_trigger() {
    let diag = diagnostic_from_error(&ValidationError::UnsupportedTrigger {
        trigger: "cron".to_owned(),
    });
    assert!(diag.message.contains("cron"));
}

// ---------------------------------------------------------------------------
// Exact-code assertion tests
// ---------------------------------------------------------------------------

#[test]
fn expression_stack_exceeded_code_and_message() {
    let diag = diagnostic_from_error(&ValidationError::ExpressionStackExceeded {
        declared: 100,
        limit: 64,
    });
    assert_eq!(diag.numeric_code.code(), 0x0501);
    assert!(diag.message.contains("100"));
    assert!(diag.message.contains("64"));
}

#[test]
fn expression_stack_mismatch_code_and_message() {
    let diag = diagnostic_from_error(&ValidationError::ExpressionStackMismatch {
        expr_index: 3,
        declared: 4,
        computed: 2,
    });
    assert_eq!(diag.numeric_code.code(), 0x0502);
    assert!(diag.message.contains("3"));
    assert!(diag.message.contains("4"));
    assert!(diag.message.contains("2"));
}

#[test]
fn accessor_slot_out_of_range_code_and_message() {
    let diag = diagnostic_from_error(&ValidationError::AccessorSlotOutOfRange {
        accessor_index: 1,
        slot: 10,
        slot_count: 5,
    });
    assert_eq!(diag.numeric_code.code(), 0x0503);
    assert!(diag.message.contains("10"));
    assert!(diag.message.contains("5"));
}

#[test]
fn slot_dependency_cycle_code_and_message() {
    let diag = diagnostic_from_error(&ValidationError::SlotDependencyCycle {
        slot: 2,
        chain: "2 -> 3 -> 2".to_owned(),
    });
    assert_eq!(diag.numeric_code.code(), 0x0507);
    assert!(diag.message.contains("2 -> 3 -> 2"));
}

#[test]
fn action_contract_missing_code_and_message() {
    let diag = diagnostic_from_error(&ValidationError::ActionContractMissing {
        action_id: 42,
        node_index: 3,
    });
    assert_eq!(diag.numeric_code.code(), 0x0509);
    assert!(diag.message.contains("42"));
    assert!(diag.message.contains("3"));
}

#[test]
fn action_contract_orphan_code_and_message() {
    let diag = diagnostic_from_error(&ValidationError::ActionContractOrphan { action_id: 10 });
    assert_eq!(diag.numeric_code.code(), 0x050A);
    assert!(diag.message.contains("10"));
}

#[test]
fn slot_type_inconsistency_code_and_message() {
    let diag = diagnostic_from_error(&ValidationError::SlotTypeInconsistency { slot: 4 });
    assert_eq!(diag.numeric_code.code(), 0x050B);
    assert!(diag.message.contains("4"));
}

#[test]
fn non_deterministic_path_code_and_message() {
    let diag = diagnostic_from_error(&ValidationError::NonDeterministicPath {
        from_node: 1,
        to_node: 5,
    });
    assert_eq!(diag.numeric_code.code(), 0x050C);
    assert!(diag.message.contains("1"));
    assert!(diag.message.contains("5"));
}

#[test]
fn forbidden_yaml_feature_code_is_e0102() {
    let diag = diagnostic_from_error(&ValidationError::ForbiddenYamlFeature);
    assert_eq!(diag.numeric_code.code(), 0x0102);
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn direct_runtime_reference_code_is_e0204() {
    let diag = diagnostic_from_error(&ValidationError::DirectRuntimeReference);
    assert_eq!(diag.numeric_code.code(), 0x0204);
}

#[test]
fn secret_result_leak_code_is_e0406() {
    let diag = diagnostic_from_error(&ValidationError::SecretResultLeak);
    assert_eq!(diag.numeric_code.code(), 0x0406);
}

#[test]
fn payload_too_large_code_is_e0408() {
    let diag = diagnostic_from_error(&ValidationError::PayloadTooLarge);
    assert_eq!(diag.numeric_code.code(), 0x0408);
}

#[test]
fn http_trigger_out_of_core_code_is_e040c() {
    let diag = diagnostic_from_error(&ValidationError::HttpTriggerOutOfCore);
    assert_eq!(diag.numeric_code.code(), 0x040C);
}

// ---------------------------------------------------------------------------
// Mapping invariant — every variant maps to a non-empty message
// ---------------------------------------------------------------------------

#[test]
fn all_variants_map_to_non_empty_message() {
    for error in crate::diag_convert::all_variants() {
        let (_, message) = error_diagnostic_parts(&error);
        assert!(!message.is_empty(), "empty message for {error:?}");
    }
}
