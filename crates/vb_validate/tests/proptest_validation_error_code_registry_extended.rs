//! Property test: Every ValidationError variant maps to a registered
//! SymbolicCode in CODE_REGISTRY (46 variants including gate-specific E05xx
//! and contract-discovery E06xx codes).
//!
//! PO-010 / PS-010: Error code stability — all ValidationError variants registered.
//!
//! Invariants:
//!   - Every ValidationError::code() returns a SymbolicCode.
//!   - Every returned symbolic name exists in CODE_REGISTRY.

use vb_core::diagnostic::{CODE_REGISTRY, SymbolicCode};
use vb_validate::ValidationError;

fn all_validation_error_variants() -> Vec<ValidationError> {
    vec![
        // Schema errors (E01xx)
        ValidationError::DuplicateKey,
        ValidationError::ForbiddenYamlFeature,
        ValidationError::UnknownTopLevelField,
        ValidationError::UnknownStepField,
        ValidationError::MissingRequiredField {
            field: "steps".into(),
        },
        ValidationError::InvalidVersion {
            version: "999".into(),
        },
        ValidationError::InvalidId {
            id: "bad-id!".into(),
        },
        ValidationError::ReservedId {
            id: "run".into(),
        },
        ValidationError::DuplicateId {
            id: "step1".into(),
        },
        ValidationError::MultipleStepPrimitives,
        ValidationError::MissingStepPrimitive,
        // Reference errors (E02xx)
        ValidationError::UnknownReference {
            reference: "nonexistent".into(),
        },
        ValidationError::FutureReference {
            reference: "future-ref".into(),
        },
        ValidationError::SecretNotDeclared {
            secret: "api_key".into(),
        },
        ValidationError::DirectRuntimeReference,
        // Control flow errors (E03xx)
        ValidationError::InvalidThenTarget,
        ValidationError::ControlFlowCycle,
        ValidationError::UnreachableStep {
            step: "orphan".into(),
        },
        ValidationError::InvalidChoose,
        ValidationError::InvalidForEach,
        ValidationError::InvalidTogether,
        ValidationError::InvalidCollect,
        ValidationError::InvalidReduce,
        ValidationError::InvalidRepeat,
        // Type/taint errors (E04xx)
        ValidationError::InvalidWait,
        ValidationError::InvalidAsk,
        ValidationError::InvalidFinish,
        ValidationError::InvalidRetry,
        ValidationError::InvalidOnError,
        ValidationError::SecretResultLeak,
        ValidationError::TypeMismatch {
            expected: "u64".into(),
            found: "string".into(),
        },
        ValidationError::PayloadTooLarge,
        ValidationError::LimitRequired {
            resource: "cpu".into(),
        },
        ValidationError::LimitExceeded {
            resource: "memory".into(),
        },
        ValidationError::UnsupportedTrigger {
            trigger: "cron".into(),
        },
        ValidationError::HttpTriggerOutOfCore,
        // Gate verifier errors (E05xx)
        ValidationError::ExpressionStackExceeded {
            declared: 100,
            limit: 50,
        },
        ValidationError::ExpressionStackMismatch {
            expr_index: 0,
            declared: 10,
            computed: 20,
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
            max: 5,
        },
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 999,
            symbols_count: 10,
        },
        ValidationError::SlotReferenceOutOfRange {
            slot: 5,
            slot_count: 3,
            context: "step".into(),
        },
        ValidationError::LoopBodyStepOutOfRange {
            step: 10,
            node_count: 5,
            source_node: 2,
            label: "body".into(),
        },
        ValidationError::SlotDependencyCycle {
            slot: 1,
            chain: "1→2→1".into(),
        },
        ValidationError::NodeKindConstraintViolation {
            node_index: 3,
            detail: "bad kind".into(),
        },
        ValidationError::ActionContractMissing {
            action_id: 1,
            node_index: 2,
        },
        ValidationError::ActionContractOrphan { action_id: 1 },
        ValidationError::CapabilityNameEmpty {
            action_id: 1,
            capability_index: 0,
        },
        ValidationError::CapabilityNameTooLong {
            action_id: 1,
            capability_index: 0,
            len: 200,
            max: 100,
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
    ]
}

#[test]
fn all_validation_error_variants_enumerated() {
    let variants = all_validation_error_variants();
    // We expect 46 variants. Count what we have:
    assert!(
        variants.len() >= 46,
        "Expected at least 46 ValidationError variants, found {}",
        variants.len()
    );
}

#[test]
fn every_validation_error_code_is_registered() {
    for error in &all_validation_error_variants() {
        let code = error.code();
        // Verify reconstruction from static string
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "ValidationError::code() returned '{}' which is not a registered SymbolicCode. \
             Variant: {:?}",
            code.as_str(),
            error
        );
        // Verify in CODE_REGISTRY
        assert!(
            CODE_REGISTRY
                .iter()
                .any(|e| e.symbolic == code.as_str()),
            "ValidationError code '{}' not found in CODE_REGISTRY. Variant: {:?}",
            code.as_str(),
            error
        );
    }
}

#[test]
fn validation_error_schema_codes_cover_section16_names() {
    // Verify that each Section 16 code name has at least one variant
    let section16_names: &[&str] = &[
        "DUPLICATE_KEY",
        "FORBIDDEN_YAML_FEATURE",
        "UNKNOWN_TOP_LEVEL_FIELD",
        "UNKNOWN_STEP_FIELD",
        "MISSING_REQUIRED_FIELD",
        "INVALID_VERSION",
        "INVALID_ID",
        "RESERVED_ID",
        "DUPLICATE_ID",
        "MULTIPLE_STEP_PRIMITIVES",
        "MISSING_STEP_PRIMITIVE",
        "UNKNOWN_REFERENCE",
        "FUTURE_REFERENCE",
        "SECRET_NOT_DECLARED",
        "DIRECT_RUNTIME_REFERENCE",
        "INVALID_THEN_TARGET",
        "CONTROL_FLOW_CYCLE",
        "UNREACHABLE_STEP",
        "INVALID_CHOOSE",
        "INVALID_FOR_EACH",
        "INVALID_TOGETHER",
        "INVALID_COLLECT",
        "INVALID_REDUCE",
        "INVALID_REPEAT",
        "INVALID_WAIT",
        "INVALID_ASK",
        "INVALID_FINISH",
        "INVALID_RETRY",
        "INVALID_ON_ERROR",
        "SECRET_RESULT_LEAK",
        "TYPE_MISMATCH",
        "PAYLOAD_TOO_LARGE",
        "LIMIT_REQUIRED",
        "LIMIT_EXCEEDED",
        "UNSUPPORTED_TRIGGER",
        "HTTP_TRIGGER_OUT_OF_CORE",
    ];

    let mut covered: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for error in &all_validation_error_variants() {
        covered.insert(error.code().as_str());
    }

    for name in section16_names {
        assert!(
            covered.contains(name),
            "Section 16 code '{}' has no ValidationError variant covering it",
            name
        );
    }
}

#[test]
fn validation_error_gate_codes_cover_e05xx_names() {
    let gate_names: &[&str] = &[
        "EXPRESSION_STACK_EXCEEDED",
        "EXPRESSION_STACK_MISMATCH",
        "ACCESSOR_SLOT_OUT_OF_RANGE",
        "ACCESSOR_PATH_INVALID",
        "ACCESSOR_PATH_TOO_DEEP",
        "ACCESSOR_SYMBOL_OUT_OF_BOUNDS",
        "SLOT_REFERENCE_OUT_OF_RANGE",
        "LOOP_BODY_STEP_OUT_OF_RANGE",
        "SLOT_DEPENDENCY_CYCLE",
        "NODE_KIND_CONSTRAINT_VIOLATION",
        "ACTION_CONTRACT_MISSING",
        "ACTION_CONTRACT_ORPHAN",
        "CAPABILITY_NAME_EMPTY",
        "CAPABILITY_NAME_TOO_LONG",
        "CAPABILITY_NAME_INVALID",
        "CAPABILITY_ACTION_MISMATCH",
    ];

    let mut covered: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for error in &all_validation_error_variants() {
        covered.insert(error.code().as_str());
    }

    for name in gate_names {
        assert!(
            covered.contains(name),
            "Gate code '{}' has no ValidationError variant covering it",
            name
        );
    }
}
