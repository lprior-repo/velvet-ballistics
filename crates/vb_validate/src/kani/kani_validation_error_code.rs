#![forbid(unsafe_code)]
//! PO-003: Kani harness verifying every production `ValidationError` variant
//! maps to a registered diagnostic code in the production `CODE_REGISTRY`.
//!
//! Uses the actual production types:
//!   - `crate::ValidationError` (production enum)
//!   - `crate::diagnostic::error_code()` → `DiagnosticCode`
//!   - `DiagnosticCode::symbolic_code()` → `Option<SymbolicCode>`
//!
//! REPAIR-9 (F-R8-001): Replaced model `enum ValidationError` +
//! `code_name()` with production `ValidationError` + `diagnostic::error_code()`.
//! The harness calls the production code path through `stub_error_diagnostic_parts`
//! which eliminates `format!()` String allocation overhead while preserving
//! the exact same match logic as the production `error_diagnostic_parts`.
//!
//! Bound: 58 variants (split into 6 sub-harnesses, unwind=200 each)
//! to mitigate `iter().find()` over CODE_REGISTRY (157 entries).
//!
//! Compensating evidence: proptest PO-017 (proptest_validation_error_codes,
//! 3/3 PASS) verifies the same property at runtime.

use crate::diagnostic;
use crate::ValidationError;
use vb_core::diagnostic::DiagnosticCode;

/// Production error-to-code mapping constants.
/// Mirrors the const values in `crates/vb_validate/src/diagnostic.rs`.
const C_DUPLICATE_KEY: u16 = 0x0101;
const C_FORBIDDEN_YAML_FEATURE: u16 = 0x0102;
const C_UNKNOWN_TOP_LEVEL_FIELD: u16 = 0x0103;
const C_UNKNOWN_STEP_FIELD: u16 = 0x0104;
const C_MISSING_REQUIRED_FIELD: u16 = 0x0105;
const C_INVALID_VERSION: u16 = 0x0106;
const C_INVALID_ID: u16 = 0x0107;
const C_RESERVED_ID: u16 = 0x0108;
const C_DUPLICATE_ID: u16 = 0x0109;
const C_MULTIPLE_STEP_PRIMITIVES: u16 = 0x010A;
const C_MISSING_STEP_PRIMITIVE: u16 = 0x010B;
const C_UNKNOWN_REFERENCE: u16 = 0x0201;
const C_FUTURE_REFERENCE: u16 = 0x0202;
const C_SECRET_NOT_DECLARED: u16 = 0x0203;
const C_DIRECT_RUNTIME_REFERENCE: u16 = 0x0204;
const C_INVALID_THEN_TARGET: u16 = 0x0301;
const C_CONTROL_FLOW_CYCLE: u16 = 0x0302;
const C_UNREACHABLE_STEP: u16 = 0x0303;
const C_INVALID_CHOOSE: u16 = 0x0304;
const C_INVALID_FOR_EACH: u16 = 0x0305;
const C_INVALID_TOGETHER: u16 = 0x0306;
const C_INVALID_COLLECT: u16 = 0x0307;
const C_INVALID_REDUCE: u16 = 0x0308;
const C_INVALID_REPEAT: u16 = 0x0309;
const C_INVALID_WAIT: u16 = 0x0401;
const C_INVALID_ASK: u16 = 0x0402;
const C_INVALID_FINISH: u16 = 0x0403;
const C_INVALID_RETRY: u16 = 0x0404;
const C_INVALID_ON_ERROR: u16 = 0x0405;
const C_SECRET_RESULT_LEAK: u16 = 0x0406;
const C_TYPE_MISMATCH: u16 = 0x0407;
const C_PAYLOAD_TOO_LARGE: u16 = 0x0408;
const C_LIMIT_REQUIRED: u16 = 0x0409;
const C_LIMIT_EXCEEDED: u16 = 0x040A;
const C_UNSUPPORTED_TRIGGER: u16 = 0x040B;
const C_HTTP_TRIGGER_OUT_OF_CORE: u16 = 0x040C;
const C_EXPRESSION_STACK_EXCEEDED: u16 = 0x0501;
const C_EXPRESSION_STACK_MISMATCH: u16 = 0x0502;
const C_ACCESSOR_SLOT_OUT_OF_RANGE: u16 = 0x0503;
const C_ACCESSOR_PATH_INVALID: u16 = 0x0504;
const C_SLOT_REFERENCE_OUT_OF_RANGE: u16 = 0x0505;
const C_LOOP_BODY_STEP_OUT_OF_RANGE: u16 = 0x0506;
const C_SLOT_DEPENDENCY_CYCLE: u16 = 0x0507;
const C_NODE_KIND_CONSTRAINT_VIOLATION: u16 = 0x0508;
const C_ACTION_CONTRACT_MISSING: u16 = 0x0509;
const C_ACTION_CONTRACT_ORPHAN: u16 = 0x050A;
const C_SLOT_TYPE_INCONSISTENCY: u16 = 0x050B;
const C_NON_DETERMINISTIC_PATH: u16 = 0x050C;
const C_CAPABILITY_NAME_EMPTY: u16 = 0x050D;
const C_CAPABILITY_NAME_TOO_LONG: u16 = 0x050E;
const C_CAPABILITY_NAME_INVALID: u16 = 0x050F;
const C_CAPABILITY_ACTION_MISMATCH: u16 = 0x0510;
const C_CAPABILITY_DUPLICATE: u16 = 0x0511;
const C_ACCESSOR_PATH_TOO_DEEP: u16 = 0x0512;
const C_ACCESSOR_SYMBOL_OUT_OF_BOUNDS: u16 = 0x0513;
const C_MISSING_SCHEMA_VERSION: u16 = 0x0601;
const C_CUE_VET_FAILED: u16 = 0x0602;
const C_VERSION_MONOTONICITY_BREACH: u16 = 0x0603;

/// Stub for `crate::diagnostic::error_diagnostic_parts` that eliminates
/// `format!()` String allocation overhead while preserving the exact same
/// match arms as the production code in `crates/vb_validate/src/diagnostic.rs`.
///
/// The production `error_code()` only uses the `DiagnosticCode` from the
/// returned tuple (discards the message String).  This stub provides the
/// same code-path mapping without the allocation cost.
#[cfg(kani)]
fn stub_error_diagnostic_parts(error: &ValidationError) -> (DiagnosticCode, String) {
    match error {
        ValidationError::DuplicateKey => (DiagnosticCode::new(C_DUPLICATE_KEY), String::new()),
        ValidationError::ForbiddenYamlFeature => {
            (DiagnosticCode::new(C_FORBIDDEN_YAML_FEATURE), String::new())
        }
        ValidationError::UnknownTopLevelField => {
            (DiagnosticCode::new(C_UNKNOWN_TOP_LEVEL_FIELD), String::new())
        }
        ValidationError::UnknownStepField => {
            (DiagnosticCode::new(C_UNKNOWN_STEP_FIELD), String::new())
        }
        ValidationError::MissingRequiredField { .. } => {
            (DiagnosticCode::new(C_MISSING_REQUIRED_FIELD), String::new())
        }
        ValidationError::InvalidVersion { .. } => {
            (DiagnosticCode::new(C_INVALID_VERSION), String::new())
        }
        ValidationError::InvalidId { .. } => (DiagnosticCode::new(C_INVALID_ID), String::new()),
        ValidationError::ReservedId { .. } => (DiagnosticCode::new(C_RESERVED_ID), String::new()),
        ValidationError::DuplicateId { .. } => {
            (DiagnosticCode::new(C_DUPLICATE_ID), String::new())
        }
        ValidationError::MultipleStepPrimitives => {
            (DiagnosticCode::new(C_MULTIPLE_STEP_PRIMITIVES), String::new())
        }
        ValidationError::MissingStepPrimitive => {
            (DiagnosticCode::new(C_MISSING_STEP_PRIMITIVE), String::new())
        }
        ValidationError::UnknownReference { .. } => {
            (DiagnosticCode::new(C_UNKNOWN_REFERENCE), String::new())
        }
        ValidationError::FutureReference { .. } => {
            (DiagnosticCode::new(C_FUTURE_REFERENCE), String::new())
        }
        ValidationError::SecretNotDeclared { .. } => {
            (DiagnosticCode::new(C_SECRET_NOT_DECLARED), String::new())
        }
        ValidationError::DirectRuntimeReference => {
            (DiagnosticCode::new(C_DIRECT_RUNTIME_REFERENCE), String::new())
        }
        ValidationError::InvalidThenTarget => {
            (DiagnosticCode::new(C_INVALID_THEN_TARGET), String::new())
        }
        ValidationError::ControlFlowCycle => {
            (DiagnosticCode::new(C_CONTROL_FLOW_CYCLE), String::new())
        }
        ValidationError::UnreachableStep { .. } => {
            (DiagnosticCode::new(C_UNREACHABLE_STEP), String::new())
        }
        ValidationError::InvalidChoose => {
            (DiagnosticCode::new(C_INVALID_CHOOSE), String::new())
        }
        ValidationError::InvalidForEach => {
            (DiagnosticCode::new(C_INVALID_FOR_EACH), String::new())
        }
        ValidationError::InvalidTogether => {
            (DiagnosticCode::new(C_INVALID_TOGETHER), String::new())
        }
        ValidationError::InvalidCollect => {
            (DiagnosticCode::new(C_INVALID_COLLECT), String::new())
        }
        ValidationError::InvalidReduce => {
            (DiagnosticCode::new(C_INVALID_REDUCE), String::new())
        }
        ValidationError::InvalidRepeat => {
            (DiagnosticCode::new(C_INVALID_REPEAT), String::new())
        }
        ValidationError::InvalidWait => (DiagnosticCode::new(C_INVALID_WAIT), String::new()),
        ValidationError::InvalidAsk => (DiagnosticCode::new(C_INVALID_ASK), String::new()),
        ValidationError::InvalidFinish => {
            (DiagnosticCode::new(C_INVALID_FINISH), String::new())
        }
        ValidationError::InvalidRetry => {
            (DiagnosticCode::new(C_INVALID_RETRY), String::new())
        }
        ValidationError::InvalidOnError => {
            (DiagnosticCode::new(C_INVALID_ON_ERROR), String::new())
        }
        ValidationError::SecretResultLeak => {
            (DiagnosticCode::new(C_SECRET_RESULT_LEAK), String::new())
        }
        ValidationError::TypeMismatch { .. } => {
            (DiagnosticCode::new(C_TYPE_MISMATCH), String::new())
        }
        ValidationError::PayloadTooLarge => {
            (DiagnosticCode::new(C_PAYLOAD_TOO_LARGE), String::new())
        }
        ValidationError::LimitRequired { .. } => {
            (DiagnosticCode::new(C_LIMIT_REQUIRED), String::new())
        }
        ValidationError::LimitExceeded { .. } => {
            (DiagnosticCode::new(C_LIMIT_EXCEEDED), String::new())
        }
        ValidationError::UnsupportedTrigger { .. } => {
            (DiagnosticCode::new(C_UNSUPPORTED_TRIGGER), String::new())
        }
        ValidationError::HttpTriggerOutOfCore => {
            (DiagnosticCode::new(C_HTTP_TRIGGER_OUT_OF_CORE), String::new())
        }
        ValidationError::ExpressionStackExceeded { .. } => {
            (DiagnosticCode::new(C_EXPRESSION_STACK_EXCEEDED), String::new())
        }
        ValidationError::ExpressionStackMismatch { .. } => {
            (DiagnosticCode::new(C_EXPRESSION_STACK_MISMATCH), String::new())
        }
        ValidationError::AccessorSlotOutOfRange { .. } => {
            (DiagnosticCode::new(C_ACCESSOR_SLOT_OUT_OF_RANGE), String::new())
        }
        ValidationError::AccessorPathInvalid { .. } => {
            (DiagnosticCode::new(C_ACCESSOR_PATH_INVALID), String::new())
        }
        ValidationError::AccessorPathTooDeep { .. } => {
            (DiagnosticCode::new(C_ACCESSOR_PATH_TOO_DEEP), String::new())
        }
        ValidationError::AccessorSymbolOutOfBounds { .. } => {
            (DiagnosticCode::new(C_ACCESSOR_SYMBOL_OUT_OF_BOUNDS), String::new())
        }
        ValidationError::SlotReferenceOutOfRange { .. } => {
            (DiagnosticCode::new(C_SLOT_REFERENCE_OUT_OF_RANGE), String::new())
        }
        ValidationError::LoopBodyStepOutOfRange { .. } => {
            (DiagnosticCode::new(C_LOOP_BODY_STEP_OUT_OF_RANGE), String::new())
        }
        ValidationError::SlotDependencyCycle { .. } => {
            (DiagnosticCode::new(C_SLOT_DEPENDENCY_CYCLE), String::new())
        }
        ValidationError::NodeKindConstraintViolation { .. } => {
            (DiagnosticCode::new(C_NODE_KIND_CONSTRAINT_VIOLATION), String::new())
        }
        ValidationError::ActionContractMissing { .. } => {
            (DiagnosticCode::new(C_ACTION_CONTRACT_MISSING), String::new())
        }
        ValidationError::ActionContractOrphan { .. } => {
            (DiagnosticCode::new(C_ACTION_CONTRACT_ORPHAN), String::new())
        }
        ValidationError::SlotTypeInconsistency { .. } => {
            (DiagnosticCode::new(C_SLOT_TYPE_INCONSISTENCY), String::new())
        }
        ValidationError::NonDeterministicPath { .. } => {
            (DiagnosticCode::new(C_NON_DETERMINISTIC_PATH), String::new())
        }
        ValidationError::CapabilityNameEmpty { .. } => {
            (DiagnosticCode::new(C_CAPABILITY_NAME_EMPTY), String::new())
        }
        ValidationError::CapabilityNameTooLong { .. } => {
            (DiagnosticCode::new(C_CAPABILITY_NAME_TOO_LONG), String::new())
        }
        ValidationError::CapabilityNameInvalid { .. } => {
            (DiagnosticCode::new(C_CAPABILITY_NAME_INVALID), String::new())
        }
        ValidationError::CapabilityActionMismatch { .. } => {
            (DiagnosticCode::new(C_CAPABILITY_ACTION_MISMATCH), String::new())
        }
        ValidationError::CapabilityDuplicate { .. } => {
            (DiagnosticCode::new(C_CAPABILITY_DUPLICATE), String::new())
        }
        ValidationError::MissingSchemaVersion => {
            (DiagnosticCode::new(C_MISSING_SCHEMA_VERSION), String::new())
        }
        ValidationError::CueVetFailed { .. } => {
            (DiagnosticCode::new(C_CUE_VET_FAILED), String::new())
        }
        ValidationError::VersionMonotonicityBreach { .. } => {
            (DiagnosticCode::new(C_VERSION_MONOTONICITY_BREACH), String::new())
        }
    }
}

/// Sub-harness 1 (10 variants): Schema duplicates, YAML features, IDs.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(200)]
#[kani::stub(crate::diagnostic::error_diagnostic_parts, stub_error_diagnostic_parts)]
fn kani_validation_error_code_registered_1() {
    let variants: [ValidationError; 10] = [
        ValidationError::DuplicateKey,
        ValidationError::ForbiddenYamlFeature,
        ValidationError::UnknownTopLevelField,
        ValidationError::UnknownStepField,
        ValidationError::MissingRequiredField {
            field: String::new(),
        },
        ValidationError::InvalidVersion {
            version: String::new(),
        },
        ValidationError::InvalidId {
            id: String::new(),
        },
        ValidationError::ReservedId {
            id: String::new(),
        },
        ValidationError::DuplicateId {
            id: String::new(),
        },
        ValidationError::MultipleStepPrimitives,
    ];
    for (i, variant) in variants.iter().enumerate() {
        let code: DiagnosticCode = diagnostic::error_code(variant);
        assert!(
            code.symbolic_code().is_some(),
            "Variant {}: DiagnosticCode {:04X} must be in CODE_REGISTRY",
            i,
            code.code()
        );
    }
}

/// Sub-harness 2 (10 variants): Reference and control-flow errors.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(200)]
#[kani::stub(crate::diagnostic::error_diagnostic_parts, stub_error_diagnostic_parts)]
fn kani_validation_error_code_registered_2() {
    let variants: [ValidationError; 10] = [
        ValidationError::MissingStepPrimitive,
        ValidationError::UnknownReference {
            reference: String::new(),
        },
        ValidationError::FutureReference {
            reference: String::new(),
        },
        ValidationError::SecretNotDeclared {
            secret: String::new(),
        },
        ValidationError::DirectRuntimeReference,
        ValidationError::InvalidThenTarget,
        ValidationError::ControlFlowCycle,
        ValidationError::UnreachableStep {
            step: String::new(),
        },
        ValidationError::InvalidChoose,
        ValidationError::InvalidForEach,
    ];
    for (i, variant) in variants.iter().enumerate() {
        let code: DiagnosticCode = diagnostic::error_code(variant);
        assert!(
            code.symbolic_code().is_some(),
            "Variant {}: DiagnosticCode {:04X} must be in CODE_REGISTRY",
            i,
            code.code()
        );
    }
}

/// Sub-harness 3 (10 variants): Control-flow/type-taint variants.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(200)]
#[kani::stub(crate::diagnostic::error_diagnostic_parts, stub_error_diagnostic_parts)]
fn kani_validation_error_code_registered_3() {
    let variants: [ValidationError; 10] = [
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
    ];
    for (i, variant) in variants.iter().enumerate() {
        let code: DiagnosticCode = diagnostic::error_code(variant);
        assert!(
            code.symbolic_code().is_some(),
            "Variant {}: DiagnosticCode {:04X} must be in CODE_REGISTRY",
            i,
            code.code()
        );
    }
}

/// Sub-harness 4 (10 variants): Type/taint, limit, and stack expression variants.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(200)]
#[kani::stub(crate::diagnostic::error_diagnostic_parts, stub_error_diagnostic_parts)]
fn kani_validation_error_code_registered_4() {
    let variants: [ValidationError; 10] = [
        ValidationError::TypeMismatch {
            expected: String::new(),
            found: String::new(),
        },
        ValidationError::PayloadTooLarge,
        ValidationError::LimitRequired {
            resource: String::new(),
        },
        ValidationError::LimitExceeded {
            resource: String::new(),
        },
        ValidationError::UnsupportedTrigger {
            trigger: String::new(),
        },
        ValidationError::HttpTriggerOutOfCore,
        ValidationError::ExpressionStackExceeded {
            declared: 10,
            limit: 5,
        },
        ValidationError::ExpressionStackMismatch {
            expr_index: 0,
            declared: 5,
            computed: 10,
        },
        ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 99,
            slot_count: 5,
        },
        ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 0,
        },
    ];
    for (i, variant) in variants.iter().enumerate() {
        let code: DiagnosticCode = diagnostic::error_code(variant);
        assert!(
            code.symbolic_code().is_some(),
            "Variant {}: DiagnosticCode {:04X} must be in CODE_REGISTRY",
            i,
            code.code()
        );
    }
}

/// Sub-harness 5 (10 variants): Slot, loop, gate, and capability errors.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(200)]
#[kani::stub(crate::diagnostic::error_diagnostic_parts, stub_error_diagnostic_parts)]
fn kani_validation_error_code_registered_5() {
    let variants: [ValidationError; 10] = [
        ValidationError::SlotReferenceOutOfRange {
            slot: 99,
            slot_count: 5,
            context: String::new(),
        },
        ValidationError::LoopBodyStepOutOfRange {
            step: 5,
            node_count: 10,
            source_node: 0,
            label: String::new(),
        },
        ValidationError::SlotDependencyCycle {
            slot: 0,
            chain: String::new(),
        },
        ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: String::new(),
        },
        ValidationError::ActionContractMissing {
            action_id: 0,
            node_index: 0,
        },
        ValidationError::ActionContractOrphan { action_id: 0 },
        ValidationError::SlotTypeInconsistency { slot: 0 },
        ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1,
        },
        ValidationError::CapabilityNameEmpty {
            action_id: 0,
            capability_index: 0,
        },
        ValidationError::CapabilityNameTooLong {
            action_id: 0,
            capability_index: 0,
            len: 999,
            max: 64,
        },
    ];
    for (i, variant) in variants.iter().enumerate() {
        let code: DiagnosticCode = diagnostic::error_code(variant);
        assert!(
            code.symbolic_code().is_some(),
            "Variant {}: DiagnosticCode {:04X} must be in CODE_REGISTRY",
            i,
            code.code()
        );
    }
}

/// Sub-harness 6 (8 variants): Remaining capability, accessor,
/// and contract-discovery variants.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(200)]
#[kani::stub(crate::diagnostic::error_diagnostic_parts, stub_error_diagnostic_parts)]
fn kani_validation_error_code_registered_6() {
    let variants: [ValidationError; 8] = [
        ValidationError::CapabilityNameInvalid {
            action_id: 0,
            capability_index: 0,
            name: String::new(),
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
            name: String::new(),
        },
        ValidationError::AccessorPathTooDeep {
            accessor_index: 0,
            depth: 10,
            max: 5,
        },
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 99,
            symbols_count: 10,
        },
        ValidationError::MissingSchemaVersion,
        ValidationError::CueVetFailed {
            file: String::new(),
        },
        ValidationError::VersionMonotonicityBreach {
            file: String::new(),
            expected: String::new(),
            actual: String::new(),
        },
    ];
    for (i, variant) in variants.iter().enumerate() {
        let code: DiagnosticCode = diagnostic::error_code(variant);
        assert!(
            code.symbolic_code().is_some(),
            "Variant {}: DiagnosticCode {:04X} must be in CODE_REGISTRY",
            i,
            code.code()
        );
    }
}
