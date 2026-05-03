//! Diagnostic rendering for validation errors.
//!
//! Converts `ValidationError` variants into stable `Diagnostic` records with
//! error codes matching the master contract (Section 16).

use crate::ValidationError;
use vb_core::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use vb_core::span::Span;

use super::diag_codes::*;

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

/// Maps each ValidationError variant to its stable diagnostic code and message.
fn error_diagnostic_parts(error: &ValidationError) -> (DiagnosticCode, String) {
    match error {
        ValidationError::DuplicateKey => (DiagnosticCode::new(CODE_DUPLICATE_KEY), "duplicate key".into()),
        ValidationError::ForbiddenYamlFeature => (DiagnosticCode::new(CODE_FORBIDDEN_YAML_FEATURE), "forbidden YAML feature".into()),
        ValidationError::UnknownTopLevelField => (DiagnosticCode::new(CODE_UNKNOWN_TOP_LEVEL_FIELD), "unknown top-level field".into()),
        ValidationError::UnknownStepField => (DiagnosticCode::new(CODE_UNKNOWN_STEP_FIELD), "unknown step field".into()),
        ValidationError::MissingRequiredField { field } => (DiagnosticCode::new(CODE_MISSING_REQUIRED_FIELD), format!("missing required field: {field}")),
        ValidationError::InvalidVersion { version } => (DiagnosticCode::new(CODE_INVALID_VERSION), format!("invalid version: {version}")),
        ValidationError::InvalidId { id } => (DiagnosticCode::new(CODE_INVALID_ID), format!("invalid ID: {id}")),
        ValidationError::ReservedId { id } => (DiagnosticCode::new(CODE_RESERVED_ID), format!("reserved ID: {id}")),
        ValidationError::DuplicateId { id } => (DiagnosticCode::new(CODE_DUPLICATE_ID), format!("duplicate ID: {id}")),
        ValidationError::MultipleStepPrimitives => (DiagnosticCode::new(CODE_MULTIPLE_STEP_PRIMITIVES), "multiple step primitives".into()),
        ValidationError::MissingStepPrimitive => (DiagnosticCode::new(CODE_MISSING_STEP_PRIMITIVE), "missing step primitive".into()),
        ValidationError::UnknownReference { reference } => (DiagnosticCode::new(CODE_UNKNOWN_REFERENCE), format!("unknown reference: {reference}")),
        ValidationError::FutureReference { reference } => (DiagnosticCode::new(CODE_FUTURE_REFERENCE), format!("future reference: {reference}")),
        ValidationError::SecretNotDeclared { secret } => (DiagnosticCode::new(CODE_SECRET_NOT_DECLARED), format!("secret not declared: {secret}")),
        ValidationError::DirectRuntimeReference => (DiagnosticCode::new(CODE_DIRECT_RUNTIME_REFERENCE), "direct runtime reference".into()),
        ValidationError::InvalidThenTarget => (DiagnosticCode::new(CODE_INVALID_THEN_TARGET), "invalid then target".into()),
        ValidationError::ControlFlowCycle => (DiagnosticCode::new(CODE_CONTROL_FLOW_CYCLE), "control-flow cycle".into()),
        ValidationError::UnreachableStep { step } => (DiagnosticCode::new(CODE_UNREACHABLE_STEP), format!("unreachable step: {step}")),
        ValidationError::InvalidChoose => (DiagnosticCode::new(CODE_INVALID_CHOOSE), "invalid choose".into()),
        ValidationError::InvalidForEach => (DiagnosticCode::new(CODE_INVALID_FOR_EACH), "invalid for_each".into()),
        ValidationError::InvalidTogether => (DiagnosticCode::new(CODE_INVALID_TOGETHER), "invalid together".into()),
        ValidationError::InvalidCollect => (DiagnosticCode::new(CODE_INVALID_COLLECT), "invalid collect".into()),
        ValidationError::InvalidReduce => (DiagnosticCode::new(CODE_INVALID_REDUCE), "invalid reduce".into()),
        ValidationError::InvalidRepeat => (DiagnosticCode::new(CODE_INVALID_REPEAT), "invalid repeat".into()),
        ValidationError::InvalidWait => (DiagnosticCode::new(CODE_INVALID_WAIT), "invalid wait".into()),
        ValidationError::InvalidAsk => (DiagnosticCode::new(CODE_INVALID_ASK), "invalid ask".into()),
        ValidationError::InvalidFinish => (DiagnosticCode::new(CODE_INVALID_FINISH), "invalid finish".into()),
        ValidationError::InvalidRetry => (DiagnosticCode::new(CODE_INVALID_RETRY), "invalid retry".into()),
        ValidationError::InvalidOnError => (DiagnosticCode::new(CODE_INVALID_ON_ERROR), "invalid on_error".into()),
        ValidationError::SecretResultLeak => (DiagnosticCode::new(CODE_SECRET_RESULT_LEAK), "secret result leak".into()),
        ValidationError::TypeMismatch { expected, found } => (DiagnosticCode::new(CODE_TYPE_MISMATCH), format!("type mismatch: expected {expected}, found {found}")),
        ValidationError::PayloadTooLarge => (DiagnosticCode::new(CODE_PAYLOAD_TOO_LARGE), "payload too large".into()),
        ValidationError::LimitRequired { resource } => (DiagnosticCode::new(CODE_LIMIT_REQUIRED), format!("limit required: {resource}")),
        ValidationError::LimitExceeded { resource } => (DiagnosticCode::new(CODE_LIMIT_EXCEEDED), format!("limit exceeded: {resource}")),
        ValidationError::UnsupportedTrigger { trigger } => (DiagnosticCode::new(CODE_UNSUPPORTED_TRIGGER), format!("unsupported trigger: {trigger}")),
        ValidationError::HttpTriggerOutOfCore => (DiagnosticCode::new(CODE_HTTP_TRIGGER_OUT_OF_CORE), "HTTP trigger out of core".into()),
        ValidationError::ExpressionStackExceeded { declared, limit } => (DiagnosticCode::new(CODE_EXPRESSION_STACK_EXCEEDED), format!("expression stack exceeded: declared {declared}, limit {limit}")),
        ValidationError::ExpressionStackMismatch { expr_index, declared, computed } => (DiagnosticCode::new(CODE_EXPRESSION_STACK_MISMATCH), format!("expression stack mismatch: expr {expr_index}, declared {declared}, computed {computed}")),
        ValidationError::AccessorSlotOutOfRange { accessor_index, slot, slot_count } => (DiagnosticCode::new(CODE_ACCESSOR_SLOT_OUT_OF_RANGE), format!("accessor slot out of range: accessor {accessor_index}, slot {slot}, slot_count {slot_count}")),
        ValidationError::AccessorPathInvalid { accessor_index, segment_index } => (DiagnosticCode::new(CODE_ACCESSOR_PATH_INVALID), format!("accessor path invalid: accessor {accessor_index}, segment {segment_index}")),
        ValidationError::SlotReferenceOutOfRange { slot, slot_count, context } => (DiagnosticCode::new(CODE_SLOT_REFERENCE_OUT_OF_RANGE), format!("slot reference out of range: slot {slot}, slot_count {slot_count}, context {context}")),
        ValidationError::LoopBodyStepOutOfRange { step, node_count, source_node, label } => (DiagnosticCode::new(CODE_LOOP_BODY_STEP_OUT_OF_RANGE), format!("loop body step out of range: step {step}, node_count {node_count}, source_node {source_node}, label {label}")),
        ValidationError::SlotDependencyCycle { slot, chain } => (DiagnosticCode::new(CODE_SLOT_DEPENDENCY_CYCLE), format!("slot dependency cycle: slot {slot}, chain {chain}")),
        ValidationError::NodeKindConstraintViolation { node_index, detail } => (DiagnosticCode::new(CODE_NODE_KIND_CONSTRAINT_VIOLATION), format!("node kind constraint violation: node {node_index}, {detail}")),
        ValidationError::ActionContractMissing { action_id, node_index } => (DiagnosticCode::new(CODE_ACTION_CONTRACT_MISSING), format!("action contract missing: action_id {action_id} referenced by Do node {node_index}")),
        ValidationError::ActionContractOrphan { action_id } => (DiagnosticCode::new(CODE_ACTION_CONTRACT_ORPHAN), format!("action contract orphan: action_id {action_id} has no corresponding Do node")),
        ValidationError::SlotTypeInconsistency { slot } => (DiagnosticCode::new(CODE_SLOT_TYPE_INCONSISTENCY), format!("slot type inconsistency: slot {slot} has incompatible writers")),
        ValidationError::NonDeterministicPath { from_node, to_node } => (DiagnosticCode::new(CODE_NON_DETERMINISTIC_PATH), format!("non-deterministic path: from node {from_node} to node {to_node} contains no suspension point")),
    }
}
