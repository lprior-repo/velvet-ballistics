//! Property tests for ValidationError symbolic code uniqueness and coverage.
//!
//! Compensates: verified in Kani (PO-003), defense-in-depth runtime check.
//! Invariant: All 58 ValidationError variants produce unique SymbolicCodes.
//! Invariant: Every returned SymbolicCode is registered in CODE_REGISTRY.
//! Invariant: code() and symbolic_code() are consistent.

use std::collections::BTreeSet;
use vb_core::diagnostic::{CODE_REGISTRY, HasSymbolicCode, SymbolicCode};
use vb_validate::ValidationError;

/// Enumerate all 58 ValidationError variants.
fn all_validation_error_variants() -> Vec<ValidationError> {
    vec![
        // Schema: E01xx (11 variants)
        ValidationError::DuplicateKey,
        ValidationError::ForbiddenYamlFeature,
        ValidationError::UnknownTopLevelField,
        ValidationError::UnknownStepField,
        ValidationError::MissingRequiredField {
            field: "version".into(),
        },
        ValidationError::InvalidVersion {
            version: "0".into(),
        },
        ValidationError::InvalidId { id: "test".into() },
        ValidationError::ReservedId { id: "if".into() },
        ValidationError::DuplicateId { id: "dup".into() },
        ValidationError::MultipleStepPrimitives,
        ValidationError::MissingStepPrimitive,
        // Reference: E02xx (4 variants)
        ValidationError::UnknownReference {
            reference: "ref".into(),
        },
        ValidationError::FutureReference {
            reference: "ref".into(),
        },
        ValidationError::SecretNotDeclared { secret: "s".into() },
        ValidationError::DirectRuntimeReference,
        // Control Flow: E03xx (9 variants)
        ValidationError::InvalidThenTarget,
        ValidationError::ControlFlowCycle,
        ValidationError::UnreachableStep { step: "1".into() },
        ValidationError::InvalidChoose,
        ValidationError::InvalidForEach,
        ValidationError::InvalidTogether,
        ValidationError::InvalidCollect,
        ValidationError::InvalidReduce,
        ValidationError::InvalidRepeat,
        // Type/Taint: E04xx (12 variants)
        ValidationError::InvalidWait,
        ValidationError::InvalidAsk,
        ValidationError::InvalidFinish,
        ValidationError::InvalidRetry,
        ValidationError::InvalidOnError,
        ValidationError::SecretResultLeak,
        ValidationError::TypeMismatch {
            expected: "a".into(),
            found: "b".into(),
        },
        ValidationError::PayloadTooLarge,
        ValidationError::LimitRequired {
            resource: "cpu".into(),
        },
        ValidationError::LimitExceeded {
            resource: "cpu".into(),
        },
        ValidationError::UnsupportedTrigger {
            trigger: "cron".into(),
        },
        ValidationError::HttpTriggerOutOfCore,
        // Gate Verifier: E05xx (19 variants)
        ValidationError::ExpressionStackExceeded {
            declared: 65,
            limit: 64,
        },
        ValidationError::ExpressionStackMismatch {
            expr_index: 0,
            declared: 1,
            computed: 2,
        },
        ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 3,
        },
        ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 1,
        },
        ValidationError::AccessorPathTooDeep {
            accessor_index: 0,
            depth: 10,
            max: 8,
        },
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 5,
            symbols_count: 3,
        },
        ValidationError::SlotReferenceOutOfRange {
            slot: 5,
            slot_count: 3,
            context: "test".into(),
        },
        ValidationError::LoopBodyStepOutOfRange {
            step: 10,
            node_count: 5,
            source_node: 0,
            label: "test".into(),
        },
        ValidationError::SlotDependencyCycle {
            slot: 0,
            chain: "0→1→0".into(),
        },
        ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: "test".into(),
        },
        ValidationError::ActionContractMissing {
            action_id: 0,
            node_index: 1,
        },
        ValidationError::ActionContractOrphan { action_id: 0 },
        ValidationError::CapabilityNameEmpty {
            action_id: 0,
            capability_index: 0,
        },
        ValidationError::CapabilityNameTooLong {
            action_id: 0,
            capability_index: 0,
            len: 200,
            max: 64,
        },
        ValidationError::CapabilityNameInvalid {
            action_id: 0,
            capability_index: 0,
            name: "$inv@lid".into(),
        },
        ValidationError::CapabilityActionMismatch {
            contract_action_id: 0,
            capability_action_id: 1,
            capability_index: 0,
        },
        ValidationError::CapabilityDuplicate {
            action_id: 0,
            first_index: 0,
            duplicate_index: 1,
            name: "test".into(),
        },
        ValidationError::SlotTypeInconsistency { slot: 0 },
        ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1,
        },
        // Contract Discovery: E06xx (3 variants)
        ValidationError::MissingSchemaVersion,
        ValidationError::CueVetFailed {
            file: "test".into(),
        },
        ValidationError::VersionMonotonicityBreach {
            file: "test".into(),
            expected: "1".into(),
            actual: "0".into(),
        },
    ]
}

#[test]
fn validation_error_code_returns_symbolic_for_all_58_variants() {
    let variants = all_validation_error_variants();
    assert_eq!(
        variants.len(),
        58,
        "must have exactly 58 ValidationError variants"
    );

    for error in &variants {
        let code = error.code();
        // Verify the symbolic code is registered
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "ValidationError code '{}' must be registered in CODE_REGISTRY",
            code.as_str()
        );
    }
}

#[test]
fn validation_error_code_all_58_unique_symbolic_codes() {
    let variants = all_validation_error_variants();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for error in &variants {
        let code = error.code();
        let name = code.as_str();
        let is_new = seen.insert(name);
        assert!(
            is_new,
            "duplicate symbolic code '{}' returned by multiple ValidationError variants",
            name
        );
    }
    assert_eq!(
        seen.len(),
        58,
        "all 58 variants must produce unique SymbolicCodes"
    );
}

#[test]
fn validation_error_code_and_symbolic_code_consistent() {
    let variants = all_validation_error_variants();
    for error in &variants {
        let code1 = error.code();
        let code2 = HasSymbolicCode::symbolic_code(error);
        assert_eq!(
            code1, code2,
            "ValidationError::code() and HasSymbolicCode::symbolic_code() must agree"
        );
    }
}

#[test]
fn validation_error_all_codes_registered_in_code_registry() {
    let variants = all_validation_error_variants();
    for error in &variants {
        let code = error.code();
        let found = CODE_REGISTRY.iter().any(|e| e.symbolic == code.as_str());
        assert!(
            found,
            "ValidationError code '{}' must have an entry in CODE_REGISTRY",
            code.as_str()
        );
    }
}
