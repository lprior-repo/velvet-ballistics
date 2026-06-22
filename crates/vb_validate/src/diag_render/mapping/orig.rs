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
///
/// The match is split into one helper per Section 16 code family:
/// `map_schema_*` (E01xx), `map_reference_*` (E02xx), `map_control_flow_*`
/// (E03xx), `map_type_taint_resource_*` (E04xx), `map_gate_*` (E05xx),
/// `map_contract_*` (E06xx). Each helper is under 50 lines and keeps the
/// diagnostic mapping for a single family in one place.
pub fn error_diagnostic_parts(error: &ValidationError) -> (DiagnosticCode, String) {
    match error {
        ValidationError::DuplicateKey => map_schema_duplicate_key(),
        ValidationError::ForbiddenYamlFeature => map_schema_forbidden_yaml(),
        ValidationError::UnknownTopLevelField => map_schema_unknown_top_level(),
        ValidationError::UnknownStepField => map_schema_unknown_step_field(),
        ValidationError::MissingRequiredField { field } => map_schema_missing_required_field(field),
        ValidationError::InvalidVersion { version } => map_schema_invalid_version(version),
        ValidationError::InvalidId { id } => map_schema_invalid_id(id),
        ValidationError::ReservedId { id } => map_schema_reserved_id(id),
        ValidationError::DuplicateId { id } => map_schema_duplicate_id(id),
        ValidationError::MultipleStepPrimitives => map_schema_multiple_step_primitives(),
        ValidationError::MissingStepPrimitive => map_schema_missing_step_primitive(),
        ValidationError::UnknownReference { reference } => map_reference_unknown(reference),
        ValidationError::FutureReference { reference } => map_reference_future(reference),
        ValidationError::SecretNotDeclared { secret } => map_reference_secret_not_declared(secret),
        ValidationError::DirectRuntimeReference => map_reference_direct_runtime(),
        ValidationError::ScopeGuardViolation {
            reference,
            required_scope,
        } => map_reference_scope_guard(reference, required_scope),
        ValidationError::DirectLoopReference { variable } => map_reference_direct_loop(variable),
        ValidationError::DirectStepReference { step } => map_reference_direct_step(step),
        ValidationError::StepSkippedReference { step, reference } => {
            map_reference_step_skipped(usize::from(step.get()), reference)
        }
        ValidationError::ResultReferenceMissing {
            step,
            missing_output,
        } => map_reference_result_missing(step, *missing_output),
        ValidationError::UnsupportedStepField { step, field } => {
            map_reference_unsupported_step_field(step, field)
        }
        ValidationError::InvalidThenTarget => map_control_flow_invalid_then(),
        ValidationError::ControlFlowCycle => map_control_flow_cycle(),
        ValidationError::UnreachableStep { step } => map_control_flow_unreachable(step),
        ValidationError::InvalidChoose => map_control_flow_invalid_choose(),
        ValidationError::InvalidForEach => map_control_flow_invalid_for_each(),
        ValidationError::InvalidTogether => map_control_flow_invalid_together(),
        ValidationError::InvalidCollect => map_control_flow_invalid_collect(),
        ValidationError::InvalidReduce => map_control_flow_invalid_reduce(),
        ValidationError::InvalidRepeat => map_control_flow_invalid_repeat(),
        ValidationError::InvalidWait => map_control_flow_invalid_wait(),
        ValidationError::InvalidAsk => map_control_flow_invalid_ask(),
        ValidationError::InvalidFinish => map_control_flow_invalid_finish(),
        ValidationError::InvalidRetry => map_control_flow_invalid_retry(),
        ValidationError::InvalidOnError => map_control_flow_invalid_on_error(),
        ValidationError::SecretResultLeak => map_type_taint_secret_result_leak(),
        ValidationError::TypeMismatch { expected, found } => {
            map_type_taint_type_mismatch(expected, found)
        }
        ValidationError::PayloadTooLarge => map_type_taint_payload_too_large(),
        ValidationError::LimitRequired { resource } => map_type_taint_limit_required(resource),
        ValidationError::LimitExceeded { resource } => map_type_taint_limit_exceeded(resource),
        ValidationError::UnsupportedTrigger { trigger } => {
            map_type_taint_unsupported_trigger(trigger)
        }
        ValidationError::HttpTriggerOutOfCore => map_type_taint_http_trigger_out_of_core(),
        ValidationError::ExpressionStackExceeded { declared, limit } => {
            map_type_taint_expression_stack_exceeded(*declared, *limit)
        }
        ValidationError::ExpressionStackMismatch {
            expr_index,
            declared,
            computed,
        } => map_type_taint_expression_stack_mismatch(*expr_index, *declared, *computed),
        ValidationError::AccessorSlotOutOfRange {
            accessor_index,
            slot,
            slot_count,
        } => map_type_taint_accessor_slot_out_of_range(*accessor_index, *slot, *slot_count),
        ValidationError::AccessorPathInvalid {
            accessor_index,
            segment_index,
        } => map_type_taint_accessor_path_invalid(*accessor_index, *segment_index),
        ValidationError::AccessorPathTooDeep {
            accessor_index,
            depth,
            max,
        } => map_type_taint_accessor_path_too_deep(*accessor_index, *depth, *max),
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index,
            segment_index,
            symbol,
            symbols_count,
        } => map_type_taint_accessor_symbol_out_of_bounds(
            *accessor_index,
            *segment_index,
            *symbol,
            *symbols_count,
        ),
        ValidationError::SlotReferenceOutOfRange {
            slot,
            slot_count,
            context,
        } => map_type_taint_slot_reference_out_of_range(*slot, *slot_count, context),
        ValidationError::LoopBodyStepOutOfRange {
            step,
            node_count,
            source_node,
            label,
        } => map_gate_loop_body_step_out_of_range(*step, *node_count, *source_node, label),
        ValidationError::SlotDependencyCycle { slot, chain } => {
            map_type_taint_slot_dependency_cycle(*slot, chain)
        }
        ValidationError::NodeKindConstraintViolation { node_index, detail } => {
            map_gate_node_kind_constraint(*node_index, detail)
        }
        ValidationError::SlotTypeInconsistency { slot } => {
            map_type_taint_slot_type_inconsistency(*slot)
        }
        ValidationError::NonDeterministicPath { from_node, to_node } => {
            map_type_taint_non_deterministic_path(*from_node, *to_node)
        }
        ValidationError::ActionContractMissing {
            action_id,
            node_index,
        } => map_contract_action_missing(*action_id, *node_index),
        ValidationError::ActionContractOrphan { action_id } => {
            map_contract_action_orphan(*action_id)
        }
        ValidationError::CapabilityNameEmpty {
            action_id,
            capability_index,
        } => map_contract_capability_name_empty(*action_id, *capability_index),
        ValidationError::CapabilityNameTooLong {
            action_id,
            capability_index,
            len,
            max,
        } => map_contract_capability_name_too_long(*action_id, *capability_index, *len, *max),
        ValidationError::CapabilityNameInvalid {
            action_id,
            capability_index,
            name,
        } => map_contract_capability_name_invalid(*action_id, *capability_index, name),
        ValidationError::CapabilityActionMismatch {
            contract_action_id,
            capability_action_id,
            capability_index,
        } => map_contract_capability_action_mismatch(
            *contract_action_id,
            *capability_action_id,
            *capability_index,
        ),
        ValidationError::CapabilityDuplicate {
            action_id,
            first_index,
            duplicate_index,
            name,
        } => map_contract_capability_duplicate(*action_id, *first_index, *duplicate_index, name),
        ValidationError::MissingSchemaVersion => map_contract_missing_schema_version(),
        ValidationError::CueVetFailed { file } => map_contract_cue_vet_failed(file),
        ValidationError::VersionMonotonicityBreach {
            file,
            expected,
            actual,
        } => map_contract_version_monotonicity(file, expected, actual),
    }
}

// =========================================================================
// E01xx — Schema validation errors
// =========================================================================

fn map_schema_duplicate_key() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_DUPLICATE_KEY),
        "duplicate key".into(),
    )
}

fn map_schema_forbidden_yaml() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_FORBIDDEN_YAML_FEATURE),
        "forbidden YAML feature".into(),
    )
}

fn map_schema_unknown_top_level() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_UNKNOWN_TOP_LEVEL_FIELD),
        "unknown top-level field".into(),
    )
}

fn map_schema_unknown_step_field() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_UNKNOWN_STEP_FIELD),
        "unknown step field".into(),
    )
}

fn map_schema_missing_required_field(field: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_MISSING_REQUIRED_FIELD),
        format!("missing required field: {field}"),
    )
}

fn map_schema_invalid_version(version: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_VERSION),
        format!("invalid version: {version}"),
    )
}

fn map_schema_invalid_id(id: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_ID),
        format!("invalid ID: {id}"),
    )
}

fn map_schema_reserved_id(id: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_RESERVED_ID),
        format!("reserved ID: {id}"),
    )
}

fn map_schema_duplicate_id(id: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_DUPLICATE_ID),
        format!("duplicate ID: {id}"),
    )
}

fn map_schema_multiple_step_primitives() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_MULTIPLE_STEP_PRIMITIVES),
        "multiple step primitives".into(),
    )
}

fn map_schema_missing_step_primitive() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_MISSING_STEP_PRIMITIVE),
        "missing step primitive".into(),
    )
}

// =========================================================================
// E02xx — Reference validation errors
// =========================================================================

fn map_reference_unknown(reference: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_UNKNOWN_REFERENCE),
        format!("unknown reference: {reference}"),
    )
}

fn map_reference_future(reference: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_FUTURE_REFERENCE),
        format!("future reference: {reference}"),
    )
}

fn map_reference_secret_not_declared(secret: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_SECRET_NOT_DECLARED),
        format!("secret not declared: {secret}"),
    )
}

fn map_reference_direct_runtime() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_DIRECT_RUNTIME_REFERENCE),
        "direct runtime reference".into(),
    )
}

fn map_reference_scope_guard(reference: &str, required_scope: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_SCOPE_GUARD_VIOLATION),
        format!("scope guard violation: {reference} requires {required_scope} scope"),
    )
}

fn map_reference_direct_loop(variable: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_DIRECT_LOOP_REFERENCE),
        format!("loop variables must use the `$loop.<var>` prefix (found `${variable}`)"),
    )
}

fn map_reference_direct_step(step: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_DIRECT_STEP_REFERENCE),
        format!("step reference `{step}` must use the `$steps.X` prefix"),
    )
}

fn map_reference_step_skipped(step: usize, reference: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_STEP_SKIPPED_REFERENCE),
        format!(
            "step {} skipped due to unresolved reference `{reference}`",
            step
        ),
    )
}

fn map_reference_result_missing(
    step: &vb_core::ids::StepIdx,
    missing_output: vb_core::ids::SymbolId,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_RESULT_REFERENCE_MISSING),
        format!(
            "step {step} does not produce an output; cannot reference field symbol {missing_output:?}"
        ),
    )
}

fn map_reference_unsupported_step_field(step: &str, field: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_UNSUPPORTED_STEP_FIELD),
        format!(
            "step `{step}` does not expose a `{field}` field; allowed fields are `output` and `result`"
        ),
    )
}

// =========================================================================
// E03xx — Control-flow errors
// =========================================================================

fn map_control_flow_invalid_then() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_THEN_TARGET),
        "invalid then target".into(),
    )
}

fn map_control_flow_cycle() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_CONTROL_FLOW_CYCLE),
        "control-flow cycle".into(),
    )
}

fn map_control_flow_unreachable(step: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_UNREACHABLE_STEP),
        format!("unreachable step: {step}"),
    )
}

fn map_control_flow_invalid_choose() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_CHOOSE),
        "invalid choose".into(),
    )
}

fn map_control_flow_invalid_for_each() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_FOR_EACH),
        "invalid for_each".into(),
    )
}

fn map_control_flow_invalid_together() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_TOGETHER),
        "invalid together".into(),
    )
}

fn map_control_flow_invalid_collect() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_COLLECT),
        "invalid collect".into(),
    )
}

fn map_control_flow_invalid_reduce() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_REDUCE),
        "invalid reduce".into(),
    )
}

fn map_control_flow_invalid_repeat() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_REPEAT),
        "invalid repeat".into(),
    )
}

fn map_control_flow_invalid_wait() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_WAIT),
        "invalid wait".into(),
    )
}

fn map_control_flow_invalid_ask() -> (DiagnosticCode, String) {
    (DiagnosticCode::new(CODE_INVALID_ASK), "invalid ask".into())
}

fn map_control_flow_invalid_finish() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_FINISH),
        "invalid finish".into(),
    )
}

fn map_control_flow_invalid_retry() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_RETRY),
        "invalid retry".into(),
    )
}

fn map_control_flow_invalid_on_error() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_INVALID_ON_ERROR),
        "invalid on_error".into(),
    )
}

// =========================================================================
// E04xx — Type / taint / resource errors
// =========================================================================

fn map_type_taint_secret_result_leak() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_SECRET_RESULT_LEAK),
        "secret result leak".into(),
    )
}

fn map_type_taint_type_mismatch(expected: &str, found: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_TYPE_MISMATCH),
        format!("type mismatch: expected {expected}, found {found}"),
    )
}

fn map_type_taint_payload_too_large() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_PAYLOAD_TOO_LARGE),
        "payload too large".into(),
    )
}

fn map_type_taint_limit_required(resource: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_LIMIT_REQUIRED),
        format!("limit required: {resource}"),
    )
}

fn map_type_taint_limit_exceeded(resource: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_LIMIT_EXCEEDED),
        format!("limit exceeded: {resource}"),
    )
}

fn map_type_taint_unsupported_trigger(trigger: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_UNSUPPORTED_TRIGGER),
        format!("unsupported trigger: {trigger}"),
    )
}

fn map_type_taint_http_trigger_out_of_core() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_HTTP_TRIGGER_OUT_OF_CORE),
        "HTTP trigger out of core".into(),
    )
}

fn map_type_taint_expression_stack_exceeded(
    declared: usize,
    limit: usize,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_EXPRESSION_STACK_EXCEEDED),
        format!("expression stack exceeded: declared {declared}, limit {limit}"),
    )
}

fn map_type_taint_expression_stack_mismatch(
    expr_index: usize,
    declared: usize,
    computed: usize,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_EXPRESSION_STACK_MISMATCH),
        format!(
            "expression stack mismatch: expr {expr_index}, declared {declared}, computed {computed}"
        ),
    )
}

fn map_type_taint_accessor_slot_out_of_range(
    accessor_index: usize,
    slot: usize,
    slot_count: usize,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_ACCESSOR_SLOT_OUT_OF_RANGE),
        format!(
            "accessor slot out of range: accessor {accessor_index}, slot {slot}, slot_count {slot_count}"
        ),
    )
}

fn map_type_taint_accessor_path_invalid(
    accessor_index: usize,
    segment_index: usize,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_ACCESSOR_PATH_INVALID),
        format!("accessor path invalid: accessor {accessor_index}, segment {segment_index}"),
    )
}

fn map_type_taint_accessor_path_too_deep(
    accessor_index: usize,
    depth: usize,
    max: usize,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_ACCESSOR_PATH_TOO_DEEP),
        format!("accessor path too deep: accessor {accessor_index}, depth {depth}, max {max}"),
    )
}

fn map_type_taint_accessor_symbol_out_of_bounds(
    accessor_index: usize,
    segment_index: usize,
    symbol: u32,
    symbols_count: u32,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_ACCESSOR_SYMBOL_OUT_OF_BOUNDS),
        format!(
            "accessor symbol out of bounds: accessor {accessor_index}, segment {segment_index}, symbol {symbol}, symbols_count {symbols_count}"
        ),
    )
}

fn map_type_taint_slot_reference_out_of_range(
    slot: usize,
    slot_count: usize,
    context: &str,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_SLOT_REFERENCE_OUT_OF_RANGE),
        format!(
            "slot reference out of range: slot {slot}, slot_count {slot_count}, context {context}"
        ),
    )
}

fn map_type_taint_slot_dependency_cycle(slot: usize, chain: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_SLOT_DEPENDENCY_CYCLE),
        format!("slot dependency cycle: slot {slot}, chain {chain}"),
    )
}

fn map_type_taint_slot_type_inconsistency(slot: usize) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_SLOT_TYPE_INCONSISTENCY),
        format!("slot type inconsistency: slot {slot} has incompatible writers"),
    )
}

fn map_type_taint_non_deterministic_path(
    from_node: usize,
    to_node: usize,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_NON_DETERMINISTIC_PATH),
        format!(
            "non-deterministic path: from node {from_node} to node {to_node} contains no suspension point"
        ),
    )
}

// =========================================================================
// E05xx — Gate verifier errors
// =========================================================================

fn map_gate_loop_body_step_out_of_range(
    step: usize,
    node_count: usize,
    source_node: usize,
    label: &str,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_LOOP_BODY_STEP_OUT_OF_RANGE),
        format!(
            "loop body step out of range: step {step}, node_count {node_count}, source_node {source_node}, label {label}"
        ),
    )
}

fn map_gate_node_kind_constraint(node_index: usize, detail: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_NODE_KIND_CONSTRAINT_VIOLATION),
        format!("node kind constraint violation: node {node_index}, {detail}"),
    )
}

// =========================================================================
// E06xx — Contract-discovery errors
// =========================================================================

fn map_contract_action_missing(action_id: usize, node_index: usize) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_ACTION_CONTRACT_MISSING),
        format!(
            "action contract missing: action_id {action_id} referenced by Do node {node_index}"
        ),
    )
}

fn map_contract_action_orphan(action_id: usize) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_ACTION_CONTRACT_ORPHAN),
        format!("action contract orphan: action_id {action_id} has no corresponding Do node"),
    )
}

fn map_contract_capability_name_empty(
    action_id: usize,
    capability_index: usize,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_CAPABILITY_NAME_EMPTY),
        format!(
            "capability name is empty for action {action_id} at required_capabilities[{capability_index}]"
        ),
    )
}

fn map_contract_capability_name_too_long(
    action_id: usize,
    capability_index: usize,
    len: usize,
    max: usize,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_CAPABILITY_NAME_TOO_LONG),
        format!(
            "capability name too long for action {action_id} at required_capabilities[{capability_index}]: {len} > {max}"
        ),
    )
}

fn map_contract_capability_name_invalid(
    action_id: usize,
    capability_index: usize,
    name: &str,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_CAPABILITY_NAME_INVALID),
        format!(
            "invalid capability name for action {action_id} at required_capabilities[{capability_index}]: {name}"
        ),
    )
}

fn map_contract_capability_action_mismatch(
    contract_action_id: usize,
    capability_action_id: usize,
    capability_index: usize,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_CAPABILITY_ACTION_MISMATCH),
        format!(
            "capability action {capability_action_id} does not match contract action {contract_action_id} at required_capabilities[{capability_index}]"
        ),
    )
}

fn map_contract_capability_duplicate(
    action_id: usize,
    first_index: usize,
    duplicate_index: usize,
    name: &str,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_CAPABILITY_DUPLICATE),
        format!(
            "duplicate capability requirement for action {action_id}: {name} at required_capabilities[{first_index}] and required_capabilities[{duplicate_index}]"
        ),
    )
}

fn map_contract_missing_schema_version() -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_MISSING_SCHEMA_VERSION),
        "missing schema_version field".into(),
    )
}

fn map_contract_cue_vet_failed(file: &str) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_CUE_VET_FAILED),
        format!("cue vet failed for {file}"),
    )
}

fn map_contract_version_monotonicity(
    file: &str,
    expected: &str,
    actual: &str,
) -> (DiagnosticCode, String) {
    (
        DiagnosticCode::new(CODE_VERSION_MONOTONICITY_BREACH),
        format!("version monotonicity breach: {file} expected {expected} got {actual}"),
    )
}
