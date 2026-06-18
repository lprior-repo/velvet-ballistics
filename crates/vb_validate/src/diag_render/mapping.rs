#![forbid(unsafe_code)]
//! ValidationError → diagnostic code & message mapping.
//!
//! Every `ValidationError` variant maps to a stable [`DiagnosticCode`] and a
//! human-readable message. The codes follow the Section 16 master contract:
//!
//! - E01xx — Schema validation errors
//! - E02xx — Reference validation errors
//! - E03xx — Control-flow errors
//! - E04xx — Type/taint/resource errors
//! - E05xx — Gate verifier errors
//! - E06xx — Contract-discovery errors

#![allow(unreachable_pub)]
use crate::ValidationError;

use crate::diag_codes::*;
use vb_core::diagnostic::DiagnosticCode;

/// Maps each [`ValidationError`] variant to its stable diagnostic code and message.
///
/// Every `DiagnosticCode` produced here is registered in `CODE_REGISTRY` via the
/// corresponding `CODE_*` constant imported from `diag_codes`.
pub(super) fn error_diagnostic_parts(error: &ValidationError) -> (DiagnosticCode, String) {
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
        ValidationError::ResultReferenceMissing {
            step,
            missing_output,
        } => (
            DiagnosticCode::new(CODE_RESULT_REFERENCE_MISSING),
            format!(
                "step {step} does not produce an output; cannot reference field symbol {missing_output:?}"
            ),
        ),
        ValidationError::UnsupportedStepField { step, field } => (
            DiagnosticCode::new(CODE_UNSUPPORTED_STEP_FIELD),
            format!(
                "step `{step}` does not expose a `{field}` field; allowed fields are `output` and `result`"
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
        ValidationError::InvalidAsk => (
            DiagnosticCode::new(CODE_INVALID_ASK),
            "invalid ask".into(),
        ),
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
            format!(
                "accessor path too deep: accessor {accessor_index}, depth {depth}, max {max}"
            ),
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
