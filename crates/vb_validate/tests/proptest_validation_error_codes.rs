#![forbid(unsafe_code)]
//! PO-017: proptest for ValidationError code uniqueness.
//!
//! Tests: Every ValidationError variant produces a unique DiagnosticCode
//! via vb_validate::diagnostic::error_code. Codes are non-zero.
//!
//! Bound: enumeration (not random).

use std::collections::HashSet;
use std::str::FromStr;
use vb_core::diagnostic::DiagnosticCode;
use vb_validate::ValidationError;
use vb_validate::diagnostic;

/// Generate representative instances of each ValidationError variant.
/// Uses the actual variant signatures from vb_validate::ValidationError.
fn all_validation_error_variants() -> Vec<ValidationError> {
    vec![
        // Schema validation
        ValidationError::DuplicateKey,
        ValidationError::ForbiddenYamlFeature,
        ValidationError::UnknownTopLevelField,
        ValidationError::UnknownStepField,
        ValidationError::MissingRequiredField {
            field: "test".into(),
        },
        ValidationError::InvalidVersion {
            version: "0.0".into(),
        },
        ValidationError::InvalidId { id: "x".into() },
        ValidationError::ReservedId { id: "x".into() },
        ValidationError::DuplicateId { id: "x".into() },
        ValidationError::MultipleStepPrimitives,
        ValidationError::MissingStepPrimitive,
        // Reference validation
        ValidationError::UnknownReference {
            reference: "x".into(),
        },
        ValidationError::FutureReference {
            reference: "x".into(),
        },
        ValidationError::SecretNotDeclared { secret: "x".into() },
        ValidationError::DirectRuntimeReference,
        // Control flow
        ValidationError::InvalidThenTarget,
        ValidationError::ControlFlowCycle,
        ValidationError::UnreachableStep { step: "x".into() },
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
        // Taint/Resource
        ValidationError::SecretResultLeak,
        ValidationError::TypeMismatch {
            expected: "u64".into(),
            found: "string".into(),
        },
        ValidationError::PayloadTooLarge,
        ValidationError::LimitRequired {
            resource: "steps".into(),
        },
        ValidationError::LimitExceeded {
            resource: "steps".into(),
        },
        ValidationError::UnsupportedTrigger {
            trigger: "http".into(),
        },
        ValidationError::HttpTriggerOutOfCore,
        // Gate errors
        ValidationError::ExpressionStackExceeded {
            declared: 100,
            limit: 64,
        },
        ValidationError::ExpressionStackMismatch {
            expr_index: 0,
            declared: 10,
            computed: 8,
        },
        ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 100,
            slot_count: 5,
        },
        ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 0,
        },
        ValidationError::AccessorPathTooDeep {
            accessor_index: 0,
            depth: 100,
            max: 32,
        },
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 100,
            symbols_count: 5,
        },
        ValidationError::SlotReferenceOutOfRange {
            slot: 100,
            slot_count: 5,
            context: "test".into(),
        },
        ValidationError::LoopBodyStepOutOfRange {
            step: 100,
            node_count: 5,
            source_node: 0,
            label: "loop".into(),
        },
        ValidationError::SlotDependencyCycle {
            slot: 0,
            chain: "0→1→0".into(),
        },
        ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: "invalid".into(),
        },
        ValidationError::ActionContractMissing {
            action_id: 1,
            node_index: 0,
        },
        ValidationError::ActionContractOrphan { action_id: 1 },
        ValidationError::CapabilityNameEmpty {
            action_id: 1,
            capability_index: 0,
        },
        ValidationError::CapabilityNameTooLong {
            action_id: 1,
            capability_index: 0,
            len: 256,
            max: 64,
        },
        ValidationError::CapabilityNameInvalid {
            action_id: 1,
            capability_index: 0,
            name: "bad!".into(),
        },
        ValidationError::CapabilityActionMismatch {
            contract_action_id: 1,
            capability_action_id: 2,
            capability_index: 0,
        },
        ValidationError::CapabilityDuplicate {
            action_id: 1,
            first_index: 0,
            duplicate_index: 1,
            name: "dup".into(),
        },
        ValidationError::SlotTypeInconsistency { slot: 0 },
        ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1,
        },
        // Contract discovery
        ValidationError::MissingSchemaVersion,
        ValidationError::CueVetFailed {
            file: "test.cue".into(),
        },
        ValidationError::VersionMonotonicityBreach {
            file: "test.cue".into(),
            expected: "1".into(),
            actual: "2".into(),
        },
    ]
}

#[test]
fn all_validation_error_variants_produce_unique_codes() {
    let variants = all_validation_error_variants();
    assert!(!variants.is_empty(), "Must have at least one variant");

    let mut codes = HashSet::new();
    let mut duplicates = Vec::new();

    for (i, variant) in variants.iter().enumerate() {
        let code: DiagnosticCode = diagnostic::error_code(variant);
        let raw = code.code();

        // Every code must be non-zero
        assert_ne!(raw, 0, "Variant {} must have non-zero code", i);

        // Check for duplicates
        if !codes.insert(raw) {
            duplicates.push(format!(
                "Duplicate code E{:04X} at variant index {}",
                raw, i
            ));
        }
    }

    assert!(
        duplicates.is_empty(),
        "ValidationError variants must produce unique codes:\n{}",
        duplicates.join("\n")
    );
}

#[test]
fn validation_error_codes_are_all_parseable() {
    let variants = all_validation_error_variants();
    for (i, variant) in variants.iter().enumerate() {
        let code: DiagnosticCode = diagnostic::error_code(variant);
        let display = code.to_string();
        let parsed = DiagnosticCode::from_str(&display);
        assert_eq!(
            parsed,
            Ok(code),
            "Variant {} code {} must round-trip via from_str",
            i,
            display
        );
    }
}

#[test]
fn validation_error_code_count_matches_expected() {
    let variants = all_validation_error_variants();
    assert!(
        variants.len() >= 30,
        "Expected at least 30 ValidationError variants, got {}",
        variants.len()
    );
}
