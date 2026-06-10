#![forbid(unsafe_code)]
//! Diagnostic rendering for validation errors.
//!
//! Converts `ValidationError` variants into stable `Diagnostic` records with
//! error codes matching the master contract (Section 16).

#![allow(unreachable_pub)]
use crate::ValidationError;
use vb_core::diagnostic::{Diagnostic, DiagnosticCode, Severity, SymbolicCode};
use vb_core::span::Span;

use crate::diag_codes::*;

/// Converts a validation error into a diagnostic record.
pub fn diagnostic_from_error(error: &ValidationError) -> Diagnostic {
    let (code, message) = error_diagnostic_parts(error);
    // All codes from error_diagnostic_parts are registered in CODE_REGISTRY.
    diagnostic_from_parts(code, message, Severity::Error, Span::ZERO)
}

/// Fallback SymbolicCode used when a registered code lookup fails.
fn diagnostic_fallback_symbolic() -> SymbolicCode {
    use std::sync::OnceLock;
    static FALLBACK: OnceLock<SymbolicCode> = OnceLock::new();
    *FALLBACK.get_or_init(
        || match SymbolicCode::from_static("MISSING_REQUIRED_FIELD") {
            Some(c) => c,
            None => {
                std::process::abort();
            }
        },
    )
}

/// Internal helper: constructs a Diagnostic from a DiagnosticCode,
/// resolving the symbolic code from the registry.
fn diagnostic_from_parts(
    code: DiagnosticCode,
    message: String,
    severity: Severity,
    span: Span,
) -> Diagnostic {
    match code.symbolic_code() {
        Some(sc) => Diagnostic::new(sc, message.into(), severity, span, None),
        None => {
            let fallback = diagnostic_fallback_symbolic();
            let annotated = format!("[unregistered {:04X}] {}", code.code(), message);
            Diagnostic::new(fallback, annotated.into(), severity, span, None)
        }
    }
}

/// Returns the stable diagnostic code for a validation error.
pub fn error_code(error: &ValidationError) -> DiagnosticCode {
    let (code, _) = error_diagnostic_parts(error);
    code
}

/// Maps each ValidationError variant to its stable diagnostic code and message.
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
        ValidationError::ScopeGuardViolation {
            reference,
            required_scope,
        } => (
            DiagnosticCode::new(CODE_SCOPE_GUARD_VIOLATION),
            format!("scope guard violation: {reference} requires {required_scope} scope"),
        ),
        ValidationError::DirectLoopReference { variable } => (
            DiagnosticCode::new(CODE_DIRECT_LOOP_REFERENCE),
            format!("loop variables must use the `$loop.<var>` prefix (found `${variable}`)"),
        ),
        ValidationError::DirectStepReference { step } => (
            DiagnosticCode::new(CODE_DIRECT_STEP_REFERENCE),
            format!("step reference `{step}` must use the `$steps.X` prefix"),
        ),
        ValidationError::StepSkippedReference { step, reference } => (
            DiagnosticCode::new(CODE_STEP_SKIPPED_REFERENCE),
            format!(
                "step {} skipped due to unresolved reference `{reference}`",
                step.get()
            ),
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
        ValidationError::VersionMonotonicityBreach {
            file,
            expected,
            actual,
        } => (
            DiagnosticCode::new(CODE_VERSION_MONOTONICITY_BREACH),
            format!("version monotonicity breach: {file} expected {expected} got {actual}"),
        ),
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::diag_convert::all_variants;
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
}
