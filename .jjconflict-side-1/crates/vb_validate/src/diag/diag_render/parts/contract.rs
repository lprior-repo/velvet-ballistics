#![forbid(unsafe_code)]
//! Contract diagnostic code and message mapping.

use crate::ValidationError;
use crate::diag::diag_codes::*;
use vb_core::diagnostic::DiagnosticCode;

pub(super) fn contract_parts(error: &ValidationError) -> (DiagnosticCode, String) {
    match error {
        ValidationError::LoopBodyStepOutOfRange {
            step,
            node_count,
            source_node,
            label,
        } => code_msg(
            CODE_LOOP_BODY_STEP_OUT_OF_RANGE,
            format!(
                "loop body step out of range: step {step}, node_count {node_count}, source_node {source_node}, label {label}"
            ),
        ),
        ValidationError::SlotDependencyCycle { slot, chain } => code_msg(
            CODE_SLOT_DEPENDENCY_CYCLE,
            format!("slot dependency cycle: slot {slot}, chain {chain}"),
        ),
        ValidationError::NodeKindConstraintViolation { node_index, detail } => code_msg(
            CODE_NODE_KIND_CONSTRAINT_VIOLATION,
            format!("node kind constraint violation: node {node_index}, {detail}"),
        ),
        ValidationError::ActionContractMissing {
            action_id,
            node_index,
        } => action_contract_missing(*action_id, *node_index),
        ValidationError::ActionContractOrphan { action_id } => code_msg(
            CODE_ACTION_CONTRACT_ORPHAN,
            format!("action contract orphan: action_id {action_id} has no corresponding Do node"),
        ),
        ValidationError::CapabilityNameEmpty {
            action_id,
            capability_index,
        } => capability_name_empty(*action_id, *capability_index),
        ValidationError::CapabilityNameTooLong {
            action_id,
            capability_index,
            len,
            max,
        } => capability_name_too_long(*action_id, *capability_index, *len, *max),
        ValidationError::CapabilityNameInvalid {
            action_id,
            capability_index,
            name,
        } => capability_name_invalid(*action_id, *capability_index, name),
        ValidationError::CapabilityActionMismatch {
            contract_action_id,
            capability_action_id,
            capability_index,
        } => capability_action_mismatch(
            *contract_action_id,
            *capability_action_id,
            *capability_index,
        ),
        ValidationError::CapabilityDuplicate {
            action_id,
            first_index,
            duplicate_index,
            name,
        } => capability_duplicate(*action_id, *first_index, *duplicate_index, name),
        ValidationError::SlotTypeInconsistency { slot } => code_msg(
            CODE_SLOT_TYPE_INCONSISTENCY,
            format!("slot type inconsistency: slot {slot} has incompatible writers"),
        ),
        ValidationError::NonDeterministicPath { from_node, to_node } => code_msg(
            CODE_NON_DETERMINISTIC_PATH,
            format!(
                "non-deterministic path: from node {from_node} to node {to_node} contains no suspension point"
            ),
        ),
        ValidationError::MissingSchemaVersion => {
            code_msg(CODE_MISSING_SCHEMA_VERSION, "missing schema_version field")
        }
        ValidationError::CueVetFailed { file } => {
            code_msg(CODE_CUE_VET_FAILED, format!("cue vet failed for {file}"))
        }
        ValidationError::VersionMonotonicityBreach {
            file,
            expected,
            actual,
        } => code_msg(
            CODE_VERSION_MONOTONICITY_BREACH,
            format!("version monotonicity breach: {file} expected {expected} got {actual}"),
        ),
        other => code_msg(
            CODE_MISSING_REQUIRED_FIELD,
            format!("unmapped validation error: {other}"),
        ),
    }
}

fn code_msg(code: u16, message: impl Into<String>) -> (DiagnosticCode, String) {
    (DiagnosticCode::new(code), message.into())
}

fn action_contract_missing(action_id: usize, node_index: usize) -> (DiagnosticCode, String) {
    code_msg(
        CODE_ACTION_CONTRACT_MISSING,
        format!(
            "action contract missing: action_id {action_id} referenced by Do node {node_index}"
        ),
    )
}

fn capability_name_empty(action_id: usize, capability_index: usize) -> (DiagnosticCode, String) {
    code_msg(
        CODE_CAPABILITY_NAME_EMPTY,
        format!(
            "capability name is empty for action {action_id} at required_capabilities[{capability_index}]"
        ),
    )
}

fn capability_name_too_long(
    action_id: usize,
    capability_index: usize,
    len: usize,
    max: usize,
) -> (DiagnosticCode, String) {
    code_msg(
        CODE_CAPABILITY_NAME_TOO_LONG,
        format!(
            "capability name too long for action {action_id} at required_capabilities[{capability_index}]: {len} > {max}"
        ),
    )
}

fn capability_name_invalid(
    action_id: usize,
    capability_index: usize,
    name: &str,
) -> (DiagnosticCode, String) {
    code_msg(
        CODE_CAPABILITY_NAME_INVALID,
        format!(
            "invalid capability name for action {action_id} at required_capabilities[{capability_index}]: {name}"
        ),
    )
}

fn capability_action_mismatch(
    contract_action_id: usize,
    capability_action_id: usize,
    capability_index: usize,
) -> (DiagnosticCode, String) {
    code_msg(
        CODE_CAPABILITY_ACTION_MISMATCH,
        format!(
            "capability action {capability_action_id} does not match contract action {contract_action_id} at required_capabilities[{capability_index}]"
        ),
    )
}

fn capability_duplicate(
    action_id: usize,
    first_index: usize,
    duplicate_index: usize,
    name: &str,
) -> (DiagnosticCode, String) {
    code_msg(
        CODE_CAPABILITY_DUPLICATE,
        format!(
            "duplicate capability requirement for action {action_id}: {name} at required_capabilities[{first_index}] and required_capabilities[{duplicate_index}]"
        ),
    )
}
