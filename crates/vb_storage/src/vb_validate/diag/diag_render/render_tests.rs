#![forbid(unsafe_code)]

use crate::vb_validate::*;
use crate::vb_validate::diag::diag_convert::all_variants;
use std::collections::BTreeSet;

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
    for error in all_variants() {
        let diag = diagnostic_from_error(&error);
        assert!(!diag.message.is_empty(), "empty message for {error:?}");
    }
}

#[test]
fn error_code_is_non_zero_for_all_variants() {
    for error in all_variants() {
        let code = error_code(&error).code();
        assert_ne!(code, 0, "zero code for {error:?}");
    }
}

#[test]
fn error_code_is_unique_for_all_variants() {
    let mut seen = BTreeSet::new();
    for error in all_variants() {
        let code = error_code(&error).code();
        assert!(seen.insert(code), "duplicate code {code:#06x}");
    }
}

#[test]
fn basic_messages_contain_payloads() {
    let cases = [
        (
            ValidationError::MissingRequiredField {
                field: "steps".to_owned(),
            },
            "steps",
        ),
        (
            ValidationError::InvalidVersion {
                version: "bad/v2".to_owned(),
            },
            "bad/v2",
        ),
        (
            ValidationError::InvalidId {
                id: "123bad".to_owned(),
            },
            "123bad",
        ),
        (
            ValidationError::ReservedId {
                id: "runtime".to_owned(),
            },
            "runtime",
        ),
        (
            ValidationError::DuplicateId {
                id: "dup_step".to_owned(),
            },
            "dup_step",
        ),
        (
            ValidationError::UnknownReference {
                reference: "$input.x".to_owned(),
            },
            "$input.x",
        ),
        (
            ValidationError::FutureReference {
                reference: "$steps.later".to_owned(),
            },
            "$steps.later",
        ),
        (
            ValidationError::SecretNotDeclared {
                secret: "api_key".to_owned(),
            },
            "api_key",
        ),
        (
            ValidationError::UnreachableStep {
                step: "orphan".to_owned(),
            },
            "orphan",
        ),
    ];
    for (error, needle) in cases {
        assert!(diagnostic_from_error(&error).message.contains(needle));
    }
}

#[test]
fn type_and_limit_messages_contain_values() {
    let diag = diagnostic_from_error(&ValidationError::TypeMismatch {
        expected: "boolean".to_owned(),
        found: "number".to_owned(),
    });
    assert!(diag.message.contains("boolean"));
    assert!(diag.message.contains("number"));

    for error in [
        ValidationError::LimitRequired {
            resource: "max_slots".to_owned(),
        },
        ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        },
        ValidationError::UnsupportedTrigger {
            trigger: "cron".to_owned(),
        },
    ] {
        let diag = diagnostic_from_error(&error);
        assert!(!diag.message.is_empty());
    }
}

#[test]
fn gate_codes_and_messages_are_stable() {
    let stack_exceeded = diagnostic_from_error(&ValidationError::ExpressionStackExceeded {
        declared: 100,
        limit: 64,
    });
    assert_eq!(stack_exceeded.numeric_code.code(), 0x0501);
    assert!(stack_exceeded.message.contains("100"));
    assert!(stack_exceeded.message.contains("64"));

    let stack_mismatch = diagnostic_from_error(&ValidationError::ExpressionStackMismatch {
        expr_index: 3,
        declared: 4,
        computed: 2,
    });
    assert_eq!(stack_mismatch.numeric_code.code(), 0x0502);

    let accessor = diagnostic_from_error(&ValidationError::AccessorSlotOutOfRange {
        accessor_index: 1,
        slot: 10,
        slot_count: 5,
    });
    assert_eq!(accessor.numeric_code.code(), 0x0503);
}

#[test]
fn contract_codes_and_messages_are_stable() {
    let cycle = diagnostic_from_error(&ValidationError::SlotDependencyCycle {
        slot: 2,
        chain: "2 -> 3 -> 2".to_owned(),
    });
    assert_eq!(cycle.numeric_code.code(), 0x0507);
    assert!(cycle.message.contains("2 -> 3 -> 2"));

    let missing = diagnostic_from_error(&ValidationError::ActionContractMissing {
        action_id: 42,
        node_index: 3,
    });
    assert_eq!(missing.numeric_code.code(), 0x0509);

    let orphan = diagnostic_from_error(&ValidationError::ActionContractOrphan { action_id: 10 });
    assert_eq!(orphan.numeric_code.code(), 0x050A);

    let slot = diagnostic_from_error(&ValidationError::SlotTypeInconsistency { slot: 4 });
    assert_eq!(slot.numeric_code.code(), 0x050B);

    let path = diagnostic_from_error(&ValidationError::NonDeterministicPath {
        from_node: 1,
        to_node: 5,
    });
    assert_eq!(path.numeric_code.code(), 0x050C);
}

#[test]
fn selected_legacy_codes_are_stable() {
    let forbidden = diagnostic_from_error(&ValidationError::ForbiddenYamlFeature);
    assert_eq!(forbidden.numeric_code.code(), 0x0102);
    assert_eq!(forbidden.severity, Severity::Error);
    assert_eq!(
        diagnostic_from_error(&ValidationError::DirectRuntimeReference)
            .numeric_code
            .code(),
        0x0204
    );
    assert_eq!(
        diagnostic_from_error(&ValidationError::SecretResultLeak)
            .numeric_code
            .code(),
        0x0406
    );
    assert_eq!(
        diagnostic_from_error(&ValidationError::PayloadTooLarge)
            .numeric_code
            .code(),
        0x0408
    );
    assert_eq!(
        diagnostic_from_error(&ValidationError::HttpTriggerOutOfCore)
            .numeric_code
            .code(),
        0x040C
    );
}
