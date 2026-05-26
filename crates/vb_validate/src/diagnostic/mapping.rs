#![forbid(unsafe_code)]
//! Error-to-diagnostic mapping.
//!
//! Canonical implementation of `diagnostic_from_error` and `error_code`
//! that converts `ValidationError` variants into stable `Diagnostic` records.

use crate::ValidationError;
use vb_core::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use vb_core::span::Span;
use vb_yaml::source_map::SemanticSourceMap;

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
///
/// Propagates the error's [`Span`] into the diagnostic. When the error
/// carries `Span::ZERO`, the diagnostic falls back to `Span::ZERO` —
/// backward compatible with prior behavior.
///
/// When `semantic_map` is provided and the error's span is non-zero,
/// the YAML author path is looked up from the map. If found, it is
/// appended to the diagnostic message (e.g., `"unknown field at path $.inputs"`).
/// Path annotation is additive only — it SHALL NOT replace the primary
/// error message. When the map is absent or no matching path exists,
/// the message is un-annotated.
pub fn diagnostic_from_error(
    error: &ValidationError,
    semantic_map: Option<&SemanticSourceMap>,
) -> Diagnostic {
    let (code, message, span) = error_diagnostic_parts(error);
    let annotated_message = if span != Span::ZERO {
        if let Some(map) = semantic_map {
            // u32 → usize is always safe on 64-bit platforms; use
            // checked conversion for zero-panic compliance on all targets.
            let start_offset = match usize::try_from(span.start) {
                Ok(v) => v,
                Err(_) => {
                    return Diagnostic::new(code, message.into(), Severity::Error, span, None);
                }
            };
            let end_offset = match usize::try_from(span.end) {
                Ok(v) => v,
                Err(_) => {
                    return Diagnostic::new(code, message.into(), Severity::Error, span, None);
                }
            };
            if let Some(path) = map.find_path_for_offset(start_offset, end_offset) {
                format!("{message} at path {path}")
            } else {
                message
            }
        } else {
            message
        }
    } else {
        message
    };
    Diagnostic::new(code, annotated_message.into(), Severity::Error, span, None)
}

/// Returns the stable diagnostic code for a validation error.
pub fn error_code(error: &ValidationError) -> DiagnosticCode {
    let (code, _, _) = error_diagnostic_parts(error);
    code
}

// ---------------------------------------------------------------------------
// Internal mapping
// ---------------------------------------------------------------------------

fn error_diagnostic_parts(error: &ValidationError) -> (DiagnosticCode, String, Span) {
    match error {
        ValidationError::DuplicateKey { span } => (
            DiagnosticCode::new(CODE_DUPLICATE_KEY),
            "duplicate key".into(),
            *span,
        ),
        ValidationError::ForbiddenYamlFeature { span } => (
            DiagnosticCode::new(CODE_FORBIDDEN_YAML_FEATURE),
            "forbidden YAML feature".into(),
            *span,
        ),
        ValidationError::UnknownTopLevelField { span } => (
            DiagnosticCode::new(CODE_UNKNOWN_TOP_LEVEL_FIELD),
            "unknown top-level field".into(),
            *span,
        ),
        ValidationError::UnknownStepField { span } => (
            DiagnosticCode::new(CODE_UNKNOWN_STEP_FIELD),
            "unknown step field".into(),
            *span,
        ),
        ValidationError::MissingRequiredField { field, span } => (
            DiagnosticCode::new(CODE_MISSING_REQUIRED_FIELD),
            format!("missing required field: {field}"),
            *span,
        ),
        ValidationError::InvalidVersion { version, span } => (
            DiagnosticCode::new(CODE_INVALID_VERSION),
            format!("invalid version: {version}"),
            *span,
        ),
        ValidationError::InvalidId { id, span } => (
            DiagnosticCode::new(CODE_INVALID_ID),
            format!("invalid ID: {id}"),
            *span,
        ),
        ValidationError::ReservedId { id, span } => (
            DiagnosticCode::new(CODE_RESERVED_ID),
            format!("reserved ID: {id}"),
            *span,
        ),
        ValidationError::DuplicateId { id, span } => (
            DiagnosticCode::new(CODE_DUPLICATE_ID),
            format!("duplicate ID: {id}"),
            *span,
        ),
        ValidationError::MultipleStepPrimitives { span } => (
            DiagnosticCode::new(CODE_MULTIPLE_STEP_PRIMITIVES),
            "multiple step primitives".into(),
            *span,
        ),
        ValidationError::MissingStepPrimitive { span } => (
            DiagnosticCode::new(CODE_MISSING_STEP_PRIMITIVE),
            "missing step primitive".into(),
            *span,
        ),
        ValidationError::UnknownReference { reference, span } => (
            DiagnosticCode::new(CODE_UNKNOWN_REFERENCE),
            format!("unknown reference: {reference}"),
            *span,
        ),
        ValidationError::FutureReference { reference, span } => (
            DiagnosticCode::new(CODE_FUTURE_REFERENCE),
            format!("future reference: {reference}"),
            *span,
        ),
        ValidationError::SecretNotDeclared { secret, span } => (
            DiagnosticCode::new(CODE_SECRET_NOT_DECLARED),
            format!("secret not declared: {secret}"),
            *span,
        ),
        ValidationError::DirectRuntimeReference { span } => (
            DiagnosticCode::new(CODE_DIRECT_RUNTIME_REFERENCE),
            "direct runtime reference".into(),
            *span,
        ),
        ValidationError::InvalidThenTarget { span } => (
            DiagnosticCode::new(CODE_INVALID_THEN_TARGET),
            "invalid then target".into(),
            *span,
        ),
        ValidationError::ControlFlowCycle { span } => (
            DiagnosticCode::new(CODE_CONTROL_FLOW_CYCLE),
            "control-flow cycle".into(),
            *span,
        ),
        ValidationError::UnreachableStep { step, span } => (
            DiagnosticCode::new(CODE_UNREACHABLE_STEP),
            format!("unreachable step: {step}"),
            *span,
        ),
        ValidationError::InvalidChoose { span } => (
            DiagnosticCode::new(CODE_INVALID_CHOOSE),
            "invalid choose".into(),
            *span,
        ),
        ValidationError::InvalidForEach { span } => (
            DiagnosticCode::new(CODE_INVALID_FOR_EACH),
            "invalid for_each".into(),
            *span,
        ),
        ValidationError::InvalidTogether { span } => (
            DiagnosticCode::new(CODE_INVALID_TOGETHER),
            "invalid together".into(),
            *span,
        ),
        ValidationError::InvalidCollect { span } => (
            DiagnosticCode::new(CODE_INVALID_COLLECT),
            "invalid collect".into(),
            *span,
        ),
        ValidationError::InvalidReduce { span } => (
            DiagnosticCode::new(CODE_INVALID_REDUCE),
            "invalid reduce".into(),
            *span,
        ),
        ValidationError::InvalidRepeat { span } => (
            DiagnosticCode::new(CODE_INVALID_REPEAT),
            "invalid repeat".into(),
            *span,
        ),
        ValidationError::InvalidWait { span } => (
            DiagnosticCode::new(CODE_INVALID_WAIT),
            "invalid wait".into(),
            *span,
        ),
        ValidationError::InvalidAsk { span } => (
            DiagnosticCode::new(CODE_INVALID_ASK),
            "invalid ask".into(),
            *span,
        ),
        ValidationError::InvalidFinish { span } => (
            DiagnosticCode::new(CODE_INVALID_FINISH),
            "invalid finish".into(),
            *span,
        ),
        ValidationError::InvalidRetry { span } => (
            DiagnosticCode::new(CODE_INVALID_RETRY),
            "invalid retry".into(),
            *span,
        ),
        ValidationError::InvalidOnError { span } => (
            DiagnosticCode::new(CODE_INVALID_ON_ERROR),
            "invalid on_error".into(),
            *span,
        ),
        ValidationError::SecretResultLeak { span } => (
            DiagnosticCode::new(CODE_SECRET_RESULT_LEAK),
            "secret result leak".into(),
            *span,
        ),
        ValidationError::TypeMismatch {
            expected,
            found,
            span,
        } => (
            DiagnosticCode::new(CODE_TYPE_MISMATCH),
            format!("type mismatch: expected {expected}, found {found}"),
            *span,
        ),
        ValidationError::PayloadTooLarge { span } => (
            DiagnosticCode::new(CODE_PAYLOAD_TOO_LARGE),
            "payload too large".into(),
            *span,
        ),
        ValidationError::LimitRequired { resource, span } => (
            DiagnosticCode::new(CODE_LIMIT_REQUIRED),
            format!("limit required: {resource}"),
            *span,
        ),
        ValidationError::LimitExceeded { resource, span } => (
            DiagnosticCode::new(CODE_LIMIT_EXCEEDED),
            format!("limit exceeded: {resource}"),
            *span,
        ),
        ValidationError::UnsupportedTrigger { trigger, span } => (
            DiagnosticCode::new(CODE_UNSUPPORTED_TRIGGER),
            format!("unsupported trigger: {trigger}"),
            *span,
        ),
        ValidationError::HttpTriggerOutOfCore { span } => (
            DiagnosticCode::new(CODE_HTTP_TRIGGER_OUT_OF_CORE),
            "HTTP trigger out of core".into(),
            *span,
        ),
        ValidationError::ExpressionStackExceeded {
            declared,
            limit,
            span,
        } => (
            DiagnosticCode::new(CODE_EXPRESSION_STACK_EXCEEDED),
            format!("expression stack exceeded: declared {declared}, limit {limit}"),
            *span,
        ),
        ValidationError::ExpressionStackMismatch {
            expr_index,
            declared,
            computed,
            span,
        } => (
            DiagnosticCode::new(CODE_EXPRESSION_STACK_MISMATCH),
            format!(
                "expression stack mismatch: expr {expr_index}, declared {declared}, computed {computed}"
            ),
            *span,
        ),
        ValidationError::AccessorSlotOutOfRange {
            accessor_index,
            slot,
            slot_count,
            span,
        } => (
            DiagnosticCode::new(CODE_ACCESSOR_SLOT_OUT_OF_RANGE),
            format!(
                "accessor slot out of range: accessor {accessor_index}, slot {slot}, slot_count {slot_count}"
            ),
            *span,
        ),
        ValidationError::AccessorPathInvalid {
            accessor_index,
            segment_index,
            span,
        } => (
            DiagnosticCode::new(CODE_ACCESSOR_PATH_INVALID),
            format!("accessor path invalid: accessor {accessor_index}, segment {segment_index}"),
            *span,
        ),
        ValidationError::AccessorPathTooDeep {
            accessor_index,
            depth,
            max,
            span,
        } => (
            DiagnosticCode::new(CODE_ACCESSOR_PATH_TOO_DEEP),
            format!("accessor path too deep: accessor {accessor_index}, depth {depth}, max {max}"),
            *span,
        ),
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index,
            segment_index,
            symbol,
            symbols_count,
            span,
        } => (
            DiagnosticCode::new(CODE_ACCESSOR_SYMBOL_OUT_OF_BOUNDS),
            format!(
                "accessor symbol out of bounds: accessor {accessor_index}, segment {segment_index}, symbol {symbol}, symbols_count {symbols_count}"
            ),
            *span,
        ),
        ValidationError::SlotReferenceOutOfRange {
            slot,
            slot_count,
            context,
            span,
        } => (
            DiagnosticCode::new(CODE_SLOT_REFERENCE_OUT_OF_RANGE),
            format!(
                "slot reference out of range: slot {slot}, slot_count {slot_count}, context {context}"
            ),
            *span,
        ),
        ValidationError::LoopBodyStepOutOfRange {
            step,
            node_count,
            source_node,
            label,
            span,
        } => (
            DiagnosticCode::new(CODE_LOOP_BODY_STEP_OUT_OF_RANGE),
            format!(
                "loop body step out of range: step {step}, node_count {node_count}, source_node {source_node}, label {label}"
            ),
            *span,
        ),
        ValidationError::SlotDependencyCycle { slot, chain, span } => (
            DiagnosticCode::new(CODE_SLOT_DEPENDENCY_CYCLE),
            format!("slot dependency cycle: slot {slot}, chain {chain}"),
            *span,
        ),
        ValidationError::NodeKindConstraintViolation {
            node_index,
            detail,
            span,
        } => (
            DiagnosticCode::new(CODE_NODE_KIND_CONSTRAINT_VIOLATION),
            format!("node kind constraint violation: node {node_index}, {detail}"),
            *span,
        ),
        ValidationError::ActionContractMissing {
            action_id,
            node_index,
            span,
        } => (
            DiagnosticCode::new(CODE_ACTION_CONTRACT_MISSING),
            format!(
                "action contract missing: action_id {action_id} referenced by Do node {node_index}"
            ),
            *span,
        ),
        ValidationError::ActionContractOrphan { action_id, span } => (
            DiagnosticCode::new(CODE_ACTION_CONTRACT_ORPHAN),
            format!("action contract orphan: action_id {action_id} has no corresponding Do node"),
            *span,
        ),
        ValidationError::CapabilityNameEmpty {
            action_id,
            capability_index,
            span,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_NAME_EMPTY),
            format!(
                "capability name is empty for action {action_id} at required_capabilities[{capability_index}]"
            ),
            *span,
        ),
        ValidationError::CapabilityNameTooLong {
            action_id,
            capability_index,
            len,
            max,
            span,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_NAME_TOO_LONG),
            format!(
                "capability name too long for action {action_id} at required_capabilities[{capability_index}]: {len} > {max}"
            ),
            *span,
        ),
        ValidationError::CapabilityNameInvalid {
            action_id,
            capability_index,
            name,
            span,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_NAME_INVALID),
            format!(
                "invalid capability name for action {action_id} at required_capabilities[{capability_index}]: {name}"
            ),
            *span,
        ),
        ValidationError::CapabilityActionMismatch {
            contract_action_id,
            capability_action_id,
            capability_index,
            span,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_ACTION_MISMATCH),
            format!(
                "capability action {capability_action_id} does not match contract action {contract_action_id} at required_capabilities[{capability_index}]"
            ),
            *span,
        ),
        ValidationError::CapabilityDuplicate {
            action_id,
            first_index,
            duplicate_index,
            name,
            span,
        } => (
            DiagnosticCode::new(CODE_CAPABILITY_DUPLICATE),
            format!(
                "duplicate capability requirement for action {action_id}: {name} at required_capabilities[{first_index}] and required_capabilities[{duplicate_index}]"
            ),
            *span,
        ),
        ValidationError::SlotTypeInconsistency { slot, span } => (
            DiagnosticCode::new(CODE_SLOT_TYPE_INCONSISTENCY),
            format!("slot type inconsistency: slot {slot} has incompatible writers"),
            *span,
        ),
        ValidationError::NonDeterministicPath {
            from_node,
            to_node,
            span,
        } => (
            DiagnosticCode::new(CODE_NON_DETERMINISTIC_PATH),
            format!(
                "non-deterministic path: from node {from_node} to node {to_node} contains no suspension point"
            ),
            *span,
        ),
        ValidationError::MissingSchemaVersion { span } => (
            DiagnosticCode::new(CODE_MISSING_SCHEMA_VERSION),
            "missing schema_version field".into(),
            *span,
        ),
        ValidationError::CueVetFailed { file, span } => (
            DiagnosticCode::new(CODE_CUE_VET_FAILED),
            format!("cue vet failed for {file}"),
            *span,
        ),
        ValidationError::VersionMonotonicityBreach {
            file,
            expected,
            actual,
            span,
        } => (
            DiagnosticCode::new(CODE_VERSION_MONOTONICITY_BREACH),
            format!("version monotonicity breach: {file} expected {expected} got {actual}"),
            *span,
        ),
    }
}
