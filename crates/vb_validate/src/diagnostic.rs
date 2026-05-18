#![forbid(unsafe_code)]
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
const CODE_MULTIPLE_STEP_PRIMITIVES: u16 = 0x010A;
const CODE_MISSING_STEP_PRIMITIVE: u16 = 0x010B;

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
const CODE_LIMIT_EXCEEDED: u16 = 0x040A;
const CODE_UNSUPPORTED_TRIGGER: u16 = 0x040B;
const CODE_HTTP_TRIGGER_OUT_OF_CORE: u16 = 0x040C;

/// Gate verifier errors: E05xx.
const CODE_EXPRESSION_STACK_EXCEEDED: u16 = 0x0501;
const CODE_EXPRESSION_STACK_MISMATCH: u16 = 0x0502;
const CODE_ACCESSOR_SLOT_OUT_OF_RANGE: u16 = 0x0503;
const CODE_ACCESSOR_PATH_INVALID: u16 = 0x0504;
const CODE_SLOT_REFERENCE_OUT_OF_RANGE: u16 = 0x0505;
const CODE_LOOP_BODY_STEP_OUT_OF_RANGE: u16 = 0x0506;
const CODE_SLOT_DEPENDENCY_CYCLE: u16 = 0x0507;
const CODE_NODE_KIND_CONSTRAINT_VIOLATION: u16 = 0x0508;
const CODE_ACTION_CONTRACT_MISSING: u16 = 0x0509;
const CODE_ACTION_CONTRACT_ORPHAN: u16 = 0x050A;
const CODE_SLOT_TYPE_INCONSISTENCY: u16 = 0x050B;
const CODE_NON_DETERMINISTIC_PATH: u16 = 0x050C;
const CODE_CAPABILITY_NAME_EMPTY: u16 = 0x050D;
const CODE_CAPABILITY_NAME_TOO_LONG: u16 = 0x050E;
const CODE_CAPABILITY_NAME_INVALID: u16 = 0x050F;
const CODE_CAPABILITY_ACTION_MISMATCH: u16 = 0x0510;
const CODE_CAPABILITY_DUPLICATE: u16 = 0x0511;
const CODE_ACCESSOR_PATH_TOO_DEEP: u16 = 0x0512;
const CODE_ACCESSOR_SYMBOL_OUT_OF_BOUNDS: u16 = 0x0513;

    // Contract-discovery codes (vb-6f02)
    const CODE_MISSING_SCHEMA_VERSION: u16 = 0x0601;
    const CODE_CUE_VET_FAILED: u16 = 0x0602;
    const CODE_VERSION_MONOTONICITY_BREACH: u16 = 0x0603;

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
        ValidationError::DuplicateKey => (
            DiagnosticCode::new(CODE_DUPLICATE_KEY),
            "duplicate key".into(),
        ),
        ValidationError::ForbiddenYamlFeature => (
            DiagnosticCode::new(CODE_FORBIDDEN_YAML_FEATURE),
            "forbidden YAML feature".into(),
        ),
        ValidationError::UnknownTopLevelField => (
            DiagnosticCode::new(CODE_UNKNOWN_TOP_LEVEL_FIELD),
            "unknown top-level field".into(),
        ),
        ValidationError::UnknownStepField => (
            DiagnosticCode::new(CODE_UNKNOWN_STEP_FIELD),
            "unknown step field".into(),
        ),
        ValidationError::MissingRequiredField { field } => (
            DiagnosticCode::new(CODE_MISSING_REQUIRED_FIELD),
            format!("missing required field: {field}"),
        ),
        ValidationError::InvalidVersion { version } => (
            DiagnosticCode::new(CODE_INVALID_VERSION),
            format!("invalid version: {version}"),
        ),
        ValidationError::InvalidId { id } => (
            DiagnosticCode::new(CODE_INVALID_ID),
            format!("invalid ID: {id}"),
        ),
        ValidationError::ReservedId { id } => (
            DiagnosticCode::new(CODE_RESERVED_ID),
            format!("reserved ID: {id}"),
        ),
        ValidationError::DuplicateId { id } => (
            DiagnosticCode::new(CODE_DUPLICATE_ID),
            format!("duplicate ID: {id}"),
        ),
        ValidationError::MultipleStepPrimitives => (
            DiagnosticCode::new(CODE_MULTIPLE_STEP_PRIMITIVES),
            "multiple step primitives".into(),
        ),
        ValidationError::MissingStepPrimitive => (
            DiagnosticCode::new(CODE_MISSING_STEP_PRIMITIVE),
            "missing step primitive".into(),
        ),
        ValidationError::UnknownReference { reference } => (
            DiagnosticCode::new(CODE_UNKNOWN_REFERENCE),
            format!("unknown reference: {reference}"),
        ),
        ValidationError::FutureReference { reference } => (
            DiagnosticCode::new(CODE_FUTURE_REFERENCE),
            format!("future reference: {reference}"),
        ),
        ValidationError::SecretNotDeclared { secret } => (
            DiagnosticCode::new(CODE_SECRET_NOT_DECLARED),
            format!("secret not declared: {secret}"),
        ),
        ValidationError::DirectRuntimeReference => (
            DiagnosticCode::new(CODE_DIRECT_RUNTIME_REFERENCE),
            "direct runtime reference".into(),
        ),
        ValidationError::InvalidThenTarget => (
            DiagnosticCode::new(CODE_INVALID_THEN_TARGET),
            "invalid then target".into(),
        ),
        ValidationError::ControlFlowCycle => (
            DiagnosticCode::new(CODE_CONTROL_FLOW_CYCLE),
            "control-flow cycle".into(),
        ),
        ValidationError::UnreachableStep { step } => (
            DiagnosticCode::new(CODE_UNREACHABLE_STEP),
            format!("unreachable step: {step}"),
        ),
        ValidationError::InvalidChoose => (
            DiagnosticCode::new(CODE_INVALID_CHOOSE),
            "invalid choose".into(),
        ),
        ValidationError::InvalidForEach => (
            DiagnosticCode::new(CODE_INVALID_FOR_EACH),
            "invalid for_each".into(),
        ),
        ValidationError::InvalidTogether => (
            DiagnosticCode::new(CODE_INVALID_TOGETHER),
            "invalid together".into(),
        ),
        ValidationError::InvalidCollect => (
            DiagnosticCode::new(CODE_INVALID_COLLECT),
            "invalid collect".into(),
        ),
        ValidationError::InvalidReduce => (
            DiagnosticCode::new(CODE_INVALID_REDUCE),
            "invalid reduce".into(),
        ),
        ValidationError::InvalidRepeat => (
            DiagnosticCode::new(CODE_INVALID_REPEAT),
            "invalid repeat".into(),
        ),
        ValidationError::InvalidWait => (
            DiagnosticCode::new(CODE_INVALID_WAIT),
            "invalid wait".into(),
        ),
        ValidationError::InvalidAsk => {
            (DiagnosticCode::new(CODE_INVALID_ASK), "invalid ask".into())
        }
        ValidationError::InvalidFinish => (
            DiagnosticCode::new(CODE_INVALID_FINISH),
            "invalid finish".into(),
        ),
        ValidationError::InvalidRetry => (
            DiagnosticCode::new(CODE_INVALID_RETRY),
            "invalid retry".into(),
        ),
        ValidationError::InvalidOnError => (
            DiagnosticCode::new(CODE_INVALID_ON_ERROR),
            "invalid on_error".into(),
        ),
        ValidationError::SecretResultLeak => (
            DiagnosticCode::new(CODE_SECRET_RESULT_LEAK),
            "secret result leak".into(),
        ),
        ValidationError::TypeMismatch { expected, found } => (
            DiagnosticCode::new(CODE_TYPE_MISMATCH),
            format!("type mismatch: expected {expected}, found {found}"),
        ),
        ValidationError::PayloadTooLarge => (
            DiagnosticCode::new(CODE_PAYLOAD_TOO_LARGE),
            "payload too large".into(),
        ),
        ValidationError::LimitRequired { resource } => (
            DiagnosticCode::new(CODE_LIMIT_REQUIRED),
            format!("limit required: {resource}"),
        ),
        ValidationError::LimitExceeded { resource } => (
            DiagnosticCode::new(CODE_LIMIT_EXCEEDED),
            format!("limit exceeded: {resource}"),
        ),
        ValidationError::UnsupportedTrigger { trigger } => (
            DiagnosticCode::new(CODE_UNSUPPORTED_TRIGGER),
            format!("unsupported trigger: {trigger}"),
        ),
        ValidationError::HttpTriggerOutOfCore => (
            DiagnosticCode::new(CODE_HTTP_TRIGGER_OUT_OF_CORE),
            "HTTP trigger out of core".into(),
        ),
        ValidationError::ExpressionStackExceeded { declared, limit } => (
            DiagnosticCode::new(CODE_EXPRESSION_STACK_EXCEEDED),
            format!("expression stack exceeded: declared {declared}, limit {limit}"),
        ),
        ValidationError::ExpressionStackMismatch {
            expr_index,
            declared,
            computed,
        } => (
            DiagnosticCode::new(CODE_EXPRESSION_STACK_MISMATCH),
            format!(
                "expression stack mismatch: expr {expr_index}, declared {declared}, computed {computed}"
            ),
        ),
        ValidationError::AccessorSlotOutOfRange {
            accessor_index,
            slot,
            slot_count,
        } => (
            DiagnosticCode::new(CODE_ACCESSOR_SLOT_OUT_OF_RANGE),
            format!(
                "accessor slot out of range: accessor {accessor_index}, slot {slot}, slot_count {slot_count}"
            ),
        ),
        ValidationError::AccessorPathInvalid {
            accessor_index,
            segment_index,
        } => (
            DiagnosticCode::new(CODE_ACCESSOR_PATH_INVALID),
            format!("accessor path invalid: accessor {accessor_index}, segment {segment_index}"),
        ),
        ValidationError::AccessorPathTooDeep {
            accessor_index,
            depth,
            max,
        } => (
            DiagnosticCode::new(CODE_ACCESSOR_PATH_TOO_DEEP),
            format!("accessor path too deep: accessor {accessor_index}, depth {depth}, max {max}"),
        ),
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index,
            segment_index,
            symbol,
            symbols_count,
        } => (
            DiagnosticCode::new(CODE_ACCESSOR_SYMBOL_OUT_OF_BOUNDS),
            format!(
                "accessor symbol out of bounds: accessor {accessor_index}, segment {segment_index}, symbol {symbol}, symbols_count {symbols_count}"
            ),
        ),
        ValidationError::SlotReferenceOutOfRange {
            slot,
            slot_count,
            context,
        } => (
            DiagnosticCode::new(CODE_SLOT_REFERENCE_OUT_OF_RANGE),
            format!(
                "slot reference out of range: slot {slot}, slot_count {slot_count}, context {context}"
            ),
        ),
        ValidationError::LoopBodyStepOutOfRange {
            step,
            node_count,
            source_node,
            label,
        } => (
            DiagnosticCode::new(CODE_LOOP_BODY_STEP_OUT_OF_RANGE),
            format!(
                "loop body step out of range: step {step}, node_count {node_count}, source_node {source_node}, label {label}"
            ),
        ),
        ValidationError::SlotDependencyCycle { slot, chain } => (
            DiagnosticCode::new(CODE_SLOT_DEPENDENCY_CYCLE),
            format!("slot dependency cycle: slot {slot}, chain {chain}"),
        ),
        ValidationError::NodeKindConstraintViolation { node_index, detail } => (
            DiagnosticCode::new(CODE_NODE_KIND_CONSTRAINT_VIOLATION),
            format!("node kind constraint violation: node {node_index}, {detail}"),
        ),
        ValidationError::ActionContractMissing {
            action_id,
            node_index,
        } => (
            DiagnosticCode::new(CODE_ACTION_CONTRACT_MISSING),
            format!(
                "action contract missing: action_id {action_id} referenced by Do node {node_index}"
            ),
        ),
        ValidationError::ActionContractOrphan { action_id } => (
            DiagnosticCode::new(CODE_ACTION_CONTRACT_ORPHAN),
            format!("action contract orphan: action_id {action_id} has no corresponding Do node"),
        ),
        ValidationError::CapabilityNameEmpty {
            action_id,
            capability_index,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_NAME_EMPTY),
            format!(
                "capability name is empty for action {action_id} at required_capabilities[{capability_index}]"
            ),
        ),
        ValidationError::CapabilityNameTooLong {
            action_id,
            capability_index,
            len,
            max,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_NAME_TOO_LONG),
            format!(
                "capability name too long for action {action_id} at required_capabilities[{capability_index}]: {len} > {max}"
            ),
        ),
        ValidationError::CapabilityNameInvalid {
            action_id,
            capability_index,
            name,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_NAME_INVALID),
            format!(
                "invalid capability name for action {action_id} at required_capabilities[{capability_index}]: {name}"
            ),
        ),
        ValidationError::CapabilityActionMismatch {
            contract_action_id,
            capability_action_id,
            capability_index,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_ACTION_MISMATCH),
            format!(
                "capability action {capability_action_id} does not match contract action {contract_action_id} at required_capabilities[{capability_index}]"
            ),
        ),
        ValidationError::CapabilityDuplicate {
            action_id,
            first_index,
            duplicate_index,
            name,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_DUPLICATE),
            format!(
                "duplicate capability requirement for action {action_id}: {name} at required_capabilities[{first_index}] and required_capabilities[{duplicate_index}]"
            ),
        ),
        ValidationError::SlotTypeInconsistency { slot } => (
            DiagnosticCode::new(CODE_SLOT_TYPE_INCONSISTENCY),
            format!("slot type inconsistency: slot {slot} has incompatible writers"),
        ),
        ValidationError::NonDeterministicPath { from_node, to_node } => (
            DiagnosticCode::new(CODE_NON_DETERMINISTIC_PATH),
            format!(
                "non-deterministic path: from node {from_node} to node {to_node} contains no suspension point"
            ),
        ),
        ValidationError::MissingSchemaVersion => (
            DiagnosticCode::new(CODE_MISSING_SCHEMA_VERSION),
            "missing schema_version field".into(),
        ),
        ValidationError::CueVetFailed { file } => (
            DiagnosticCode::new(CODE_CUE_VET_FAILED),
            format!("cue vet failed for {file}"),
        ),
        ValidationError::VersionMonotonicityBreach { file, expected, actual } => (
            DiagnosticCode::new(CODE_VERSION_MONOTONICITY_BREACH),
            format!("version monotonicity breach: {file} expected {expected} got {actual}"),
        ),
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
        let diag = diagnostic_from_error(&ValidationError::DuplicateId { id: "step1".into() });
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

    #[test]
    fn all_variants_have_unique_diagnostic_codes() {
        let errors = all_variants();
        let mut seen = std::collections::BTreeSet::new();
        for error in errors {
            let code = error_code(&error).code();
            assert!(seen.insert(code), "duplicate diagnostic code {code:#06x}");
        }
    }

    fn all_variants() -> Vec<ValidationError> {
        vec![
            ValidationError::DuplicateKey,
            ValidationError::ForbiddenYamlFeature,
            ValidationError::UnknownTopLevelField,
            ValidationError::UnknownStepField,
            ValidationError::MissingRequiredField {
                field: "test".into(),
            },
            ValidationError::InvalidVersion {
                version: "v0".into(),
            },
            ValidationError::InvalidId { id: "BAD".into() },
            ValidationError::ReservedId {
                id: "runtime".into(),
            },
            ValidationError::DuplicateId { id: "dup".into() },
            ValidationError::MultipleStepPrimitives,
            ValidationError::MissingStepPrimitive,
            ValidationError::UnknownReference {
                reference: "$x".into(),
            },
            ValidationError::FutureReference {
                reference: "$steps.s".into(),
            },
            ValidationError::SecretNotDeclared {
                secret: "tok".into(),
            },
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
            ValidationError::TypeMismatch {
                expected: "a".into(),
                found: "b".into(),
            },
            ValidationError::PayloadTooLarge,
            ValidationError::LimitRequired {
                resource: "r".into(),
            },
            ValidationError::LimitExceeded {
                resource: "r".into(),
            },
            ValidationError::UnsupportedTrigger {
                trigger: "cron".into(),
            },
            ValidationError::HttpTriggerOutOfCore,
            ValidationError::ExpressionStackExceeded {
                declared: 65,
                limit: 64,
            },
            ValidationError::ExpressionStackMismatch {
                expr_index: 0,
                declared: 2,
                computed: 1,
            },
            ValidationError::AccessorSlotOutOfRange {
                accessor_index: 0,
                slot: 5,
                slot_count: 2,
            },
            ValidationError::AccessorPathInvalid {
                accessor_index: 0,
                segment_index: 1,
            },
            ValidationError::SlotReferenceOutOfRange {
                slot: 99,
                slot_count: 10,
                context: "node 0".into(),
            },
            ValidationError::LoopBodyStepOutOfRange {
                step: 99,
                node_count: 5,
                source_node: 0,
                label: "for_each body".into(),
            },
            ValidationError::SlotDependencyCycle {
                slot: 0,
                chain: "slot 0 -> slot 1 -> slot 0".into(),
            },
            ValidationError::NodeKindConstraintViolation {
                node_index: 0,
                detail: "test".into(),
            },
            ValidationError::ActionContractMissing {
                action_id: 1,
                node_index: 0,
            },
            ValidationError::ActionContractOrphan { action_id: 2 },
            ValidationError::CapabilityNameEmpty {
                action_id: 1,
                capability_index: 0,
            },
            ValidationError::CapabilityNameTooLong {
                action_id: 1,
                capability_index: 0,
                len: 129,
                max: 128,
            },
            ValidationError::CapabilityNameInvalid {
                action_id: 1,
                capability_index: 0,
                name: "network:github".into(),
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
                name: "network".into(),
            },
            ValidationError::SlotTypeInconsistency { slot: 0 },
            ValidationError::NonDeterministicPath {
                from_node: 0,
                to_node: 1,
            },
            ValidationError::MissingSchemaVersion,
            ValidationError::CueVetFailed { file: "test.cue".into() },
            ValidationError::VersionMonotonicityBreach {
                file: "test.cue".into(),
                expected: "v2.0".into(),
                actual: "v1.9".into(),
            },
        ]
    }

    // ---------------------------------------------------------------------------
    // BDD exact-assertion tests
    // ---------------------------------------------------------------------------

    #[test]
    fn diagnostic_from_error_includes_error_code() {
        // Given a ValidationError::DuplicateKey
        let error = ValidationError::DuplicateKey;
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the diagnostic has code E0101
        assert_eq!(diag.code.code(), 0x0101);
    }

    #[test]
    fn diagnostic_from_error_includes_message() {
        // Given a ValidationError::MissingRequiredField
        let error = ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        };
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the message is non-empty and contains the field name
        assert!(!diag.message.is_empty());
        assert!(diag.message.contains("steps"));
    }

    #[test]
    fn diagnostic_from_error_includes_location() {
        // Given any ValidationError
        let error = ValidationError::ControlFlowCycle;
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the span is present (ZERO for now but always set)
        assert_eq!(diag.span, Span::ZERO);
    }

    #[test]
    fn error_code_returns_known_code_for_duplicate_key() {
        // Given a ValidationError::DuplicateKey
        let error = ValidationError::DuplicateKey;
        // When error_code is called
        let code = error_code(&error);
        // Then it returns code E0101 (0x0101)
        assert_eq!(code.code(), 0x0101);
    }

    #[test]
    fn error_code_returns_known_code_for_missing_required_field() {
        // Given a ValidationError::MissingRequiredField
        let error = ValidationError::MissingRequiredField {
            field: "version".to_owned(),
        };
        // When error_code is called
        let code = error_code(&error);
        // Then it returns code E0105 (0x0105)
        assert_eq!(code.code(), 0x0105);
    }

    #[test]
    fn error_code_is_non_empty_for_all_variants() {
        // Given all ValidationError variants
        let errors = all_variants();
        // When error_code is called for each
        // Then every variant produces a non-zero code
        for error in &errors {
            let code = error_code(error).code();
            assert_ne!(code, 0, "error_code returned 0 for {error:?}");
        }
    }

    #[test]
    fn diagnostic_from_error_for_invalid_id_includes_id() {
        // Given a ValidationError::InvalidId
        let error = ValidationError::InvalidId {
            id: "bad-id".to_owned(),
        };
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the message contains the id
        assert_eq!(diag.code.code(), 0x0107);
        assert!(diag.message.contains("bad-id"));
    }

    #[test]
    fn diagnostic_from_error_for_reserved_id_includes_id() {
        // Given a ValidationError::ReservedId
        let error = ValidationError::ReservedId {
            id: "runtime".to_owned(),
        };
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the message contains the id
        assert_eq!(diag.code.code(), 0x0108);
        assert!(diag.message.contains("runtime"));
    }

    #[test]
    fn diagnostic_from_error_for_duplicate_id_includes_id() {
        // Given a ValidationError::DuplicateId
        let error = ValidationError::DuplicateId {
            id: "step1".to_owned(),
        };
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the message contains the id
        assert_eq!(diag.code.code(), 0x0109);
        assert!(diag.message.contains("step1"));
    }

    #[test]
    fn diagnostic_from_error_for_unknown_reference_includes_reference() {
        // Given a ValidationError::UnknownReference
        let error = ValidationError::UnknownReference {
            reference: "$input.missing".to_owned(),
        };
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the message contains the reference
        assert_eq!(diag.code.code(), 0x0201);
        assert!(diag.message.contains("$input.missing"));
    }

    #[test]
    fn diagnostic_from_error_for_future_reference_includes_reference() {
        // Given a ValidationError::FutureReference
        let error = ValidationError::FutureReference {
            reference: "$steps.build".to_owned(),
        };
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the message contains the reference
        assert_eq!(diag.code.code(), 0x0202);
        assert!(diag.message.contains("$steps.build"));
    }

    #[test]
    fn diagnostic_from_error_for_limit_exceeded_includes_resource() {
        // Given a ValidationError::LimitExceeded
        let error = ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        };
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the message contains the resource
        assert_eq!(diag.code.code(), 0x040A);
        assert!(diag.message.contains("max_steps"));
    }

    #[test]
    fn diagnostic_from_error_for_unsupported_trigger_includes_trigger() {
        // Given a ValidationError::UnsupportedTrigger
        let error = ValidationError::UnsupportedTrigger {
            trigger: "cron".to_owned(),
        };
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the message contains the trigger
        assert_eq!(diag.code.code(), 0x040B);
        assert!(diag.message.contains("cron"));
    }

    #[test]
    fn diagnostic_severity_is_always_error() {
        // Given all ValidationError variants
        let errors = all_variants();
        // When diagnostic_from_error is called for each
        // Then the severity is always Error
        for error in &errors {
            let diag = diagnostic_from_error(error);
            assert_eq!(
                diag.severity,
                Severity::Error,
                "wrong severity for {error:?}"
            );
        }
    }

    #[test]
    fn diagnostic_from_error_for_type_mismatch_includes_both_types() {
        // Given a ValidationError::TypeMismatch
        let error = ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
        };
        // When diagnostic_from_error is called
        let diag = diagnostic_from_error(&error);
        // Then the message contains both type names
        assert_eq!(diag.code.code(), 0x0407);
        assert!(diag.message.contains("boolean"));
        assert!(diag.message.contains("number"));
    }
}
